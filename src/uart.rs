use alloc::string::ToString;
use core::any::Any;
use defmt::error;
use esp_hal::Async;
use esp_hal::gpio::interconnect::{PeripheralInput, PeripheralOutput};
use esp_hal::peripherals::UART1;
use esp_hal::uart::{AtCmdConfig, Config, ConfigError, Uart};

// fifo_full_threshold (RX)
pub const READ_BUF_SIZE: usize = 64;
// EOT (CTRL-D)
const AT_CMD: u8 = 0x04;

struct UartStack<'a, TX, RX>
where
    TX: PeripheralOutput<'a>,
    RX: PeripheralInput<'a>,
{
    uart1: Option<UART1<'a>>,
    tx: Option<TX>,
    rx: Option<RX>,
    uart: Option<Uart<'a, Async>>,
}

impl<'a, TX, RX> UartStack<'a, TX, RX>
where
    TX: PeripheralOutput<'a>,
    RX: PeripheralInput<'a>,
{
    fn new(uart1: UART1<'a>, tx: TX, rx: RX) -> Self {
        Self {
            uart1: Some(uart1),
            tx: Some(tx),
            rx: Some(rx),
            uart: None,
        }
    }

    fn initialize(&mut self, config: Config) -> Result<(), &str> {
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

        Ok(())
    }

    fn apply_config(&mut self, config: &Config) -> Result<(), &str> {
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
