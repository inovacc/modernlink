use core::{Error, Request, Response};

pub fn execute(request: &Request) -> Result<Response, Error> {
    let client = tls::build_client(tls::TlsConfig::secure_default(), request.connect_timeout, request.read_timeout)
        .map_err(|error| Error::Transport(error.to_string()))?;
    let method = request
        .method
        .parse()
        .map_err(|error| Error::InvalidRequest(format!("invalid method: {error}")))?;
    let mut call = client.request(method, &request.url);
    for (name, value) in &request.headers {
        call = call.header(name, value);
    }
    if !request.body.is_empty() {
        call = call.body(request.body.clone());
    }
    let response = call
        .send()
        .map_err(|error| Error::Transport(error.to_string()))?;
    let peer_certificates_der = tls::peer_certificates(&response);
    let status = response.status().as_u16();
    let mut headers = std::collections::BTreeMap::new();
    for (name, value) in response.headers() {
        if let Ok(value) = value.to_str() {
            headers.insert(name.to_string(), value.to_string());
        }
    }
    let body = response
        .bytes()
        .map_err(|error| Error::Transport(error.to_string()))?
        .to_vec();
    Ok(Response { status, headers, body, tls: if peer_certificates_der.is_empty() { None } else { Some(core::TlsInfo {
        protocol: None,
        cipher_suite: None,
        peer_certificates_der,
    }) } })
}
