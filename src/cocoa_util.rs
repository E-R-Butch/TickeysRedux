//! Cocoa utility functions — objc2 migration.

use std::env;
use std::ffi::CStr;
use std::path::PathBuf;
use std::process;

use objc2::msg_send;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2_foundation::{NSBundle, NSString};

/// Get localized string from main bundle.
pub fn l10n_str(key: &str) -> Retained<NSString> {
    let bundle = unsafe { NSBundle::mainBundle() };
    let key_ns = NSString::from_str(key);
    unsafe {
        bundle.localizedStringForKey_value_table(
            &key_ns,
            None,
            None,
        )
    }
}

/// Convert NSString to Rust String.
pub fn nsstring_to_string(ns: &NSString) -> String {
    ns.to_string()
}

/// Create an NSString.
pub fn nsstr(s: &str) -> Retained<NSString> {
    NSString::from_str(s)
}

/// Extract filename from NSURL.
pub fn nsurl_filename(nsurl: &AnyObject) -> Retained<NSString> {
    unsafe {
        let path_components: Retained<AnyObject> = msg_send![nsurl, pathComponents];
        msg_send![&path_components, lastObject]
    }
}

/// Get resource path relative to the app bundle.
pub fn get_res_path(sub_path: &str) -> String {
    let exe = env::current_exe().unwrap();
    let mut path = PathBuf::from(exe.parent().unwrap());
    path.push("../Resources");
    path.push(sub_path);
    path.to_string_lossy().to_string()
}

/// Terminate the application.
pub fn app_terminate() {
    unsafe {
        let cls = get_class("NSApplication");
        let app: Retained<AnyObject> = msg_send![cls, sharedApplication];
        let _: () = msg_send![&app, terminate: std::ptr::null::<AnyObject>()];
    }
}

/// Relaunch self (used after Accessibility permission grant).
pub fn app_relaunch_self() {
    unsafe {
        let cls = get_class("NSBundle");
        let bundle: Retained<AnyObject> = msg_send![cls, mainBundle];
        let path: Retained<AnyObject> = msg_send![&bundle, executablePath];

        let pi_cls = get_class("NSProcessInfo");
        let proc_info: Retained<AnyObject> = msg_send![pi_cls, processInfo];
        let proc_id: i32 = msg_send![&proc_info, processIdentifier];
        let pid_str = NSString::from_str(&format!("{}", proc_id));

        let arr_cls = get_class("NSMutableArray");
        let args: Retained<AnyObject> = msg_send![arr_cls, new];
        let _: () = msg_send![&args, addObject: &*path];
        let _: () = msg_send![&args, addObject: &*pid_str];

        let task_cls = get_class("NSTask");
        let _: Retained<AnyObject> = msg_send![
            task_cls,
            launchedTaskWithLaunchPath: &*path,
            arguments: &*args
        ];
    }
    process::exit(0);
}

/// Helper: get class by name via CStr.
fn get_class(name: &str) -> &'static objc2::runtime::AnyClass {
    let s = format!("{}\0", name);
    let cname = CStr::from_bytes_with_nul(s.as_bytes()).unwrap();
    objc2::runtime::AnyClass::get(cname).unwrap()
}
