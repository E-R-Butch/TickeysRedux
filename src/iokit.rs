//! Inlined IOKit FFI declarations — replaces the deprecated iokit-sys crate.
//! Only the symbols needed for system power monitoring are included.

#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
#![allow(dead_code)]

use std::ffi::c_void;

// ── Types ────────────────────────────────────────────────────────────────────

pub type io_service_t = u32;
pub type io_object_t = u32;
pub type IONotificationPortRef = *mut c_void;
pub type IOReturn = i32;
pub type io_connect_t = u32;

// ── Constants ────────────────────────────────────────────────────────────────

pub const kIOMessageSystemHasPoweredOn: u32 = 0x3000000;

// ── Functions ────────────────────────────────────────────────────────────────

#[link(name = "IOKit", kind = "framework")]
extern "system" {
    pub fn IORegisterForSystemPower(
        refcon: *mut c_void,
        notificationPort: *mut IONotificationPortRef,
        callback: extern "C" fn(*mut c_void, io_service_t, u32, *mut c_void),
        notifier: *mut io_object_t,
    ) -> IOReturn;

    pub fn IONotificationPortGetRunLoopSource(
        notify: IONotificationPortRef,
    ) -> *mut c_void; // CFRunLoopSourceRef

    pub fn IODeregisterForSystemPower(notifier: *mut io_object_t);
    pub fn IONotificationPortDestroy(notify: IONotificationPortRef);
}
