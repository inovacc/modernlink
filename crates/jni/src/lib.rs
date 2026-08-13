use jni::objects::{JByteArray, JClass, JObjectArray, JString};
use jni::sys::{jbyteArray, jint, jlong, jobjectArray};
use jni::JNIEnv;
use core::{Request, Response, TlsVersion};
use std::time::Duration;
use std::cell::RefCell;

thread_local! {
    static LAST_ERROR: RefCell<String> = RefCell::new(String::new());
}

fn set_error(message: String) -> jlong {
    LAST_ERROR.with(|value| *value.borrow_mut() = message);
    0
}

#[cfg(target_os = "linux")]
#[no_mangle]
pub extern "C" fn getauxval(_kind: usize) -> usize { 0 }

struct NativeResponse(Response);

unsafe fn response<'a>(handle: jlong) -> Option<&'a NativeResponse> {
    if handle == 0 { None } else { Some(&*(handle as *const NativeResponse)) }
}

#[no_mangle]
pub extern "system" fn Java_com_modernlink_LegacyHttpClient_nativeExecute(
    mut env: JNIEnv,
    _class: JClass,
    url: JString,
    method: JString,
    headers: JObjectArray,
    body: JByteArray,
    connect_timeout_millis: jlong,
    read_timeout_millis: jlong,
    follow_redirects: jni::sys::jboolean,
    max_redirects: jint,
    minimum_tls_version: jint,
) -> jlong {
    let url = match env.get_string(&url).ok().and_then(|value| value.to_str().ok().map(str::to_owned)) {
        Some(value) => value,
        None => return set_error("invalid URL string".to_string()),
    };
    let method = match env.get_string(&method).ok().and_then(|value| value.to_str().ok().map(str::to_owned)) {
        Some(value) => value,
        None => return set_error("invalid method string".to_string()),
    };
    let mut request = match Request::new(&url) { Ok(value) => value, Err(error) => return set_error(error.to_string()) };
    request.method = method;
    if env.get_array_length(&headers).ok().unwrap_or(1) % 2 != 0 { return set_error("headers must contain name/value pairs".to_string()); }
    let header_count = env.get_array_length(&headers).ok().unwrap_or(0);
    for index in (0..header_count).step_by(2) {
        let name = match env.get_object_array_element(&headers, index).ok()
            .and_then(|value| env.get_string(&JString::from(value)).ok().and_then(|text| text.to_str().ok().map(str::to_owned))) {
            Some(value) => value,
            None => return set_error("invalid header name".to_string()),
        };
        let value = match env.get_object_array_element(&headers, index + 1).ok()
            .and_then(|value| env.get_string(&JString::from(value)).ok().and_then(|text| text.to_str().ok().map(str::to_owned))) {
            Some(value) => value,
            None => return set_error("invalid header value".to_string()),
        };
        request.headers.insert(name, value);
    }
    request.body = match env.convert_byte_array(&body) { Ok(value) => value, Err(error) => return set_error(error.to_string()) };
    if connect_timeout_millis > 0 { request.connect_timeout = Some(Duration::from_millis(connect_timeout_millis as u64)); }
    if read_timeout_millis > 0 { request.read_timeout = Some(Duration::from_millis(read_timeout_millis as u64)); }
    request.follow_redirects = follow_redirects != 0;
    if max_redirects < 0 { return set_error("maximum redirects must not be negative".to_string()); }
    request.max_redirects = max_redirects as u32;
    request.minimum_tls_version = match minimum_tls_version {
        12 => TlsVersion::Tls12,
        13 => TlsVersion::Tls13,
        _ => return set_error("unsupported minimum TLS version".to_string()),
    };
    let result = match http::execute(&request) { Ok(value) => value, Err(error) => return set_error(error.to_string()) };
    Box::into_raw(Box::new(NativeResponse(result))) as jlong
}

#[no_mangle]
pub extern "system" fn Java_com_modernlink_LegacyHttpClient_nativeLastError(env: JNIEnv, _class: JClass) -> jni::sys::jstring {
    let message = LAST_ERROR.with(|value| value.borrow().clone());
    match env.new_string(message) { Ok(value) => value.into_raw(), Err(_) => std::ptr::null_mut() }
}

