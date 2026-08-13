use base64::Engine;
use jni::objects::{JClass, JString};
use jni::sys::jstring;
use jni::JNIEnv;
use core::Request;

#[cfg(target_os = "linux")]
#[no_mangle]
pub extern "C" fn getauxval(_kind: usize) -> usize {
    0
}

#[no_mangle]
pub extern "system" fn Java_com_modernlink_LegacyHttpClient_nativeGet(
    mut env: JNIEnv,
    _class: JClass,
    url: JString,
) -> jstring {
    let result = (|| {
        let url = env.get_string(&url).ok()?.to_str().ok()?.to_owned();
        let request = Request::new(&url).ok()?;
        let response = http::execute(&request).ok()?;
        let body = base64::engine::general_purpose::STANDARD.encode(response.body);
        let mut payload = format!("{}\n", response.status);
        for (name, value) in response.headers {
            payload.push_str(&name);
            payload.push('=');
            payload.push_str(&value);
            payload.push('\n');
        }
        payload.push('\n');
        payload.push_str(&body);
        env.new_string(payload).ok().map(|value| value.into_raw())
    })();
    result.unwrap_or(std::ptr::null_mut())
}
