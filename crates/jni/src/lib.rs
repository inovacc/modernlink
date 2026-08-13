use jni::objects::{JClass, JString};
use jni::sys::{jbyteArray, jint, jlong, jobjectArray};
use jni::JNIEnv;
use core::{Request, Response};

#[cfg(target_os = "linux")]
#[no_mangle]
pub extern "C" fn getauxval(_kind: usize) -> usize {
    0
}

struct NativeResponse(Response);

unsafe fn response<'a>(handle: jlong) -> Option<&'a NativeResponse> {
    if handle == 0 { None } else { Some(&*(handle as *const NativeResponse)) }
}

#[no_mangle]
pub extern "system" fn Java_com_modernlink_LegacyHttpClient_nativeGet(
    mut env: JNIEnv,
    _class: JClass,
    url: JString,
) -> jlong {
    let url = match env.get_string(&url).ok().and_then(|value| value.to_str().ok().map(str::to_owned)) {
        Some(url) => url,
        None => return 0,
    };
    let request = match Request::new(&url) {
        Ok(request) => request,
        Err(_) => return 0,
    };
    let result = match http::execute(&request) {
        Ok(response) => response,
        Err(_) => return 0,
    };
    Box::into_raw(Box::new(NativeResponse(result))) as jlong
}

#[no_mangle]
pub extern "system" fn Java_com_modernlink_LegacyHttpClient_nativeStatus(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
) -> jint {
    unsafe { response(handle).map(|value| value.0.status as jint).unwrap_or(0) }
}

#[no_mangle]
pub extern "system" fn Java_com_modernlink_LegacyHttpClient_nativeHeaders(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
) -> jobjectArray {
    let headers = match unsafe { response(handle) } {
        Some(value) => &value.0.headers,
        None => return std::ptr::null_mut(),
    };
    let string_class = match env.find_class("java/lang/String") {
        Ok(value) => value,
        Err(_) => return std::ptr::null_mut(),
    };
    let array = match env.new_object_array((headers.len() * 2) as i32, string_class, JString::default()) {
        Ok(value) => value,
        Err(_) => return std::ptr::null_mut(),
    };
    for (index, (name, value)) in headers.iter().enumerate() {
        let name = match env.new_string(name) { Ok(value) => value, Err(_) => return std::ptr::null_mut() };
        let value = match env.new_string(value) { Ok(value) => value, Err(_) => return std::ptr::null_mut() };
        if env.set_object_array_element(&array, (index * 2) as i32, name).is_err()
            || env.set_object_array_element(&array, (index * 2 + 1) as i32, value).is_err() {
            return std::ptr::null_mut();
        }
    }
    array.into_raw()
}

#[no_mangle]
pub extern "system" fn Java_com_modernlink_LegacyHttpClient_nativeBody(
    env: JNIEnv,
    _class: JClass,
    handle: jlong,
) -> jbyteArray {
    let body = match unsafe { response(handle) } {
        Some(value) => &value.0.body,
        None => return std::ptr::null_mut(),
    };
    match env.byte_array_from_slice(body) {
        Ok(value) => value.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_modernlink_LegacyHttpClient_nativeRelease(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
) {
    if handle != 0 {
        unsafe { drop(Box::from_raw(handle as *mut NativeResponse)); }
    }
}
