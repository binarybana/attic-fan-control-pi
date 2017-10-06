#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate rocket;
extern crate rppal;

use rocket::State;

use std::sync::Mutex;

use rppal::gpio::{GPIO, Mode, Level};
use rppal::system::DeviceInfo;

// The GPIO module uses BCM pin numbering. BCM 18 equates to physical pin 12.
const GPIO_PIN: u8 = 23;

#[get("/high")]
fn high(gpio: State<Mutex<GPIO>>) -> &'static str {
    let gpio = gpio.lock().unwrap();
    gpio.write(GPIO_PIN, Level::High);
    "High"
}

#[get("/low")]
fn low(gpio: State<Mutex<GPIO>>) -> &'static str {
    let gpio = gpio.lock().unwrap();
    gpio.write(GPIO_PIN, Level::Low);
    "Low"
}

fn main() {
    let device_info = DeviceInfo::new().unwrap();
    println!("Model: {} (SoC: {})", device_info.model(), device_info.soc());

    let mut gpio = GPIO::new().unwrap();
    gpio.set_mode(GPIO_PIN, Mode::Output);

    rocket::ignite().manage(Mutex::new(gpio)).mount("/", routes![low, high]).launch();
}
