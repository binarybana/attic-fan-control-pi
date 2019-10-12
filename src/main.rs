use chrono::prelude::*;
use serde_derive::{Deserialize, Serialize};

use rppal::gpio::Gpio;

use std::sync::{Arc, Mutex};
use std::{thread, time};

#[macro_use]
extern crate rouille;

// The Gpio module uses BCM pin numbering. BCM 18 equates to physical pin 12.
// Pi1:
// const PINS: &[u8] = &[15, 18, 23, 24];
// Pi3:
// 12: gameroom
// 16: bedroom
// 20: kids room
// 21: kids room
// const PINS: &[u8] = &[12, 16, 20, 21];

#[derive(Deserialize)]
struct TempRecord {
    result: f64,
}

#[derive(Deserialize)]
struct WeatherRecord {
    main: WeatherInnerRecord,
}

#[derive(Deserialize)]
struct WeatherInnerRecord {
    temp: f64,
    humidity: f64,
}

#[derive(Serialize, Debug, Clone)]
struct ThermostatState {
    // config:
    set_point: f64,
    buffer: f64,
    smooth_alpha: f64,
    on_time: u32,
    off_time: u32,
    outside_max_humidity: f64,
    // state:
    current_temp: Option<f64>,
    fan_on: bool,
    schedule_on: bool,
    too_hot: bool,
    manual_on: bool,
    outside_temp: Option<f64>,
    outside_humidity: Option<f64>,
    outside_right: bool,
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
    let url = format!(
        "https://api.particle.io/v1/devices/{}/temp?access_token={}",
        device_id, token
    );
    let mut resp = client.get(&url).send()?;
    let temp_record: TempRecord = resp.json()?;
    let temp = temp_record.result;
    Ok(temp)
}

fn setup() -> ThermostatState {
    let alpha = 0.9;
    let smoothed_temp = get_temp()
        .map_err(|e| {
            log::warn!("Problem getting temp: {:?}", e);
            e
        })
        .ok();
    let set_point = match std::env::var("ATTIC_SET_POINT") {
        Ok(val) => val.parse().unwrap_or(17.7),
        Err(_) => {
            println!("Env var ATTIC_SET_POINT not set");
            17.7
        }
    };
    let buffer = match std::env::var("ATTIC_BUFFER") {
        Ok(val) => val.parse().unwrap_or(1.0),
        Err(_) => {
            println!("Env var ATTIC_BUFFER not set");
            1.0
        }
    };
    let outside_max_humidity = match std::env::var("OUTSIDE_MAX_HUMIDITY") {
        Ok(val) => val.parse().unwrap_or(85.0),
        Err(_) => {
            println!("Env var OUTSIDE_MAX_HUMIDITY not set");
            1.0
        }
    };
    println!("Set point: {}, buffer: {}", set_point, buffer);
    ThermostatState {
        current_temp: smoothed_temp,
        set_point: set_point,
        buffer: buffer,
        fan_on: false,
        smooth_alpha: alpha,
        schedule_on: false,
        too_hot: false,
        on_time: 2200,
        off_time: 530,
        manual_on: false,
        outside_temp: None,
        outside_humidity: None,
        outside_right: true,
        outside_max_humidity: outside_max_humidity,
    }
}

/// Always update temperature
fn temp_updater(data: Arc<Mutex<ThermostatState>>) {
    let one_minute = time::Duration::new(60, 0);
    loop {
        // hoist potentially long duration call out of mutex scope
        let temp_data = get_temp();
        {
            // mutex lock scope
            let mut tstate = data.lock().unwrap();
            let smoothed_temp = match temp_data {
                Ok(new_temp) => Some(
                    (1.0 - tstate.smooth_alpha) * tstate.current_temp.unwrap_or(new_temp)
                        + tstate.smooth_alpha * new_temp,
                ),
                Err(e) => {
                    log::warn!("Failed to get temp: {:?}", e);
                    None
                }
            };
            (*tstate).current_temp = smoothed_temp;
            // Since this loop runs faster, we update `outside_right` here instead of
            // `weather_updater`
            (*tstate).outside_right = match (
                tstate.outside_temp,
                tstate.current_temp,
                tstate.outside_humidity,
            ) {
                (None, _, _) | (_, None, _) | (_, _, None) => false,
                (Some(outside), Some(inside), Some(humidity)) => {
                    (outside < (inside - tstate.buffer))
                        && ((humidity < tstate.outside_max_humidity) || ((inside - outside) > 1.0))
                }
            };
            log::info!("smoothed temp: {:?}", smoothed_temp);
        }
        thread::sleep(one_minute);
    }
}

/// Grab latest weather data
fn weather_updater(data: Arc<Mutex<ThermostatState>>) {
    let ten_minutes = time::Duration::new(60 * 10, 0);
    let apikey = match std::env::var("OPENWEATHERMAP_KEY") {
        Ok(val) => val,
        Err(_) => panic!("Env var OPENWEATHERMAP_KEY not set"),
    };
    let client = reqwest::ClientBuilder::new().build().unwrap(); // TODO fix unwrap
    loop {
        // hoist potentially long duration call out of mutex scope
        let url = format!(
            "http://api.openweathermap.org/data/2.5/weather?id=5384690&APPID={}",
            apikey
        );
        let mut resp = client.get(&url).send().unwrap(); // TODO fix unwrap
        let weather_record: Option<WeatherRecord> = resp
            .json()
            .map_err(|e| log::warn!("Couldn't decode weather json: {:?}", e))
            .ok();
        let (temp, humidity) = match weather_record {
            None => (None, None),
            Some(r) => (Some(r.main.temp - 273.15), Some(r.main.humidity)),
        };
        log::info!("Outer temp: {:?}, Humidity: {:?}", temp, humidity);
        {
            // mutex lock scope
            let mut tstate = data.lock().unwrap();
            (*tstate).outside_temp = temp;
            (*tstate).outside_humidity = humidity;
        }
        thread::sleep(ten_minutes);
    }
}

