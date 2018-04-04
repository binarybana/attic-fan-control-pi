#[macro_use]
extern crate log;
extern crate env_logger;

extern crate simple_server;

// use simple_server::{Server, Method, StatusCode};

extern crate rppal;
extern crate rand;
extern crate reqwest;

#[macro_use]
extern crate serde_derive;

extern crate serde;
extern crate serde_json;

extern crate chrono;
use chrono::prelude::*;

use rppal::gpio::{Gpio, Mode, Level};
use rppal::system::DeviceInfo;

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

fn main() {

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
    let mut fan_on = false;

    use std::{thread, time};
    let one_minute = time::Duration::new(60, 0);

    let device_info = DeviceInfo::new().unwrap();
    println!("Model: {} (SoC: {})", device_info.model(), device_info.soc());

    let mut gpio = Gpio::new().unwrap();
    for pin in PINS {
        gpio.set_mode(*pin, Mode::Output);
        // Make sure everything is off
        gpio.write(*pin, Level::High);
    }


    loop {
        let now = Local::now();
        println!("Current time: {}", now);
        let on_time = now.date().and_hms(22, 0, 0);
        if now > now.date().and_hms(5, 30, 0) || now < on_time {
            let duration = on_time.signed_duration_since(now);
            println!("Sleeping for {:?} until {:?}", duration, on_time);
            thread::sleep(duration.to_std().unwrap());
        }
        thread::sleep(one_minute);
        let smoothed_temp = match get_temp() {
            Ok(new_temp) => (1.0 - alpha) * smoothed_temp + alpha * new_temp,
            Err(_) => {
                println!("Failed to get temp");
                continue;
            },
        };
        println!("smoothed temp: {}", smoothed_temp);

        if smoothed_temp < (set_point-buffer) && fan_on {
            // turn off
            println!("Turning off fan");
            fan_on = false;
            gpio.write(16, Level::High);
        } else if smoothed_temp > (set_point+buffer) && !fan_on {
            // turn on
            println!("Turning fan on");
            fan_on = true;
            gpio.write(16, Level::Low);
        }
    }

    // let device_info = DeviceInfo::new().unwrap();
    // println!("Model: {} (SoC: {})", device_info.model(), device_info.soc());
    //
    // let mut gpio = Gpio::new().unwrap();
    // for pin in PINS {
    //     gpio.set_mode(*pin, Mode::Output);
    // }
    //
    // let host = "0.0.0.0";
    // let port = "8000";
    //
    // let server = Server::new(|request, mut response| {
    //     info!("Request received. {} {}", request.method(), request.uri());
    //
    //     match (request.method(), request.uri().path()) {
    //         (&Method::GET, "/hello") => {
    //             Ok(response.body("<h1>Hi!</h1><p>Hello Rust!</p>".as_bytes())?)
    //         }
    //         (&Method::GET, "/12") => {
    //             let gpio = Gpio::new().unwrap();
    //             gpio.write(12, Level::Low);
    //             println!("12 on");
    //             Ok(response.body("<h1>Hi!</h1><p>12 On</p>".as_bytes())?)
    //         }
    //         (&Method::GET, "/16") => {
    //             let gpio = Gpio::new().unwrap();
    //             gpio.write(16, Level::Low);
    //             println!("16 on");
    //             Ok(response.body("<h1>Hi!</h1><p>16 On</p>".as_bytes())?)
    //         }
    //         (&Method::GET, "/20") => {
    //             let gpio = Gpio::new().unwrap();
    //             gpio.write(20, Level::Low);
    //             println!("20 on");
    //             Ok(response.body("<h1>Hi!</h1><p>20 On</p>".as_bytes())?)
    //         }
    //         (&Method::GET, "/21") => {
    //             let gpio = Gpio::new().unwrap();
    //             gpio.write(21, Level::Low);
    //             println!("21 on");
    //             Ok(response.body("<h1>Hi!</h1><p>21 On</p>".as_bytes())?)
    //         }
    //         (&Method::GET, "/pinchange") => {
    //             let gpio = Gpio::new().unwrap();
    //             let mut rng = rand::thread_rng();
    //             let num = rng.gen::<u8>() & (16-1);
    //             println!("\nnum: {:b}", num);
    //             for i in 0..4 {
    //                 print!("writing pin {}", i);
    //                 match 1 & (num>>i) {
    //                     0 => gpio.write(PINS[i], Level::High),
    //                     1 => gpio.write(PINS[i], Level::Low),
    //                     _ => panic!("Yo!"),
    //                 }
    //                 println!(" ... done");
    //             }
    //             Ok(response.body("<h1>Hi!</h1><p>Pins changed!</p>".as_bytes())?)
    //         }
    //         (&Method::GET, "/off") => {
    //             let gpio = Gpio::new().unwrap();
    //             for i in 0..4 {
    //                     gpio.write(PINS[i], Level::High);
    //             }
    //             println!(" ... done");
    //             Ok(response.body("<h1>Hi!</h1><p>Pins off.</p>".as_bytes())?)
    //         }
    //         (_, _) => {
    //             response.status(StatusCode::NOT_FOUND);
    //             Ok(response.body("<h1>404</h1><p>Not found!<p>".as_bytes())?)
    //         }
    //     }
    // });
    //
    // server.listen(host, port);
}
