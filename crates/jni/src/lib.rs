use jni::objects::{JByteArray, JClass, JObjectArray, JString};
use jni::sys::{jbyteArray, jint, jlong, jobjectArray};
use jni::JNIEnv;
use core::{Request, Response};
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
    let result = match http::execute(&request) { Ok(value) => value, Err(error) => return set_error(error.to_string()) };
    Box::into_raw(Box::new(NativeResponse(result))) as jlong
}

#[no_mangle]
pub extern "system" fn Java_com_modernlink_LegacyHttpClient_nativeLastError(env: JNIEnv, _class: JClass) -> jni::sys::jstring {
    let message = LAST_ERROR.with(|value| value.borrow().clone());
    match env.new_string(message) { Ok(value) => value.into_raw(), Err(_) => std::ptr::null_mut() }
}

#[no_mangle]
pub extern "system" fn Java_com_modernlink_LegacyHttpClient_nativeStatus(_env: JNIEnv, _class: JClass, handle: jlong) -> jint {
    unsafe { response(handle).map(|value| value.0.status as jint).unwrap_or(0) }
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
pub extern "system" fn Java_com_modernlink_LegacyHttpClient_nativeRelease(_env: JNIEnv, _class: JClass, handle: jlong) {
    if handle != 0 { unsafe { drop(Box::from_raw(handle as *mut NativeResponse)); } }
}
