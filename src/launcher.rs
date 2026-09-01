//! Login item management via SMAppService (macOS 13+).

use std::ffi::CStr;

use objc2::msg_send;
use objc2::runtime::{AnyClass, AnyObject};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LoginItemStatus {
    NotRegistered,
    Enabled,
    RequiresApproval,
    NotFound,
    Unknown(isize),
}

impl LoginItemStatus {
    pub fn is_registered(self) -> bool {
        matches!(self, Self::Enabled | Self::RequiresApproval)
    }
}

/// Read the complete SMAppService state instead of collapsing it to a bool.
pub fn start_at_login_status() -> Result<LoginItemStatus, String> {
    let service = main_app_service()?;
    let raw_status: isize = unsafe { msg_send![service, status] };
    Ok(match raw_status {
        0 => LoginItemStatus::NotRegistered,
        1 => LoginItemStatus::Enabled,
        2 => LoginItemStatus::RequiresApproval,
        3 => LoginItemStatus::NotFound,
        other => LoginItemStatus::Unknown(other),
    })
}

/// Enable or disable Start at Login and return the final system state.
pub fn set_start_at_login(enabled: bool) -> Result<LoginItemStatus, String> {
    let before = start_at_login_status()?;

    // Re-registering an already registered service is an error. Matching states
    // are therefore handled as idempotent operations.
    if enabled && before.is_registered() {
        return Ok(before);
    }
    if !enabled && before == LoginItemStatus::NotRegistered {
        return Ok(before);
    }

    let service = main_app_service()?;
    let mut error: *mut AnyObject = std::ptr::null_mut();
    let success: bool = unsafe {
        if enabled {
            msg_send![service, registerAndReturnError: &raw mut error]
        } else {
            msg_send![service, unregisterAndReturnError: &raw mut error]
        }
    };

    if !success {
        return Err(error_description(
            error,
            if enabled { "register" } else { "unregister" },
        ));
    }

    let after = start_at_login_status()?;
    let reached_expected_state = if enabled {
        after.is_registered()
    } else {
        after == LoginItemStatus::NotRegistered
    };
    if reached_expected_state {
        Ok(after)
    } else {
        Err(format!(
            "SMAppService reported success but ended in unexpected state {after:?}"
        ))
    }
}

/// Open System Settings at General > Login Items.
pub fn open_login_items_settings() -> Result<(), String> {
    let cls = get_sm_app_service_class()?;
    unsafe {
        let _: () = msg_send![cls, openSystemSettingsLoginItems];
    }
    Ok(())
}

fn main_app_service() -> Result<*mut AnyObject, String> {
    let cls = get_sm_app_service_class()?;
    let service: *mut AnyObject = unsafe { msg_send![cls, mainAppService] };
    if service.is_null() {
        Err("SMAppService.mainAppService returned nil".into())
    } else {
        Ok(service)
    }
}

fn get_sm_app_service_class() -> Result<&'static AnyClass, String> {
    AnyClass::get(c"SMAppService")
        .ok_or_else(|| "SMAppService is unavailable (macOS 13 or later is required)".into())
}

fn error_description(error: *mut AnyObject, operation: &str) -> String {
    if error.is_null() {
        return format!("Failed to {operation} the login item (no NSError was returned)");
    }

    unsafe {
        let description: *mut AnyObject = msg_send![error, localizedDescription];
        if description.is_null() {
            return format!("Failed to {operation} the login item");
        }
        let utf8: *const std::ffi::c_char = msg_send![description, UTF8String];
        if utf8.is_null() {
            format!("Failed to {operation} the login item")
        } else {
            CStr::from_ptr(utf8).to_string_lossy().into_owned()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::LoginItemStatus;

    #[test]
    fn enabled_and_requires_approval_are_registered_states() {
        assert!(LoginItemStatus::Enabled.is_registered());
        assert!(LoginItemStatus::RequiresApproval.is_registered());
        assert!(!LoginItemStatus::NotRegistered.is_registered());
        assert!(!LoginItemStatus::NotFound.is_registered());
    }
}
