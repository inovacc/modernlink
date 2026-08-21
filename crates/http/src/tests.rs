use super::{collect_response, execute, host_header, redirect_target, redirected_request};
use bytes::Bytes;
use http_body_util::Full;
use hyper::client::conn::http1;
use hyper::Request as HyperRequest;
use hyper_util::rt::TokioIo;
use modernlink_core::{Error, Request, Response, TlsInfo};
use std::future::Future;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::time::Duration;
use tokio::net::TcpStream;

fn run_async<F: Future>(future: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()
        .unwrap()
        .block_on(future)
}

fn request_with_url(url: &str) -> Request {
    let mut request = Request::new("https://example.com").unwrap();
    request.url = url.to_string();
    request.follow_redirects = false;
    request
}

fn test_tls_info() -> TlsInfo {
    TlsInfo {
        protocol: Some("TLSv1_3".to_string()),
        cipher_suite: Some("TLS13_AES_128_GCM_SHA256".to_string()),
        peer_certificates_der: vec![vec![1, 2, 3]],
    }
}

fn redirect_response(status: u16, location: Option<&str>) -> Response {
    let mut headers = std::collections::BTreeMap::new();
    if let Some(location) = location {
        headers.insert("location".to_string(), location.to_string());
    }
    Response {
        final_url: "https://example.com/start".to_string(),
        status,
        status_message: "redirect".to_string(),
        headers,
        body: Vec::new(),
        tls: None,
    }
}

#[test]
fn redirect_policy_returns_or_rewrites_without_network_io() {
    let mut request = Request::new("https://example.com/start").unwrap();
    request.method = "POST".to_string();
    request.body = b"body".to_vec();

    request.follow_redirects = false;
    assert!(
        redirected_request(&request, &redirect_response(302, Some("/next")), 0)
            .unwrap()
            .is_none()
    );
    request.follow_redirects = true;
    assert!(
        redirected_request(&request, &redirect_response(200, Some("/next")), 0)
            .unwrap()
            .is_none()
    );
    assert!(
        redirected_request(&request, &redirect_response(302, None), 0)
            .unwrap()
            .is_none()
    );
    assert!(redirected_request(
        &request,
        &redirect_response(302, Some("/next")),
        request.max_redirects,
    )
    .unwrap()
    .is_none());

    let rewritten = redirected_request(&request, &redirect_response(302, Some("/next")), 0)
        .unwrap()
        .unwrap();
    assert_eq!(rewritten.url, "https://example.com/next");
    assert_eq!(rewritten.method, "GET");
    assert!(rewritten.body.is_empty());

    let preserved = redirected_request(
        &request,
        &redirect_response(307, Some("https://other.example/next")),
        0,
    )
    .unwrap()
    .unwrap();
    assert_eq!(preserved.method, "POST");
    assert_eq!(preserved.body, b"body");

    assert!(matches!(
        redirected_request(&request, &redirect_response(308, Some("http://example.com")), 0),
        Err(Error::InvalidRequest(message)) if message.contains("https://")
    ));
}

fn collect_response_from_wire(
    chunks: Vec<(&'static [u8], Option<Duration>)>,
    read_timeout: Option<Duration>,
) -> Result<Response, Error> {
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let address = listener.local_addr().unwrap();
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut request = [0u8; 4096];
        let _ = stream.read(&mut request);
        for (chunk, delay_after) in chunks {
            let _ = stream.write_all(chunk);
            let _ = stream.flush();
            if let Some(delay) = delay_after {
                std::thread::sleep(delay);
            }
        }
    });

    let result = run_async(async move {
        let stream = TcpStream::connect(address).await.unwrap();
        let io = TokioIo::new(stream);
        let (mut sender, connection) = http1::handshake::<_, Full<Bytes>>(io).await.unwrap();
        tokio::spawn(async move {
            let _ = connection.await;
        });
        let request = HyperRequest::builder()
            .method("GET")
            .uri("/")
            .body(Full::new(Bytes::new()))
            .unwrap();
        let response = sender.send_request(request).await.unwrap();
        collect_response(
            response,
            test_tls_info(),
            read_timeout,
            "https://example.com/final".to_string(),
        )
        .await
    });
    server.join().unwrap();
    result
}

#[test]
fn resolves_relative_https_redirects() {
    let target = redirect_target("https://example.com/path/start", "/next").unwrap();
    assert_eq!(target.as_str(), "https://example.com/next");
}

#[test]
fn rejects_non_https_redirects() {
    let error = redirect_target("https://example.com/path", "http://example.com/next");
    assert_eq!(
        error,
        Err(Error::InvalidRequest(
            "redirect target must use https://".to_string()
        ))
    );
}

#[test]
fn resolves_query_only_redirects() {
    let target = redirect_target("https://example.com/path?old=1", "?new=2").unwrap();
    assert_eq!(target.as_str(), "https://example.com/path?new=2");
}

#[test]
fn formats_host_header_for_default_and_custom_ports() {
    assert_eq!(host_header("example.com", 443), "example.com");
    assert_eq!(host_header("example.com", 8443), "example.com:8443");
    assert_eq!(host_header("::1", 8443), "[::1]:8443");
}