#[no_mangle]
pub extern "system" fn Java_com_modernlink_LegacyHttpClient_nativeCapabilities(
    _env: JNIEnv, _class: JClass,
) -> jlong {
    31
}

#[no_mangle]
pub extern "system" fn Java_com_modernlink_ModernUuid_nativeV7(
    env: JNIEnv, _class: JClass,
) -> jni::sys::jstring {
    match env.new_string(core::uuid_v7()) {
        Ok(value) => value.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_modernlink_ModernUuid_nativeV4(
    env: JNIEnv, _class: JClass,
) -> jni::sys::jstring {
    match env.new_string(core::uuid_v4()) {
        Ok(value) => value.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_modernlink_ModernBase64_nativeEncode(
    env: JNIEnv, _class: JClass, value: JByteArray,
) -> jni::sys::jstring {
    let value = match env.convert_byte_array(&value) {
        Ok(value) => value,
        Err(_) => return std::ptr::null_mut(),
    };
    match env.new_string(core::base64_encode(&value)) {
        Ok(value) => value.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_modernlink_ModernBase64_nativeDecode(
    mut env: JNIEnv, _class: JClass, value: JString,
) -> jni::sys::jbyteArray {
    let value = match env.get_string(&value).ok().and_then(|text| text.to_str().ok().map(str::to_owned)) {
        Some(value) => value,
        None => return std::ptr::null_mut(),
    };
    match core::base64_decode(&value).ok().and_then(|bytes| env.byte_array_from_slice(&bytes).ok()) {
        Some(bytes) => bytes.into_raw(),
        None => std::ptr::null_mut(),
    }
}

fn java_string_array(env: &mut JNIEnv, values: &JObjectArray) -> Option<Vec<String>> {
    let length = env.get_array_length(values).ok()?;
    let mut result = Vec::with_capacity(length as usize);
    for index in 0..length {
        let value = env.get_object_array_element(values, index).ok()?;
        let value = env.get_string(&JString::from(value)).ok()?.to_str().ok()?.to_owned();
        result.push(value);
    }
    Some(result)
}

#[no_mangle]
pub extern "system" fn Java_com_modernlink_ModernJson_nativeObject(
    mut env: JNIEnv, _class: JClass, fields: JObjectArray,
) -> jni::sys::jstring {
    let fields = match java_string_array(&mut env, &fields) {
        Some(value) => value,
        None => return std::ptr::null_mut(),
    };
    match core::json_object(&fields).ok().and_then(|value| env.new_string(value).ok()) {
        Some(value) => value.into_raw(),
        None => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_modernlink_ModernJson_nativeArray(
    mut env: JNIEnv, _class: JClass, values: JObjectArray,
) -> jni::sys::jstring {
    let values = match java_string_array(&mut env, &values) {
        Some(value) => value,
        None => return std::ptr::null_mut(),
    };
    match core::json_array(&values).ok().and_then(|value| env.new_string(value).ok()) {
        Some(value) => value.into_raw(),
        None => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_modernlink_ModernJson_nativeDecode(
    mut env: JNIEnv, _class: JClass, json: JString,
) -> jni::sys::jstring {
    let json = match env.get_string(&json).ok().and_then(|text| text.to_str().ok().map(str::to_owned)) {
        Some(value) => value,
        None => return std::ptr::null_mut(),
    };
    match core::json_decode(&json).ok().and_then(|value| env.new_string(value).ok()) {
        Some(value) => value.into_raw(),
        None => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_modernlink_LegacyHttpClient_nativeStatus(_env: JNIEnv, _class: JClass, handle: jlong) -> jint {
    unsafe { response(handle).map(|value| value.0.status as jint).unwrap_or(0) }
}

#[no_mangle]
pub extern "system" fn Java_com_modernlink_LegacyHttpClient_nativeStatusMessage(
    env: JNIEnv, _class: JClass, handle: jlong,
) -> jni::sys::jstring {
    match unsafe { response(handle).map(|value| value.0.status_message.clone()) }
        .and_then(|value| env.new_string(value).ok()) {
        Some(value) => value.into_raw(),
        None => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_modernlink_LegacyHttpClient_nativeHeaders(
    mut env: JNIEnv, _class: JClass, handle: jlong,
) -> jobjectArray {
    let headers = match unsafe { response(handle) } { Some(value) => &value.0.headers, None => return std::ptr::null_mut() };
    let string_class = match env.find_class("java/lang/String") { Ok(value) => value, Err(_) => return std::ptr::null_mut() };
    let array = match env.new_object_array((headers.len() * 2) as i32, string_class, JString::default()) { Ok(value) => value, Err(_) => return std::ptr::null_mut() };
    for (index, (name, value)) in headers.iter().enumerate() {
        let name = match env.new_string(name) { Ok(value) => value, Err(_) => return std::ptr::null_mut() };
        let value = match env.new_string(value) { Ok(value) => value, Err(_) => return std::ptr::null_mut() };
        if env.set_object_array_element(&array, (index * 2) as i32, name).is_err() || env.set_object_array_element(&array, (index * 2 + 1) as i32, value).is_err() { return std::ptr::null_mut(); }
    }
    array.into_raw()
}

#[no_mangle]
pub extern "system" fn Java_com_modernlink_LegacyHttpClient_nativeBody(env: JNIEnv, _class: JClass, handle: jlong) -> jbyteArray {
    let body = match unsafe { response(handle) } { Some(value) => &value.0.body, None => return std::ptr::null_mut() };
    match env.byte_array_from_slice(body) { Ok(value) => value.into_raw(), Err(_) => std::ptr::null_mut() }
}

#[no_mangle]
pub extern "system" fn Java_com_modernlink_LegacyHttpClient_nativeTlsCertificates(
    mut env: JNIEnv, _class: JClass, handle: jlong,
) -> jobjectArray {
    let certificates = match unsafe { response(handle) } {
        Some(value) => match &value.0.tls { Some(info) => &info.peer_certificates_der, None => return std::ptr::null_mut() },
        None => return std::ptr::null_mut(),
    };
    let byte_array_class = match env.find_class("[B") { Ok(value) => value, Err(_) => return std::ptr::null_mut() };
    let array = match env.new_object_array(certificates.len() as i32, byte_array_class, JByteArray::default()) {
        Ok(value) => value,
        Err(_) => return std::ptr::null_mut(),
    };
    for (index, certificate) in certificates.iter().enumerate() {
        let value = match env.byte_array_from_slice(certificate) { Ok(value) => value, Err(_) => return std::ptr::null_mut() };
        if env.set_object_array_element(&array, index as i32, value).is_err() { return std::ptr::null_mut(); }
    }
    array.into_raw()
}

fn tls_string(handle: jlong, protocol: bool) -> Option<String> {
    unsafe { response(handle).and_then(|value| value.0.tls.as_ref()).and_then(|info| if protocol { info.protocol.clone() } else { info.cipher_suite.clone() }) }
}

#[no_mangle]
pub extern "system" fn Java_com_modernlink_LegacyHttpClient_nativeFinalUrl(
    env: JNIEnv, _class: JClass, handle: jlong,
) -> jni::sys::jstring {
    match unsafe { response(handle).map(|value| value.0.final_url.clone()) }
        .and_then(|value| env.new_string(value).ok()) {
        Some(value) => value.into_raw(),
        None => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_modernlink_LegacyHttpClient_nativeTlsProtocol(
    env: JNIEnv, _class: JClass, handle: jlong,
) -> jni::sys::jstring {
    match tls_string(handle, true).and_then(|value| env.new_string(value).ok()) {
        Some(value) => value.into_raw(),
        None => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_modernlink_LegacyHttpClient_nativeTlsCipherSuite(
    env: JNIEnv, _class: JClass, handle: jlong,
) -> jni::sys::jstring {
    match tls_string(handle, false).and_then(|value| env.new_string(value).ok()) {
        Some(value) => value.into_raw(),
        None => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_modernlink_LegacyHttpClient_nativeRelease(_env: JNIEnv, _class: JClass, handle: jlong) {
    if handle != 0 { unsafe { drop(Box::from_raw(handle as *mut NativeResponse)); } }
}
