use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::{Channel, Receiver, Sender};
use embassy_time::{Duration, Timer};
use esp_hal::gpio::Output;

pub static BLINK_CHANNEL: Channel<CriticalSectionRawMutex, (), 3> = Channel::new();
pub type BlinkSender = Sender<'static, CriticalSectionRawMutex, (), 3>;

#[embassy_executor::task]
pub async fn led_blink_task(
    mut led: Output<'static>,
    receiver: Receiver<'static, CriticalSectionRawMutex, (), 3>,
) {
    loop {
        receiver.receive().await;

        led.set_high();
        Timer::after(Duration::from_millis(50)).await;
        led.set_low();
        Timer::after(Duration::from_millis(50)).await;
    }
}
