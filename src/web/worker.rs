use crate::blinky::BlinkSender;
use crate::web::engine::{parse_request, write_response, HttpError};
use crate::web::mime::TEXT_MIME;
use crate::web::route::handle_request;
use embassy_net::tcp::TcpSocket;
use embassy_net::Stack;
use embassy_time::{Duration, Timer};

#[embassy_executor::task(pool_size = 4)]
pub async fn http_worker(stack: Stack<'static>, worker_id: usize, sender: BlinkSender) -> ! {
    let mut rx_buffer = [0u8; 2048];
    let mut tx_buffer = [0u8; 2048];
    let mut req_buffer = [0u8; 8192];

    loop {
        let mut socket = TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);
        socket.set_timeout(Some(Duration::from_secs(10)));

        if socket.accept(80).await.is_err() {
            Timer::after(Duration::from_millis(50)).await;
            continue;
        }
        let _ = sender.try_send(());

        // Content-Lengthが揃うまで読み進める簡易ループ
        let mut received = 0usize;
        let n = loop {
            if received == req_buffer.len() {
                break received; // バッファ超過。要求を打ち切って400を返す
            }
            match socket.read(&mut req_buffer[received..]).await {
                Ok(0) => break received,
                Ok(len) => {
                    received += len;
                    match parse_request(&req_buffer[..received]) {
                        Ok(_) => break received,
                        Err(HttpError::Incomplete) => continue,
                        Err(_) => break received,
                    }
                }
                Err(_) => break received,
            }
        };

        let (status, reason, mime, body) = match parse_request(&req_buffer[..n]) {
            Ok(req) => handle_request(&req),
            Err(_) => (400, "Bad Request", TEXT_MIME, &b"Bad Request"[..]),
        };

        let _ = write_response(&mut socket, status, reason, mime, body).await;
        let _ = socket.flush().await;
        socket.close();
        socket.abort();
    }
}