/// A protocol-relative redirect inherits the scheme of the page it came from. Because
/// the base is always https here, `//host/path` must resolve to https and be allowed -
/// if it resolved to http it would be a silent downgrade of a secure request.
#[test]
fn resolves_protocol_relative_redirects_to_https() {
    let target = redirect_target("https://example.com/a", "//other.example.com/b").unwrap();
    assert_eq!(target.as_str(), "https://other.example.com/b");
}

#[test]
fn resolves_fragment_only_redirects() {
    let target = redirect_target("https://example.com/a?q=1", "#section").unwrap();
    assert_eq!(target.as_str(), "https://example.com/a?q=1#section");
}

/// An empty Location resolves back to the current URL. That is a redirect loop rather
/// than an error, and it is safe only because the caller bounds `max_redirects` - this
/// pins the resolution so the bound stays the thing that stops it.
#[test]
fn an_empty_location_resolves_to_the_current_url() {
    let target = redirect_target("https://example.com/a", "").unwrap();
    assert_eq!(target.as_str(), "https://example.com/a");
}

#[test]
fn a_redirect_to_another_host_is_allowed_when_it_stays_https() {
    let target = redirect_target("https://a.example.com/x", "https://b.example.com/y").unwrap();
    assert_eq!(target.host_str(), Some("b.example.com"));
}

/// A relative redirect must not silently drop a non-default port - landing on 443
/// instead of 8443 would quietly contact a different service.
#[test]
fn a_relative_redirect_preserves_a_non_default_port() {
    let target = redirect_target("https://example.com:8443/a", "/b").unwrap();
    assert_eq!(target.as_str(), "https://example.com:8443/b");
    assert_eq!(target.port(), Some(8443));
}

#[test]
fn rejects_non_http_schemes_outright() {
    for location in [
        "ftp://example.com/f",
        "file:///etc/passwd",
        "data:text/plain,x",
    ] {
        assert!(
            redirect_target("https://example.com/a", location).is_err(),
            "must reject {location}"
        );
    }
}

/// Neither the location nor the base parses, so there is nothing to resolve against.
#[test]
fn reports_an_unparseable_base_rather_than_panicking() {
    let error = redirect_target("not a url", "also not a url");
    assert!(matches!(error, Err(Error::InvalidRequest(_))), "{error:?}");
}

#[test]
fn formats_host_header_for_ipv6_on_the_default_port() {
    // 443 returns the bare host, so an IPv6 literal comes back unbracketed here.
    assert_eq!(host_header("::1", 443), "::1");
    assert_eq!(host_header("2001:db8::1", 8443), "[2001:db8::1]:8443");
}

#[test]
fn execute_reports_invalid_urls_before_connecting() {
    let error = execute(&request_with_url("not a url")).unwrap_err();

    assert!(matches!(error, Error::InvalidRequest(_)), "{error:?}");
}

#[test]
fn execute_rejects_an_empty_host_before_connecting() {
    let error = execute(&request_with_url("https://:443/missing-host")).unwrap_err();

    assert!(matches!(error, Error::InvalidRequest(_)), "{error:?}");
}

#[test]
fn execute_reports_an_unknown_scheme_without_a_port() {
    let error = execute(&request_with_url("custom://example.com/path")).unwrap_err();

    assert_eq!(error, Error::InvalidRequest("URL has no port".to_string()));
}

#[test]
fn execute_reports_a_tls_failure_when_the_peer_closes() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let address = listener.local_addr().unwrap();
    let server = std::thread::spawn(move || {
        let _ = listener.accept();
    });
    let url = format!("https://127.0.0.1:{}/", address.port());

    let error = execute(&request_with_url(&url)).unwrap_err();

    server.join().unwrap();
    assert!(matches!(error, Error::Tls(_)), "{error:?}");
}

#[test]
fn collect_response_preserves_status_headers_body_and_tls_metadata() {
    let response = collect_response_from_wire(
            vec![
                (
                    b"HTTP/1.1 201 Created\r\nContent-Type: text/plain\r\nX-Test: first\r\nX-Test: second\r\nX-Binary: \xff\r\nContent-Length: 10\r\nConnection: close\r\n\r\nhello \xf0\x9f\x8c\x8d",
                    None,
                ),
            ],
            None,
        )
        .unwrap();

    assert_eq!(response.final_url, "https://example.com/final");
    assert_eq!(response.status, 201);
    assert_eq!(response.status_message, "Created");
    assert_eq!(
        response.headers.get("content-type"),
        Some(&"text/plain".to_string())
    );
    assert_eq!(response.headers.get("x-test"), Some(&"second".to_string()));
    assert!(!response.headers.contains_key("x-binary"));
    assert_eq!(response.body, b"hello \xf0\x9f\x8c\x8d");
    assert_eq!(
        response.tls.unwrap().peer_certificates_der,
        vec![vec![1, 2, 3]]
    );
}

#[test]
fn collect_response_reports_a_read_timeout_while_waiting_for_the_body() {
    let error = collect_response_from_wire(
        vec![
            (
                b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\nConnection: close\r\n\r\n",
                Some(Duration::from_millis(100)),
            ),
            (b"hello", None),
        ],
        Some(Duration::from_millis(20)),
    )
    .unwrap_err();

    assert_eq!(error, Error::Transport("read timeout".to_string()));
}
