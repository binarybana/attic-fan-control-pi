#[macro_use]
extern crate log;
extern crate env_logger;

extern crate simple_server;

use simple_server::{Server, Method, StatusCode};

extern crate rppal;
extern crate rand;
extern crate reqwest;

#[macro_use]
extern crate serde_derive;

extern crate serde;
extern crate serde_json;

use rand::Rng;

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

fn main() {

    let token = match std::env::var("ATTIC_ACCESS_TOKEN") {
        Ok(val) => val,
        Err(e) => panic!("Env var ATTIC_ACCESS_TOKEN (with particle access token) not set"),
    };

    let device_id = match std::env::var("ATTIC_DEVICE_ID") {
        Ok(val) => val,
        Err(e) => panic!("Env var ATTIC_DEVICE_ID (with particle device id) not set"),
    };
    
    let url = format!("https://api.particle.io/v1/devices/{}/temp?access_token={}", device_id, token);
    let mut resp = reqwest::get(&url).unwrap();
    let tempRecord: TempRecord = resp.json().unwrap();
    let temp = tempRecord.result;
    println!("{}", temp);

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
