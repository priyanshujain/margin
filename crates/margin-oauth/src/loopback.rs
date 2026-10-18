use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::time::{Duration, Instant};

use crate::CANCELLED;

/// Names shown on the page the browser lands on and in the errors.
#[derive(Debug, Clone, Copy)]
pub struct Pages<'a> {
    pub app: &'a str,
    pub provider: &'a str,
}

/// A listener on an ephemeral loopback port and the redirect URI that reaches it.
pub fn bind_loopback() -> Result<(TcpListener, String), String> {
    let listener = TcpListener::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
    let port = listener.local_addr().map_err(|e| e.to_string())?.port();
    Ok((listener, format!("http://127.0.0.1:{port}")))
}

fn write_http_message(stream: &mut TcpStream, title: &str, message: &str) {
    let body = format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><title>{title}</title></head>\
         <body style=\"font-family:system-ui,sans-serif;text-align:center;padding-top:80px;color:#222\">\
         <h2>{message}</h2></body></html>"
    );
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    let _ = stream.write_all(response.as_bytes());
    let _ = stream.flush();
}

#[derive(Debug, PartialEq, Eq)]
pub enum Redirect {
    Code(String),
    Denied(String),
    Mismatch,
    /// Anything else the browser asked for on the way, including the favicon.
    Waiting,
}

pub fn request_path(request: &str) -> &str {
    request
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .unwrap_or("")
}

pub fn parse_redirect(path: &str, expected_state: &str) -> Redirect {
    if path == "/favicon.ico" {
        return Redirect::Waiting;
    }
    let parsed = match url::Url::parse(&format!("http://127.0.0.1{path}")) {
        Ok(parsed) => parsed,
        Err(_) => return Redirect::Waiting,
    };
    let mut code = None;
    let mut state = None;
    let mut error = None;
    for (key, value) in parsed.query_pairs() {
        match key.as_ref() {
            "code" => code = Some(value.into_owned()),
            "state" => state = Some(value.into_owned()),
            "error" => error = Some(value.into_owned()),
            _ => {}
        }
    }
    if let Some(error) = error {
        return Redirect::Denied(error);
    }
    match (code, state) {
        (Some(code), Some(state)) if state == expected_state => Redirect::Code(code),
        (Some(_), _) => Redirect::Mismatch,
        _ => Redirect::Waiting,
    }
}

/// Blocks until the browser comes back with a code, the deadline passes, or `cancelled` says the
/// consent surface was closed. `access_denied` is the user pressing Cancel and returns `CANCELLED`.
pub fn await_code(
    listener: TcpListener,
    expected_state: &str,
    deadline: Instant,
    pages: Pages<'_>,
    cancelled: impl Fn() -> bool,
) -> Result<String, String> {
    let Pages { app, provider } = pages;
    listener.set_nonblocking(true).map_err(|e| e.to_string())?;
    loop {
        if Instant::now() > deadline {
            return Err(format!("Timed out waiting for {provider} authorization."));
        }
        if cancelled() {
            return Err(CANCELLED.to_string());
        }
        match listener.accept() {
            Ok((mut stream, _)) => {
                stream.set_nonblocking(false).ok();
                stream.set_read_timeout(Some(Duration::from_secs(5))).ok();
                let mut buf = [0u8; 8192];
                let n = stream.read(&mut buf).unwrap_or(0);
                let request = String::from_utf8_lossy(&buf[..n]);
                match parse_redirect(request_path(&request), expected_state) {
                    Redirect::Code(code) => {
                        write_http_message(
                            &mut stream,
                            app,
                            &format!("Connected to {app}. You can close this tab."),
                        );
                        return Ok(code);
                    }
                    Redirect::Denied(error) => {
                        write_http_message(
                            &mut stream,
                            app,
                            "Authorization was cancelled. You can close this tab.",
                        );
                        if error == "access_denied" {
                            return Err(CANCELLED.to_string());
                        }
                        return Err(format!("{provider} authorization failed: {error}"));
                    }
                    Redirect::Mismatch => {
                        write_http_message(
                            &mut stream,
                            app,
                            "Could not verify the request. You can close this tab.",
                        );
                        return Err(format!("State mismatch during {provider} authorization."));
                    }
                    Redirect::Waiting => {
                        write_http_message(&mut stream, app, &format!("Waiting for {provider}…"))
                    }
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(150));
            }
            Err(e) => return Err(e.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_path_reads_the_target_of_the_request_line() {
        let request = "GET /?code=abc HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n";
        assert_eq!(request_path(request), "/?code=abc");
        assert_eq!(request_path(""), "");
    }

    #[test]
    fn a_matching_state_yields_the_code() {
        assert_eq!(
            parse_redirect("/?code=4%2F0Ab&state=xyz&scope=openid", "xyz"),
            Redirect::Code("4/0Ab".to_string())
        );
    }

    #[test]
    fn a_foreign_state_is_rejected_rather_than_exchanged() {
        assert_eq!(
            parse_redirect("/?code=4/0Ab&state=other", "xyz"),
            Redirect::Mismatch
        );
        assert_eq!(parse_redirect("/?code=4/0Ab", "xyz"), Redirect::Mismatch);
    }

    #[test]
    fn a_denial_carries_the_providers_reason() {
        assert_eq!(
            parse_redirect("/?error=access_denied&state=xyz", "xyz"),
            Redirect::Denied("access_denied".to_string())
        );
    }

    #[test]
    fn incidental_requests_keep_the_listener_waiting() {
        assert_eq!(parse_redirect("/favicon.ico", "xyz"), Redirect::Waiting);
        assert_eq!(parse_redirect("/", "xyz"), Redirect::Waiting);
        assert_eq!(parse_redirect("", "xyz"), Redirect::Waiting);
    }
}
