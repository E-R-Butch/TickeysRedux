//! Inlined IOKit FFI declarations — replaces the deprecated iokit-sys crate.
//! Only the symbols needed for system power monitoring are included.

#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
#![allow(dead_code)]

use std::ffi::c_void;

use crate::core_foundation::{
    CFRunLoopAddSource, CFRunLoopGetCurrent, CFRunLoopRemoveSource, CFRunLoopSourceRef,
    kCFRunLoopCommonModes,
};

// ── Types ────────────────────────────────────────────────────────────────────

pub type io_service_t = u32;
pub type io_object_t = u32;
pub type IONotificationPortRef = *mut c_void;
pub type IOReturn = i32;
pub type io_connect_t = u32;

// ── Constants ────────────────────────────────────────────────────────────────

const fn iokit_common_message(message: u32) -> u32 {
    0xe000_0000 | message
}

pub const kIOMessageCanSystemSleep: u32 = iokit_common_message(0x270);
pub const kIOMessageSystemWillSleep: u32 = iokit_common_message(0x280);
pub const kIOMessageSystemHasPoweredOn: u32 = iokit_common_message(0x300);

// ── Functions ────────────────────────────────────────────────────────────────

#[link(name = "IOKit", kind = "framework")]
unsafe extern "system" {
    pub fn IORegisterForSystemPower(
        refcon: *mut c_void,
        notificationPort: *mut IONotificationPortRef,
        callback: extern "C" fn(*mut c_void, io_service_t, u32, *mut c_void),
        notifier: *mut io_object_t,
    ) -> io_connect_t;

    pub fn IONotificationPortGetRunLoopSource(notify: IONotificationPortRef) -> *mut c_void; // CFRunLoopSourceRef

    pub fn IODeregisterForSystemPower(notifier: *mut io_object_t) -> IOReturn;
    pub fn IONotificationPortDestroy(notify: IONotificationPortRef);
    pub fn IOAllowPowerChange(connect: io_connect_t, notificationID: isize) -> IOReturn;
    pub fn IOServiceClose(connect: io_connect_t) -> IOReturn;
}

pub struct SystemPowerMonitor {
    root_port: io_connect_t,
    notification_port: IONotificationPortRef,
    notifier: io_object_t,
    run_loop_source: CFRunLoopSourceRef,
    _callback_context: Box<PowerCallbackContext>,
}

type PowerCallback = extern "C" fn(io_connect_t, u32, *mut c_void);

struct PowerCallbackContext {
    root_port: io_connect_t,
    callback: PowerCallback,
}

extern "C" fn dispatch_power_event(
    refcon: *mut c_void,
    _service: io_service_t,
    message: u32,
    message_argument: *mut c_void,
) {
    if refcon.is_null() {
        return;
    }
    unsafe {
        let context = &*(refcon as *const PowerCallbackContext);
        (context.callback)(context.root_port, message, message_argument);
    }
}

impl SystemPowerMonitor {
    pub fn new(callback: PowerCallback) -> Result<Self, String> {
        unsafe {
            let mut notification_port: IONotificationPortRef = std::ptr::null_mut();
            let mut notifier: io_object_t = 0;
            let mut callback_context = Box::new(PowerCallbackContext {
                root_port: 0,
                callback,
            });
            let root_port = IORegisterForSystemPower(
                (&raw mut *callback_context).cast(),
                &mut notification_port,
                dispatch_power_event,
                &mut notifier,
            );
            if root_port == 0 {
                return Err("IORegisterForSystemPower failed".into());
            }

            let run_loop_source =
                IONotificationPortGetRunLoopSource(notification_port) as CFRunLoopSourceRef;
            if run_loop_source.is_null() {
                let _ = IODeregisterForSystemPower(&mut notifier);
                IONotificationPortDestroy(notification_port);
                let _ = IOServiceClose(root_port);
                return Err("IONotificationPortGetRunLoopSource returned null".into());
            }

            // The callback is dispatched only after this source is added, so
            // publish the returned connection before enabling delivery.
            callback_context.root_port = root_port;
            CFRunLoopAddSource(
                CFRunLoopGetCurrent(),
                run_loop_source,
                kCFRunLoopCommonModes,
            );

            Ok(Self {
                root_port,
                notification_port,
                notifier,
                run_loop_source,
                _callback_context: callback_context,
            })
        }
    }
}

impl Drop for SystemPowerMonitor {
    fn drop(&mut self) {
        unsafe {
            CFRunLoopRemoveSource(
                CFRunLoopGetCurrent(),
                self.run_loop_source,
                kCFRunLoopCommonModes,
            );
            let _ = IODeregisterForSystemPower(&mut self.notifier);
            IONotificationPortDestroy(self.notification_port);
            let _ = IOServiceClose(self.root_port);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        kIOMessageCanSystemSleep, kIOMessageSystemHasPoweredOn, kIOMessageSystemWillSleep,
    };

    #[test]
    fn power_message_constants_match_the_macos_sdk() {
        assert_eq!(kIOMessageCanSystemSleep, 0xe000_0270);
        assert_eq!(kIOMessageSystemWillSleep, 0xe000_0280);
        assert_eq!(kIOMessageSystemHasPoweredOn, 0xe000_0300);
    }
}
