use bytes::Bytes;
use core::{Error, Request, Response, TlsInfo};
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::client::conn::http1;
use hyper::header::{HeaderName, HeaderValue};
use hyper::{Method, Request as HyperRequest, Uri};
use hyper_util::rt::TokioIo;
use std::net::ToSocketAddrs;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::time::timeout;
use tokio_rustls::TlsConnector;

pub fn execute(request: &Request) -> Result<Response, Error> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()
        .map_err(|error| Error::Transport(error.to_string()))?;
    runtime.block_on(execute_async(request))
}

async fn execute_async(request: &Request) -> Result<Response, Error> {
    let mut current = request.clone();
    let mut redirects = 0u32;
    loop {
        let response = execute_once_async(&current).await?;
        let location = response.headers.get("location").cloned();
        let redirect_status = matches!(response.status, 301 | 302 | 303 | 307 | 308);
        if !current.follow_redirects || !redirect_status || location.is_none() {
            return Ok(response);
        }
        if redirects >= current.max_redirects {
            return Ok(response);
        }
        let target = redirect_target(&current.url, location.as_ref().unwrap())?;
        if target.scheme() != "https" {
            return Err(Error::InvalidRequest(
                "redirect target must use https://".to_string(),
            ));
        }
        current.url = target.to_string();
        if response.status == 301 || response.status == 302 || response.status == 303 {
            current.method = "GET".to_string();
            current.body.clear();
        }
        redirects += 1;
    }
}

fn redirect_target(current_url: &str, location: &str) -> Result<url::Url, Error> {
    let target = url::Url::parse(location)
        .or_else(|_| url::Url::parse(current_url).and_then(|base| base.join(location)))
        .map_err(|error| Error::InvalidRequest(error.to_string()))?;
    if target.scheme() != "https" {
        return Err(Error::InvalidRequest(
            "redirect target must use https://".to_string(),
        ));
    }
    Ok(target)
}

async fn execute_once_async(request: &Request) -> Result<Response, Error> {
    let parsed =
        url::Url::parse(&request.url).map_err(|error| Error::InvalidRequest(error.to_string()))?;
    let host = parsed
        .host_str()
        .ok_or_else(|| Error::InvalidRequest("URL has no host".to_string()))?;
    let port = parsed
        .port_or_known_default()
        .ok_or_else(|| Error::InvalidRequest("URL has no port".to_string()))?;
    let address = format!("{}:{}", host, port)
        .to_socket_addrs()
        .map_err(|error| Error::Transport(error.to_string()))?
        .next()
        .ok_or_else(|| Error::Transport("host has no addresses".to_string()))?;
    let connect = TcpStream::connect(address);
    let tcp = if let Some(duration) = request.connect_timeout {
        timeout(duration, connect)
            .await
            .map_err(|_| Error::Transport("connection timeout".to_string()))?
    } else {
        connect.await
    }
    .map_err(|error| Error::Transport(error.to_string()))?;
    let server_name = rustls::pki_types::ServerName::try_from(host.to_string())
        .map_err(|error| Error::Tls(error.to_string()))?;
    let connector = TlsConnector::from(tls::client_config(tls::TlsConfig::with_minimum_version(
        request.minimum_tls_version,
    )));
    let tls_connect = connector.connect(server_name, tcp);
    let tls_stream = if let Some(duration) = request.connect_timeout {
        timeout(duration, tls_connect)
            .await
            .map_err(|_| Error::Transport("TLS handshake timeout".to_string()))?
    } else {
        tls_connect.await
    }
    .map_err(|error| Error::Tls(error.to_string()))?;
    let (_, session) = tls_stream.get_ref();
    let tls_info = TlsInfo {
        protocol: session
            .protocol_version()
            .map(|value| format!("{:?}", value)),
        cipher_suite: session
            .negotiated_cipher_suite()
            .map(|value| format!("{:?}", value.suite())),
        peer_certificates_der: session
            .peer_certificates()
            .map(|values| values.iter().map(|value| value.as_ref().to_vec()).collect())
            .unwrap_or_default(),
    };
    let io = TokioIo::new(tls_stream);
    let (mut sender, connection) = http1::handshake::<_, Full<Bytes>>(io)
        .await
        .map_err(|error| Error::Transport(error.to_string()))?;
    tokio::spawn(async move {
        let _ = connection.await;
    });
    let method = request
        .method
        .parse::<Method>()
        .map_err(|error| Error::InvalidRequest(error.to_string()))?;
    let path = match (parsed.path(), parsed.query()) {
        (path, Some(query)) => format!("{}?{}", path, query),
        (path, None) => {
            if path.is_empty() {
                "/".to_string()
            } else {
                path.to_string()
            }
        }
    };
    let uri: Uri = path
        .parse::<Uri>()
        .map_err(|error| Error::InvalidRequest(error.to_string()))?;
    let mut builder = HyperRequest::builder().method(method).uri(uri);
    let headers = builder
        .headers_mut()
        .ok_or_else(|| Error::InvalidRequest("request headers unavailable".to_string()))?;
    headers.insert(
        "host",
        host_header(host, port)
            .parse::<HeaderValue>()
            .map_err(|error| Error::InvalidRequest(error.to_string()))?,
    );
    for (name, value) in &request.headers {
        let name = name
            .parse::<HeaderName>()
            .map_err(|error| Error::InvalidRequest(error.to_string()))?;
        let value = value
            .parse::<HeaderValue>()
            .map_err(|error| Error::InvalidRequest(error.to_string()))?;
        headers.insert(name, value);
    }
    let outgoing = builder
        .body(Full::new(Bytes::from(request.body.clone())))
        .map_err(|error| Error::InvalidRequest(error.to_string()))?;
    let response = if let Some(duration) = request.read_timeout {
        timeout(duration, sender.send_request(outgoing))
            .await
            .map_err(|_| Error::Transport("read timeout".to_string()))?
    } else {
        sender.send_request(outgoing).await
    }
    .map_err(|error| Error::Transport(error.to_string()))?;
    collect_response(
        response,
        tls_info,
        request.read_timeout,
        request.url.clone(),
    )
    .await
}

async fn collect_response(
    response: hyper::Response<Incoming>,
    tls: TlsInfo,
    read_timeout: Option<Duration>,
    final_url: String,
) -> Result<Response, Error> {
    let status = response.status().as_u16();
    let status_message = response
        .status()
        .canonical_reason()
        .unwrap_or("")
        .to_string();
    let mut headers = std::collections::BTreeMap::new();
    for (name, value) in response.headers() {
        if let Ok(value) = value.to_str() {
            headers.insert(name.to_string(), value.to_string());
        }
    }
    let body = if let Some(duration) = read_timeout {
        timeout(duration, response.collect())
            .await
            .map_err(|_| Error::Transport("read timeout".to_string()))?
    } else {
        response.collect().await
    }
    .map_err(|error| Error::Transport(error.to_string()))?
    .to_bytes()
    .to_vec();
    Ok(Response {
        final_url,
        status,
        status_message,
        headers,
        body,
        tls: Some(tls),
    })
}

fn host_header(host: &str, port: u16) -> String {
    if port == 443 {
        host.to_string()
    } else if host.contains(':') {
        format!("[{}]:{}", host, port)
    } else {
        format!("{}:{}", host, port)
    }
}

#[cfg(test)]
mod tests {
    use super::{host_header, redirect_target};
    use core::Error;

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
}
