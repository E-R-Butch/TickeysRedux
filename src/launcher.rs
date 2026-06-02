//! Login item management via SMAppService (macOS 13+).
//!
//! Uses ServiceManagement.framework's SMAppService to register/unregister
//! the app as a login item. Falls back gracefully if framework not available.

use std::ffi::CStr;

use objc2::msg_send;
use objc2::runtime::AnyObject;

/// Check if the app is registered as a login item.
/// Returns false if SMAppService is not available.
pub fn is_start_at_login_enabled() -> bool {
    let cls = match get_sm_app_service_class() {
        Some(c) => c,
        None => return false,
    };
    unsafe {
        let service: *mut AnyObject = msg_send![cls, mainAppService];
        let status: isize = msg_send![service, status];
        status == 1 // SMAppServiceStatusEnabled
    }
}

/// Enable or disable "Start at login".
/// Returns Ok(true) on success, Err(msg) on failure.
pub fn set_start_at_login(enabled: bool) -> Result<bool, String> {
    let cls = match get_sm_app_service_class() {
        Some(c) => c,
        None => return Err("SMAppService not available".into()),
    };
    unsafe {
        let service: *mut AnyObject = msg_send![cls, mainAppService];
        if enabled {
            let error: *mut AnyObject = std::ptr::null_mut();
            let success: bool = msg_send![service, registerAndReturnError: &raw const error];
            if !success && !error.is_null() {
                let desc: *mut AnyObject = msg_send![error, localizedDescription];
                let cstr = CStr::from_ptr(msg_send![desc, UTF8String]);
                return Err(cstr.to_string_lossy().to_string());
            }
            Ok(success)
        } else {
            let error: *mut AnyObject = std::ptr::null_mut();
            let _: () = msg_send![service, unregisterAndReturnError: &raw const error];
            Ok(true)
        }
    }
}

/// Get the SMAppService class. Returns None if unavailable.
fn get_sm_app_service_class() -> Option<&'static objc2::runtime::AnyClass> {
    let name = CStr::from_bytes_with_nul(b"SMAppService\0").unwrap();
    objc2::runtime::AnyClass::get(name)
}
