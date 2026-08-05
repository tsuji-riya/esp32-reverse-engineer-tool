#![no_std]
#![no_main]
#![feature(impl_trait_in_assoc_type)]
extern crate alloc;

use crate::wifi::setup_wifi;
use alloc::borrow::ToOwned;
use defmt::println;
use embassy_executor::Spawner;
use embedded_io_async::{Read, Write};
use esp_hal::clock::CpuClock;
use esp_hal::interrupt::software::SoftwareInterruptControl;
use esp_hal::timer::timg::TimerGroup;

#[allow(unused_imports)]
use esp_println as _;

use crate::blinky::{BLINK_CHANNEL, led_blink_task};
use crate::web::setup::setup_web;
#[allow(unused_imports)]
use esp_backtrace as _;
use esp_hal::gpio::{Level, Output, OutputConfig};
use esp_hal::system::Stack;
use static_cell::StaticCell;

mod blinky;
mod global;
mod uart;
mod util;
mod web;
mod wifi;

const SSID: &str = env!("SSID");
const PASSWORD: &str = env!("PASSWORD");

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

#[esp_rtos::main]
async fn main(spawner: Spawner) {
    // generator version: 1.3.0
    // generator parameters: --chip esp32 -o embassy -o unstable-hal -o alloc -o wifi -o ci -o defmt

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    static APP_CORE_STACK: StaticCell<Stack<8192>> = StaticCell::new();
    let app_core_stack = APP_CORE_STACK.init(Stack::new());

    esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 98768);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let sw_interrupt = SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    esp_rtos::start(timg0.timer0, sw_interrupt.software_interrupt0);

    println!("Embassy initialized!");

    // Wi-Fi
    let stack = setup_wifi(spawner, peripherals.WIFI, SSID, PASSWORD).await;

    let led_pin = Output::new(peripherals.GPIO2, Level::Low, OutputConfig::default());
    let sender = BLINK_CHANNEL.sender();
    let receiver = BLINK_CHANNEL.receiver();
    spawner.spawn(led_blink_task(led_pin, receiver).unwrap());

    setup_web(spawner, stack, sender);
}
