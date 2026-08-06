use alloc::string::ToString;
use core::any::Any;
use core::fmt::Write;
use defmt::error;
use embassy_executor::Spawner;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::signal::Signal;
use esp_hal::gpio::interconnect::{PeripheralInput, PeripheralOutput};
use esp_hal::peripherals::UART1;
use esp_hal::uart::{AtCmdConfig, Config, ConfigError, Uart, UartRx, UartTx};
use esp_hal::Async;
use static_cell::StaticCell;

// fifo_full_threshold (RX)
pub const READ_BUF_SIZE: usize = 64;
// EOT (CTRL-D)
const AT_CMD: u8 = 0x04;

pub struct UartStack<TX, RX>
where
    TX: PeripheralOutput<'static>,
    RX: PeripheralInput<'static>,
{
    uart1: Option<UART1<'static>>,
    tx: Option<TX>,
    rx: Option<RX>,
    uart: Option<Uart<'static, Async>>,
}

impl<TX, RX> UartStack<TX, RX>
where
    TX: PeripheralOutput<'static>,
    RX: PeripheralInput<'static>,
{
    pub fn new(uart1: UART1<'static>, tx: TX, rx: RX) -> Self {
        Self {
            uart1: Some(uart1),
            tx: Some(tx),
            rx: Some(rx),
            uart: None,
        }
    }

    pub fn initialize(&mut self, spawner: Spawner, config: Config) -> Result<(), &str> {
        let tx = match self.tx.take() {
            Some(tx) => tx,
            None => {
                return Err("tx pin is already initialized!");
            }
        };
        let rx = match self.rx.take() {
            Some(rx) => rx,
            None => {
                return Err("rx pin is already initialized!");
            }
        };
        let uart1 = match self.uart1.take() {
            Some(uart1) => uart1,
            None => {
                return Err("uart1 is already initialized!");
            }
        };

        let mut uart = Uart::new(uart1, config)
            .unwrap()
            .with_tx(tx)
            .with_rx(rx)
            .into_async();
        uart.set_at_cmd(AtCmdConfig::default().with_cmd_char(AT_CMD));

        let (rx, tx) = uart.split();

        static SIGNAL: StaticCell<Signal<NoopRawMutex, usize>> = StaticCell::new();
        let signal = &*SIGNAL.init(Signal::new());

        spawner.spawn(reader(rx, &signal).unwrap());
        spawner.spawn(writer(tx, &signal).unwrap());

        Ok(())
    }

    pub fn apply_config(&mut self, config: &Config) -> Result<(), &str> {
        if let Some(mut uart) = self.uart.take() {
            match uart.apply_config(config) {
                Ok(_) => {
                    self.uart = Some(uart);
                    Ok(())
                }
                Err(err) => {
                    error!("uart error: {:?}", err);
                    Err("uart error")
                }
            }
        } else {
            Err("Cannot apply config: failed Option::take from self")
        }
    }
}

#[embassy_executor::task]
async fn writer(mut tx: UartTx<'static, Async>, signal: &'static Signal<NoopRawMutex, usize>) {
    loop {
        let bytes_read = signal.wait().await;
        signal.reset();
        write!(&mut tx, "\r\n-- received {} bytes --\r\n", bytes_read).unwrap();
        embedded_io_async::Write::flush(&mut tx).await.unwrap();
    }
}

#[embassy_executor::task]
async fn reader(mut rx: UartRx<'static, Async>, signal: &'static Signal<NoopRawMutex, usize>) {
    const MAX_BUFFER_SIZE: usize = 10 * READ_BUF_SIZE + 16;

    let mut rbuf: [u8; MAX_BUFFER_SIZE] = [0u8; MAX_BUFFER_SIZE];
    let mut offset = 0;
    loop {
        let r = embedded_io_async::Read::read(&mut rx, &mut rbuf[offset..]).await;
        match r {
            Ok(len) => {
                offset += len;
                esp_println::println!("Read: {len}, data: {:?}", &rbuf[..offset]);
                offset = 0;
                signal.signal(len);
            }
            Err(e) => esp_println::println!("RX Error: {:?}", e),
        }
    }
}
