// #![feature(plugin)]
// #![plugin(rocket_codegen)]

// extern crate rocket;
extern crate rppal;
extern crate rand;

// use rocket::State;
// use std::sync::Mutex;

use rand::Rng;

use rppal::gpio::{GPIO, Mode, Level};
use rppal::system::DeviceInfo;

use std::thread;
use std::time::Duration;

// The GPIO module uses BCM pin numbering. BCM 18 equates to physical pin 12.
const pins: &[u8] = &[15, 18, 23, 24];

// #[get("/high")]
// fn high(gpio: State<Mutex<GPIO>>) -> &'static str {
//     let gpio = gpio.lock().unwrap();
//     gpio.write(GPIO_PIN, Level::High);
//     "High"
// }
//
// #[get("/low")]
// fn low(gpio: State<Mutex<GPIO>>) -> &'static str {
//     let gpio = gpio.lock().unwrap();
//     gpio.write(GPIO_PIN, Level::Low);
//     "Low"
// }

fn main() {
    let device_info = DeviceInfo::new().unwrap();
    println!("Model: {} (SoC: {})", device_info.model(), device_info.soc());

    let mut gpio = GPIO::new().unwrap();
    for pin in pins {
        gpio.set_mode(*pin, Mode::Output);
    }
    let mut rng = rand::thread_rng();

    loop {
        let num = rng.gen::<u8>() & (16-1);
        println!("\nnum: {:b}", num);
        for i in 0..4 {
            print!("writing pin {}", i);
            match 1 & (num>>i) {
                0 => gpio.write(pins[i], Level::High),
                1 => gpio.write(pins[i], Level::Low),
                _ => panic!("Yo!"),
            }
            println!(" ... done");
        }
        thread::sleep(Duration::from_millis(800));
    }

    // rocket::ignite().manage(Mutex::new(gpio)).mount("/", routes![low, high]).launch();
}
