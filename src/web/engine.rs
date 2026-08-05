use embassy_net::tcp::{Error, TcpSocket};
use embedded_io_async::Write;
use heapless::{String, Vec};

pub const MAX_HEADERS: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    Get,
    Post,
    Head,
    Other,
}

impl Method {
    fn parse(s: &str) -> Method {
        match s {
            "GET" => Method::Get,
            "POST" => Method::Post,
            "HEAD" => Method::Head,
            _ => Method::Other,
        }
    }
}

#[derive(Debug)]
pub struct Header<'a> {
    pub name: &'a str,
    pub value: &'a str,
}

#[derive(Debug)]
pub struct Request<'a> {
    pub method: Method,
    pub path: &'a str,
    pub version: &'a str,
    pub headers: Vec<Header<'a>, MAX_HEADERS>,
    pub body: &'a [u8],
}

#[derive(Debug)]
pub enum HttpError {
    /// ヘッダ終端(\r\n\r\n)や body がまだ届いていない -> 追加でreadが必要
    Incomplete,
    Malformed,
    TooManyHeaders,
}

pub fn parse_request(buf: &[u8]) -> Result<Request, HttpError> {
    let text = core::str::from_utf8(buf).map_err(|_| HttpError::Malformed)?;

    let header_end = text.find("\r\n\r\n").ok_or(HttpError::Incomplete)?;
    let head = &text[..header_end];
    let mut lines = head.split("\r\n");

    let request_line = lines.next().ok_or(HttpError::Malformed)?;
    let mut parts = request_line.split(' ');
    let method_str = parts.next().ok_or(HttpError::Malformed)?;
    let path = parts.next().ok_or(HttpError::Malformed)?;
    let version = parts.next().ok_or(HttpError::Malformed)?;

    let mut headers: Vec<Header, MAX_HEADERS> = Vec::new();
    let mut content_length: usize = 0;

    for line in lines {
        if line.is_empty() {
            continue;
        }
        let (name, value) = line.split_once(':').ok_or(HttpError::Malformed)?;
        let name = name.trim();
        let value = value.trim();
        if name.eq_ignore_ascii_case("Content-Length") {
            content_length = value.parse().map_err(|_| HttpError::Malformed)?;
        }
        headers
            .push(Header { name, value })
            .map_err(|_| HttpError::TooManyHeaders)?;
    }

    let body_start = header_end + 4;
    let available_body = buf.len().saturating_sub(body_start);
    if available_body < content_length {
        return Err(HttpError::Incomplete);
    }
    let body = &buf[body_start..body_start + content_length];

    Ok(Request {
        method: Method::parse(method_str),
        path,
        version,
        headers,
        body,
    })
}

pub async fn write_response(
    socket: &mut TcpSocket<'_>,
    status: u16,
    reason: &str,
    content_type: &str,
    body: &[u8],
) -> Result<(), Error> {
    use core::fmt::Write;
    let mut head: String<256> = String::new();
    let _ = write!(
        head,
        "HTTP/1.1 {status} {reason}\r\n\
         Content-Type: {content_type}\r\n\
         Content-Length: {}\r\n\
         Connection: close\r\n\
         \r\n",
        body.len()
    );

    let head_bytes = head.as_bytes();
    let total = head_bytes.len() + body.len();

    socket.write_all(head_bytes).await?;
    socket.write_all(body).await?;
    Ok(())
}
