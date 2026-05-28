//! CoreFoundation FFI bindings — minimal set for Tickeys.
#![allow(non_camel_case_types)]
#![allow(dead_code)]

use std::ffi::c_void;

pub type CFTypeRef = *const c_void;
pub type CFMachPortRef = *mut c_void;
pub type CFAllocatorRef = *mut c_void;
pub type CFRunLoopSourceRef = *mut c_void;
pub type CFRunLoopRef = *mut c_void;
pub type CFStringRef = *mut c_void;
pub type CFBooleanRef = *mut c_void;
pub type CFIndex = i64;
pub type CFDictionaryRef = *mut c_void;

#[repr(C)]
pub struct CFDictionaryKeyCallBacks {
    pub version: CFIndex,
    pub retain: Option<extern "C" fn(CFAllocatorRef, *const c_void) -> *const c_void>,
    pub release: Option<extern "C" fn(CFAllocatorRef, *const c_void)>,
    pub copyDescription: Option<extern "C" fn(*const c_void) -> CFStringRef>,
    pub equal: Option<extern "C" fn(*const c_void, *const c_void) -> bool>,
    pub hash: Option<extern "C" fn(*const c_void) -> CFIndex>,
}
pub type CFDictionaryValueCallBacks = CFDictionaryKeyCallBacks;

pub type CFMessagePortRef = *mut c_void;
pub type CFDataRef = *mut c_void;
pub type CFMessagePortCallBack = extern "C" fn(
    local: CFMessagePortRef,
    msgid: i32,
    data: CFDataRef,
    info: *mut c_void,
) -> CFDataRef;

#[repr(C)]
struct CFMessagePortContext {
    version: CFIndex,
    info: *mut c_void,
    retain: Option<extern "C" fn(*const c_void) -> *const c_void>,
    release: Option<extern "C" fn(*const c_void)>,
    copyDescription: Option<extern "C" fn(*const c_void) -> CFStringRef>,
}

extern "C" {
    pub static kCFRunLoopDefaultMode: CFStringRef;
    pub static kCFRunLoopCommonModes: CFStringRef;
    pub static kCFAllocatorDefault: CFAllocatorRef;

    pub static kCFBooleanTrue: CFBooleanRef;
    pub static kCFBooleanFalse: CFBooleanRef;
    pub static kAXTrustedCheckOptionPrompt: CFStringRef;

    pub static kCFTypeDictionaryKeyCallBacks: CFDictionaryKeyCallBacks;
    pub static kCFTypeDictionaryValueCallBacks: CFDictionaryValueCallBacks;
}

#[link(name = "CoreFoundation", kind = "framework")]
extern "system" {
    pub fn CFRelease(cf: CFTypeRef);

    pub fn CFMachPortCreateRunLoopSource(
        allocator: CFAllocatorRef,
        port: CFMachPortRef,
        order: CFIndex,
    ) -> CFRunLoopSourceRef;

    pub fn CFRunLoopAddSource(rl: CFRunLoopRef, source: CFRunLoopSourceRef, mode: CFStringRef);
    pub fn CFRunLoopRemoveSource(rl: CFRunLoopRef, source: CFRunLoopSourceRef, mode: CFStringRef);
    pub fn CFRunLoopGetCurrent() -> CFRunLoopRef;
    pub fn CFRunLoopRun();
    pub fn CFRunLoopStop(rl: CFRunLoopRef);

    // CFRunLoopPerformBlock — accepts an Objective-C block pointer.
    // In objc2, use Retained<Block<dyn Fn()>> and cast via as_ptr().
    pub fn CFRunLoopPerformBlock(
        rl: CFRunLoopRef,
        mode: CFTypeRef,
        block: *const c_void,
    );

    pub fn CFMessagePortCreateLocal(
        allocator: CFAllocatorRef,
        name: CFStringRef,
        callout: CFMessagePortCallBack,
        context: *mut CFMessagePortContext,
        shouldFreeInfo: bool,
    ) -> CFMessagePortRef;

    pub fn CFMessagePortCreateRunLoopSource(
        allocator: CFAllocatorRef,
        local: CFMessagePortRef,
        order: CFIndex,
    ) -> CFRunLoopSourceRef;

    pub fn CFMessagePortSendRequest(
        remote: CFMessagePortRef,
        msgid: i32,
        data: CFDataRef,
        sendTimeout: f64,
        rcvTimeout: f64,
        replyMode: CFStringRef,
        returnData: *mut CFDataRef,
    ) -> i32;

    pub fn CFMessagePortInvalidate(ms: CFMessagePortRef);

    pub fn CFDictionaryCreate(
        allocator: CFAllocatorRef,
        keys: *const *const c_void,
        values: *const *const c_void,
        numValues: CFIndex,
        keyCallBacks: *const CFDictionaryKeyCallBacks,
        valueCallBacks: *const CFDictionaryValueCallBacks,
    ) -> CFDictionaryRef;
}
