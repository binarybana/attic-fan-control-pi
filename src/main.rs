// #[macro_use]
// extern crate log;
// extern crate env_logger;

extern crate reqwest;

#[macro_use]
extern crate rouille;

#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate serde_json;

extern crate chrono;
use chrono::prelude::*;

extern crate rppal;
use rppal::gpio::{Gpio, Mode, Level};
use rppal::system::DeviceInfo;

use std::sync::{Mutex, Arc};

// The Gpio module uses BCM pin numbering. BCM 18 equates to physical pin 12.
// Pi1:
// const PINS: &[u8] = &[15, 18, 23, 24];
// Pi3:
// 12: gameroom
// 16: bedroom
// 20: kids room
// 21: kids room
const PINS: &[u8] = &[12, 16, 20, 21];

#[derive(Deserialize)]
struct TempRecord {
    result: f64,
}

#[derive(Debug, Clone)]
struct ThermostatState {
    current_temp: f64,
    set_point: f64,
    buffer: f64,
    fan_on: bool,
    schedule_on: bool,
    smooth_alpha: f64,
    power: bool,
}

fn get_temp() -> Result<f64, reqwest::Error> {
    let token = match std::env::var("ATTIC_ACCESS_TOKEN") {
        Ok(val) => val,
        Err(_) => panic!("Env var ATTIC_ACCESS_TOKEN (with particle access token) not set"),
    };

    let device_id = match std::env::var("ATTIC_DEVICE_ID") {
        Ok(val) => val,
        Err(_) => panic!("Env var ATTIC_DEVICE_ID (with particle device id) not set"),
    };
    
    let client = reqwest::ClientBuilder::new().build()?;
    let url = format!("https://api.particle.io/v1/devices/{}/temp?access_token={}", device_id, token);
    let mut resp = client.get(&url).send()?;
    let temp_record: TempRecord = resp.json()?;
    let temp = temp_record.result;
    Ok(temp)
}

fn setup() -> ThermostatState {
    let alpha = 0.9;
    let smoothed_temp = get_temp().unwrap_or(17.0);
    let set_point = match std::env::var("ATTIC_SET_POINT") {
        Ok(val) => val.parse().unwrap_or(17.7),
        Err(_) => {
            println!("Env var ATTIC_SET_POINT not set");
            17.7
        },
    };
    let buffer = match std::env::var("ATTIC_BUFFER") {
        Ok(val) => val.parse().unwrap_or(1.0),
        Err(_) => {
            println!("Env var ATTIC_BUFFER not set");
            1.0
        },
    };
    println!("Set point: {}, buffer: {}", set_point, buffer);
    let device_info = DeviceInfo::new().unwrap();
    println!("Model: {} (SoC: {})", device_info.model(), device_info.soc());

    let mut gpio = Gpio::new().unwrap();
    for pin in PINS {
        gpio.set_mode(*pin, Mode::Output);
        // Make sure everything is off
        gpio.write(*pin, Level::High);
    }
    ThermostatState {
        current_temp: smoothed_temp,
        set_point: set_point,
        buffer: buffer,
        fan_on: false,
        schedule_on: true,
        smooth_alpha: alpha,
        power: true,
    }
}

/// Always update temperature
fn temp_updater(data: Arc<Mutex<ThermostatState>>) {
    use std::{thread, time};
    let one_minute = time::Duration::new(60, 0);
    loop {
        { // mutex lock scope
            let mut tstate = data.lock().unwrap();
            let smoothed_temp = match get_temp() {
                Ok(new_temp) => (1.0 - tstate.smooth_alpha) * tstate.current_temp + tstate.smooth_alpha * new_temp,
                Err(_) => {
                    println!("Failed to get temp");
                    continue;
                },
            };
            (*tstate).current_temp = smoothed_temp;
            println!("smoothed temp: {}", smoothed_temp);
        }
        thread::sleep(one_minute);
    }
}

/// Control the fan
fn thermostat(data: Arc<Mutex<ThermostatState>>) {
    use std::{thread, time};
    let one_minute = time::Duration::new(60, 0);
    let gpio = Gpio::new().unwrap();
    loop {
        let now = Local::now();
        let on_time = now.date().and_hms(22, 0, 0);
        if now > now.date().and_hms(5, 30, 0) && now < on_time {
            println!("Shutting off fan");
            gpio.write(16, Level::High);
            let duration = on_time.signed_duration_since(now);
            println!("Sleeping for {:?} until {:?}", duration, on_time);
            { (*data.lock().unwrap()).schedule_on = false; }
            thread::sleep(duration.to_std().unwrap());
        } else {
            let mut tstate = data.lock().unwrap();
            (*tstate).schedule_on = true;
            let smoothed_temp = (*tstate).current_temp;
            if smoothed_temp < (tstate.set_point-tstate.buffer) && tstate.fan_on {
                // turn off
                println!("Turning off fan");
                (*tstate).fan_on = false;
                gpio.write(16, Level::High);
            } else if smoothed_temp > (tstate.set_point+tstate.buffer) && !tstate.fan_on {
                // turn on
                println!("Turning fan on");
                (*tstate).fan_on = true;
                gpio.write(16, Level::Low);
            }
        }
        thread::sleep(one_minute);
    }
}

fn main() {

    let data = Arc::new(Mutex::new(setup()));
    let data2 = data.clone();
    let data3 = data.clone();
    let data4 = data.clone();
    std::thread::spawn( || { thermostat(data2) });
    std::thread::spawn( || { temp_updater(data4) });

    rouille::start_server("0.0.0.0:8000", move |request| {
        router!(request,
            (GET) (/) => {
                let datainside: &ThermostatState = &(*data3.lock().unwrap());
                rouille::Response::text(format!("Thermostat state:\n{:?}", datainside))
            },

            (GET) (/on) => {
                let mut datainside = data3.lock().unwrap();
                (*datainside).power = true;
                println!("Tstat enabled manually");
                rouille::Response::text("Turned on")
            },

            (GET) (/off) => {
                let mut datainside = data3.lock().unwrap();
                (*datainside).power = false;
                println!("Tstat disabled manually");
                rouille::Response::text("Turned off")
            },

            (GET) (/set_point/{set_point: f64}) => {
                let mut datainside = data3.lock().unwrap();
                (*datainside).set_point = set_point;
                println!("Set point set to {}", set_point);
                rouille::Response::text(format!("Set point set to {}", set_point))
            },

            (GET) (/alpha/{alpha: f64}) => {
                let mut datainside = data3.lock().unwrap();
                (*datainside).smooth_alpha = alpha;
                println!("Alpha set to {}", alpha);
                rouille::Response::text(format!("Alpha set to {}", alpha))
            },

            (GET) (/buffer/{buffer: f64}) => {
                let mut datainside = data3.lock().unwrap();
                (*datainside).buffer = buffer;
                println!("Buffer set to {}", buffer);
                rouille::Response::text(format!("Buffer set to {}", buffer))
            },

            _ => rouille::Response::empty_404()
        )
    });
}
