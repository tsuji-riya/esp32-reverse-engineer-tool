use crate::blinky::BlinkSender;
use crate::web::worker::http_worker;
use crate::wifi::WEB_WORKERS_SIZE;
use embassy_executor::Spawner;
use embassy_net::Stack;

pub fn setup_web(spawner: Spawner, stack: Stack<'static>, sender: BlinkSender) {
    for id in 0..WEB_WORKERS_SIZE {
        spawner.spawn(http_worker(stack, id, sender).unwrap());
    }
}
