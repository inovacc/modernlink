use core::{Error, Request, Response};

pub fn execute(request: &Request) -> Result<Response, Error> {
    let mut builder = reqwest::blocking::Client::builder();
    if let Some(timeout) = request.connect_timeout {
        builder = builder.connect_timeout(timeout);
    }
    if let Some(timeout) = request.read_timeout {
        builder = builder.timeout(timeout);
    }
    let client = builder
        .build()
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
    Ok(Response { status, headers, body, tls: None })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
