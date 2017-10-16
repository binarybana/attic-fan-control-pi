#[macro_use]
extern crate log;
extern crate env_logger;

extern crate simple_server;

use simple_server::{Server, Method, StatusCode};

extern crate rppal;
extern crate rand;

use rand::Rng;

use rppal::gpio::{GPIO, Mode, Level};
use rppal::system::DeviceInfo;

// The GPIO module uses BCM pin numbering. BCM 18 equates to physical pin 12.
const PINS: &[u8] = &[15, 18, 23, 24];

fn main() {
    let device_info = DeviceInfo::new().unwrap();
    println!("Model: {} (SoC: {})", device_info.model(), device_info.soc());

    let mut gpio = GPIO::new().unwrap();
    for pin in PINS {
        gpio.set_mode(*pin, Mode::Output);
    }

    let host = "0.0.0.0";
    let port = "8000";

    let server = Server::new(|request, mut response| {
        info!("Request received. {} {}", request.method(), request.uri());

        match (request.method(), request.uri().path()) {
            (&Method::GET, "/hello") => {
                Ok(response.body("<h1>Hi!</h1><p>Hello Rust!</p>".as_bytes())?)
            }
            (&Method::GET, "/pinchange") => {
                let gpio = GPIO::new().unwrap();
                let mut rng = rand::thread_rng();
                let num = rng.gen::<u8>() & (16-1);
                println!("\nnum: {:b}", num);
                for i in 0..4 {
                    print!("writing pin {}", i);
                    match 1 & (num>>i) {
                        0 => gpio.write(PINS[i], Level::High),
                        1 => gpio.write(PINS[i], Level::Low),
                        _ => panic!("Yo!"),
                    }
                    println!(" ... done");
                }
                Ok(response.body("<h1>Hi!</h1><p>Pins changed!</p>".as_bytes())?)
            }
            (_, _) => {
                response.status(StatusCode::NOT_FOUND);
                Ok(response.body("<h1>404</h1><p>Not found!<p>".as_bytes())?)
            }
        }
    });

    server.listen(host, port);
}
