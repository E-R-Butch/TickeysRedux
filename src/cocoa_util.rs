//! Cocoa utility functions — objc2 migration.

use std::env;
use std::path::PathBuf;

use objc2::rc::Retained;
use objc2_foundation::{NSBundle, NSString};

/// Get localized string from main bundle.
pub fn l10n_str(key: &str) -> Retained<NSString> {
    let bundle = NSBundle::mainBundle();
    let key_ns = NSString::from_str(key);
    bundle.localizedStringForKey_value_table(&key_ns, None, None)
}

/// Convert NSString to Rust String.
pub fn nsstring_to_string(ns: &NSString) -> String {
    ns.to_string()
}

/// Create an NSString.
pub fn nsstr(s: &str) -> Retained<NSString> {
    NSString::from_str(s)
}

/// Get resource path relative to the app bundle.
pub fn get_res_path(sub_path: &str) -> String {
    let exe = env::current_exe().unwrap();
    let mut path = PathBuf::from(exe.parent().unwrap());
    path.push("../Resources");
    path.push(sub_path);
    path.to_string_lossy().to_string()
}
