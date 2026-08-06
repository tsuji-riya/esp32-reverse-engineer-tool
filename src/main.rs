#![no_std]
#![no_main]
#![feature(impl_trait_in_assoc_type)]
extern crate alloc;

use alloc::borrow::ToOwned;
use defmt::{error, info, println};
use embassy_executor::Spawner;
use embedded_io_async::{Read, Write};
use esp_hal::clock::CpuClock;
use esp_hal::interrupt::software::SoftwareInterruptControl;
use esp_hal::timer::timg::TimerGroup;

#[allow(unused_imports)]
use esp_println as _;

use crate::blinky::{led_blink_task, BLINK_CHANNEL};
use crate::uart::UartStack;
use crate::web::setup::setup_web;
use crate::wifi::{connection_task, net_task, setup_wifi};
#[allow(unused_imports)]
use esp_backtrace as _;
use esp_hal::gpio::{Level, Output, OutputConfig};
use esp_hal::system::Stack;
use esp_radio::wifi::WifiController;
use esp_rtos::embassy::Executor;
use static_cell::StaticCell;

mod blinky;
mod uart;
mod util;
mod web;
mod wifi;

const SSID: &str = env!("SSID");
const PASSWORD: &str = env!("PASSWORD");

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

pub static CONTROLLER: StaticCell<WifiController<'static>> = StaticCell::new();

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
    let (stack, wifi_controller, runner) = setup_wifi(spawner, peripherals.WIFI, SSID, PASSWORD);

    let wifi_controller = CONTROLLER.init(wifi_controller);
    let led_pin = Output::new(peripherals.GPIO2, Level::Low, OutputConfig::default());
    let receiver = BLINK_CHANNEL.receiver();

    esp_rtos::start_second_core(
        peripherals.CPU_CTRL,
        sw_interrupt.software_interrupt1,
        app_core_stack,
        move || {
            static EXECUTOR: StaticCell<Executor> = StaticCell::new();
            let executor = EXECUTOR.init(Executor::new());
            executor.run(|spawner| {
                spawner.spawn(connection_task(wifi_controller).unwrap());
                spawner.spawn(led_blink_task(led_pin, receiver).unwrap());
            });
        },
    );

    spawner.spawn(net_task(runner).unwrap());

    stack.wait_config_up().await;

    if let Some(config) = stack.config_v4() {
        info!("Got IP: {}", config.address);
    }

    let sender = BLINK_CHANNEL.sender();

    let mut uart_stack = UartStack::new(peripherals.UART1, peripherals.GPIO12, peripherals.GPIO13);
    let config = esp_hal::uart::Config::default();
    if let Err(e) = uart_stack.initialize(spawner, config) {
        error!("Failed to initialize UART: {}", e);
        return;
    };

    setup_web(spawner, stack, sender);
}
