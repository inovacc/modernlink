use super::{
    base64_decode, base64_encode, json_array, json_decode, json_object, uuid_v4, uuid_v7, Error,
    Request, TlsVersion,
};

#[test]
fn request_rejects_empty_url() {
    let error = Request::new("");
    assert!(error.is_err());
}

#[test]
fn request_rejects_whitespace_only_url() {
    assert!(matches!(
        Request::new(" \t\n"),
        Err(Error::InvalidRequest(message)) if message == "URL must not be empty"
    ));
}

#[test]
fn request_rejects_non_https_url() {
    assert!(matches!(
        Request::new("http://example.com"),
        Err(Error::InvalidRequest(message)) if message == "only https:// URLs are supported"
    ));
}

#[test]
fn request_accepts_https_url_with_get_and_empty_headers_by_default() {
    let request = Request::new("https://example.com").unwrap();
    assert_eq!(request.url, "https://example.com");
    assert_eq!(request.method, "GET");
    assert!(request.headers.is_empty());
    assert!(request.body.is_empty());
    assert!(request.connect_timeout.is_none());
    assert!(request.read_timeout.is_none());
    assert!(request.follow_redirects);
    assert_eq!(request.max_redirects, 10);
    assert_eq!(request.minimum_tls_version, TlsVersion::Tls12);
}

#[test]
fn uuid_v7_has_uuid_shape() {
    let value = uuid_v7();
    assert_eq!(value.len(), 36);
    assert_eq!(&value[14..15], "7");
}

#[test]
fn uuid_v4_has_uuid_shape() {
    let value = uuid_v4();
    assert_eq!(value.len(), 36);
    assert_eq!(&value[14..15], "4");
}

#[test]
fn base64_and_json_helpers_encode_data() {
    assert_eq!(base64_encode(b"modernlink"), "bW9kZXJubGluaw==");
    assert_eq!(base64_decode("bW9kZXJubGluaw==").unwrap(), b"modernlink");
    assert_eq!(
        json_decode("{ \"message\": \"hello\" }").unwrap(),
        "{\"message\":\"hello\"}"
    );
}

#[test]
fn base64_decode_rejects_invalid_input() {
    assert!(matches!(
        base64_decode("not base64"),
        Err(Error::InvalidRequest(_))
    ));
}

#[test]
fn json_object_rejects_unpaired_fields() {
    let fields = vec!["name".to_string()];

    assert!(matches!(
        json_object(&fields),
        Err(Error::InvalidRequest(message))
            if message == "JSON object fields must be name/value pairs"
    ));
}

#[test]
fn json_object_serializes_string_pairs_with_escaping() {
    let fields = vec![
        "message".to_string(),
        "hello".to_string(),
        "quote".to_string(),
        "a\"b".to_string(),
    ];

    assert_eq!(
        json_object(&fields).unwrap(),
        r#"{"message":"hello","quote":"a\"b"}"#
    );
}

#[test]
fn json_array_serializes_values_and_empty_arrays() {
    let values = vec!["modernlink".to_string(), "a\"b".to_string()];

    assert_eq!(json_array(&values).unwrap(), r#"["modernlink","a\"b"]"#);
    assert_eq!(json_array(&[]).unwrap(), "[]");
}

#[test]
fn json_decode_compacts_json_arrays() {
    assert_eq!(
        json_decode(r#"[ { "id": 1 }, true, null ]"#).unwrap(),
        r#"[{"id":1},true,null]"#
    );
}

#[test]
fn json_decode_rejects_invalid_json() {
    assert!(matches!(
        json_decode("{not valid json}"),
        Err(Error::InvalidRequest(_))
    ));
}

#[test]
fn error_display_preserves_messages_for_each_error_kind() {
    assert_eq!(
        Error::InvalidRequest("bad request".to_string()).to_string(),
        "bad request"
    );
    assert_eq!(
        Error::Transport("transport failed".to_string()).to_string(),
        "transport failed"
    );
    assert_eq!(
        Error::Tls("TLS failed".to_string()).to_string(),
        "TLS failed"
    );
    assert_eq!(
        Error::Native("native failed".to_string()).to_string(),
        "native failed"
    );
}