/// Control the GPIO
/// IE: implement state controller
fn overall_controller(data: Arc<Mutex<ThermostatState>>) {
    let sleep_time = time::Duration::new(5, 0);
    loop {
        {
            // mutex lock scope
            let mut tstate = data.lock().unwrap();
            if tstate.manual_on || (tstate.too_hot && tstate.outside_right && tstate.schedule_on) {
                tstate.fan_on = true;
            } else {
                tstate.fan_on = false;
            }
        }
        thread::sleep(sleep_time);
    }
}

/// Control the GPIO
/// IE: implement state controller
fn gpio_controller(data: Arc<Mutex<ThermostatState>>) {
    let sleep_time = time::Duration::new(1, 0);
    let gpio = Gpio::new().expect("Couldn't init GPIO");
    let mut pin = gpio.get(16).expect("Couldn't grab pin 16").into_output();
    // Turn it off to start with
    pin.set_high();
    loop {
        {
            // mutex lock scope
            let tstate = data.lock().unwrap();
            // Here we go ahead and "drive" the controller hard for simplicity and robustness
            if tstate.fan_on {
                pin.set_low();
            } else {
                pin.set_high();
            }
        }
        thread::sleep(sleep_time);
    }
}

/// Control the fan
fn temp_controller(data: Arc<Mutex<ThermostatState>>) {
    let one_minute = time::Duration::new(60, 0);
    loop {
        {
            // mutex lock scope
            let mut tstate = data.lock().unwrap();
            if let Some(smoothed_temp) = (*tstate).current_temp {
                if smoothed_temp < (tstate.set_point - tstate.buffer) {
                    log::info!("Cold enough");
                    (*tstate).too_hot = false;
                } else if smoothed_temp > (tstate.set_point + tstate.buffer) {
                    log::info!("Too hot");
                    (*tstate).too_hot = true;
                }
            } else {
                log::warn!("Disabling fan since we don't know current temp");
                (*tstate).too_hot = false;
            }
        }
        thread::sleep(one_minute);
    }
}

/// Time based control of the fan
fn schedule_controller(data: Arc<Mutex<ThermostatState>>) {
    let one_minute = time::Duration::new(60, 0);
    loop {
        {
            // mutex lock scope
            let mut tstate = data.lock().unwrap();
            let now = Local::now();
            let on_time = now
                .date()
                .and_hms((*tstate).on_time / 100, (*tstate).on_time % 100, 0);
            let off_time =
                now.date()
                    .and_hms((*tstate).off_time / 100, (*tstate).off_time % 100, 0);
            let time_till_on = on_time.signed_duration_since(now);
            let time_till_off = off_time.signed_duration_since(now);
            if now < on_time && time_till_on < chrono::Duration::minutes(5) {
                // Turn on thermostat
                (*tstate).schedule_on = true;
            } else if now < off_time && time_till_off < chrono::Duration::minutes(5) {
                // Turn off the thermostat
                (*tstate).schedule_on = false;
            }
        }
        thread::sleep(one_minute);
    }
}

fn main() {
    env_logger::init();
    let data = Arc::new(Mutex::new(setup()));
    let data2 = data.clone();
    let data3 = data.clone();
    let data4 = data.clone();
    let data5 = data.clone();
    let data6 = data.clone();
    let data7 = data.clone();
    let data8 = data.clone();
    std::thread::spawn(|| overall_controller(data2));
    std::thread::spawn(|| gpio_controller(data4));
    std::thread::spawn(|| schedule_controller(data5));
    std::thread::spawn(|| temp_controller(data6));
    std::thread::spawn(|| temp_updater(data7));
    std::thread::spawn(|| weather_updater(data8));

    rouille::start_server("0.0.0.0:8000", move |request| {
        rouille::router!(request,
            (GET) (/) => {
                let datainside: &ThermostatState = &(*data3.lock().unwrap());
                rouille::Response::json(datainside)
            },

            (GET) (/manual_on) => {
                let mut datainside = data3.lock().unwrap();
                (*datainside).manual_on = true;
                println!("Fan manual mode on");
                rouille::Response::text("Turned on")
            },

            (GET) (/manual_off) => {
                let mut datainside = data3.lock().unwrap();
                (*datainside).manual_on = false;
                println!("Fan manual mode off");
                rouille::Response::text("Turned off")
                },

            (GET) (/schedule_on) => {
                let mut datainside = data3.lock().unwrap();
                (*datainside).schedule_on = true;
                println!("Fan schedule manually on");
                rouille::Response::text("Schedule turned on")
            },

            (GET) (/schedule_off) => {
                let mut datainside = data3.lock().unwrap();
                (*datainside).schedule_on = false;
                println!("Fan schedule manually off");
                rouille::Response::text("Schedule turned off")
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

            (GET) (/on_time/{on_time: u32}) => {
                let mut datainside = data3.lock().unwrap();
                (*datainside).on_time = on_time;
                println!("on_time set to {}", on_time);
                rouille::Response::text(format!("on_time set to {}", on_time))
            },

            (GET) (/off_time/{off_time: u32}) => {
                let mut datainside = data3.lock().unwrap();
                (*datainside).off_time = off_time;
                println!("off_time set to {}", off_time);
                rouille::Response::text(format!("off_time set to {}", off_time))
            },

            _ => rouille::Response::empty_404()
        )
    });
}
