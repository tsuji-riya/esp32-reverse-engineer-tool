use crate::{mk_static, PASSWORD, SSID};
use defmt::info;
use embassy_executor::Spawner;
use embassy_net::{DhcpConfig, Runner, Stack, StackResources};
use embassy_time::{Duration, Timer};
use esp_hal::peripherals;
use esp_hal::rng::Rng;
use esp_radio::wifi::sta::StationConfig;
use esp_radio::wifi::{Config, ControllerConfig, Interface, WifiController};

const ESP_RADIO_RESOURCE: usize = 1;
const EMBASSY_NET_RESOURCE: usize = 1;
pub const WEB_WORKERS_SIZE: usize = 4;
pub const STACK_RESOURCE_SIZE: usize = ESP_RADIO_RESOURCE + EMBASSY_NET_RESOURCE + WEB_WORKERS_SIZE;

pub fn setup_wifi(
    spawner: Spawner,
    wifi_peripheral: peripherals::WIFI<'static>,
    ssid: &str,
    password: &str,
) -> (
    Stack<'static>,
    WifiController<'static>,
    Runner<'static, Interface<'static>>,
) {
    let station_config = Config::Station(
        StationConfig::default()
            .with_ssid(ssid)
            .with_password(password.into()),
    );

    let (wifi_controller, interfaces) = esp_radio::wifi::new(
        wifi_peripheral,
        ControllerConfig::default().with_initial_config(station_config),
    )
    .expect("Failed to initialize Wi-Fi controller");
    info!("Starting wifi");

    let mut dhcp_config = DhcpConfig::default();
    dhcp_config.hostname = Some("esp32-reverse-engineer-tool".try_into().unwrap());
    let config = embassy_net::Config::dhcpv4(dhcp_config);

    let rng = Rng::new();
    let seed = (rng.random() as u64) << 32 | rng.random() as u64;
    let (stack, runner) = embassy_net::new(
        interfaces.station,
        config,
        mk_static!(
            StackResources<STACK_RESOURCE_SIZE>,
            StackResources::<STACK_RESOURCE_SIZE>::new()
        ),
        seed,
    );

    return (stack, wifi_controller, runner);
}

#[embassy_executor::task]
pub async fn connection_task(controller: &'static mut WifiController<'static>) {
    info!("start connection task");

    loop {
        info!("About to connect...");

        match controller.connect_async().await {
            Ok(info) => {
                info!("Wifi connected to {:?}", info);

                // wait until we're no longer connected
                let info = controller.wait_for_disconnect_async().await.ok();
                info!("Disconnected: {:?}", info);
            }
            Err(e) => {
                info!("Failed to connect to wifi: {:?}", e);
            }
        }

        Timer::after(Duration::from_millis(5000)).await
    }
}

#[embassy_executor::task]
pub async fn net_task(mut runner: Runner<'static, Interface<'static>>) {
    runner.run().await
}
