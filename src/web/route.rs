use crate::web::engine::{Method, Request};
use crate::web::mime::{HTML_MIME, TEXT_MIME};
use embassy_net::tcp::TcpSocket;
use mimext::ext_to_mime;

include!("../../embedded_files.rs");

pub fn get_file(name: &str) -> Option<&'static [u8]> {
    EMBEDDED_FILES
        .binary_search_by(|(n, _)| n.cmp(&name))
        .ok()
        .map(|idx| EMBEDDED_FILES[idx].1)
}

pub fn handle_request<'a>(req: &Request) -> (u16, &'static str, &'static str, &'static [u8]) {
    match (req.method, req.path) {
        (Method::Get, "/health") => return (200, "OK", TEXT_MIME, b"OK"),
        _ => {}
    }

    // static content
    if req.method == Method::Get && req.path.len() > 0 {
        if req.path == "/" {
            return (200, "OK", HTML_MIME, get_file("index.html").unwrap());
        } else if let Some(file) = get_file(&req.path[1..]) {
            return (
                200,
                "OK",
                get_mime_from_filename(req.path).unwrap_or(TEXT_MIME),
                file,
            );
        }
    }

    (404, "Not Found", TEXT_MIME, b"Not Found")
}

pub fn handle_server_sent_event(mut socket: &TcpSocket, req: &Request) -> Result<(), ()> {
    Ok(())
}

fn get_mime_from_filename(filename: &str) -> Option<&'static str> {
    if filename.len() == 0 {
        return None;
    }

    if let Some(ext) = filename.split('.').last()
        && let Some(mime) = ext_to_mime(ext).last()
    {
        Some(mime)
    } else {
        None
    }
}
