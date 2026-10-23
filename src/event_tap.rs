use crate::core_foundation::*;
use crate::core_graphics::*;
use objc2_foundation::MainThreadMarker;
use std::ffi::c_void;

pub struct KeyboardMonitor {
    event_tap: CFMachPortRef,
    runloop_source: CFRunLoopSourceRef,
    runloop: CFRunLoopRef,
    _main_thread: MainThreadMarker,
}

impl KeyboardMonitor {
    pub fn new(
        mtm: MainThreadMarker,
        handler: CGEventTapCallBack,
        user_data: *mut c_void,
    ) -> Result<KeyboardMonitor, String> {
        unsafe {
            // Recovery checks are passive. Requesting permission is a separate,
            // once-per-launch operation, never part of an event callback.
            if !CGPreflightListenEventAccess() {
                return Err("Input Monitoring permission is not granted".to_string());
            }

            let event_tap = CGEventTapCreate(
                CGEventTapLocation::kCGSessionEventTap,
                CGEventTapPlacement::kCGHeadInsertEventTap,
                CGEventTapOptions::kCGEventTapOptionListenOnly,
                CGEventMaskBit!(CGEventType::kCGEventKeyDown),
                handler,
                user_data,
            );

            if event_tap.is_null() {
                return Err("failed to CGEventTapCreate".to_string());
            }

            let run_loop_source = CFMachPortCreateRunLoopSource(kCFAllocatorDefault, event_tap, 0);
            if run_loop_source.is_null() {
                CFMachPortInvalidate(event_tap);
                CFRelease(event_tap);
                return Err("failed to CFMachPortCreateRunLoopSource".to_string());
            }

            let runloop = CFRunLoopGetCurrent();
            CFRunLoopAddSource(runloop, run_loop_source, kCFRunLoopCommonModes);
            CGEventTapEnable(event_tap, true);

            if !CGEventTapIsEnabled(event_tap) {
                CFRunLoopRemoveSource(runloop, run_loop_source, kCFRunLoopCommonModes);
                CFMachPortInvalidate(event_tap);
                CFRelease(run_loop_source);
                CFRelease(event_tap);
                return Err("CGEventTap was created but is disabled".to_string());
            }

            Ok(KeyboardMonitor {
                event_tap,
                runloop_source: run_loop_source,
                runloop,
                _main_thread: mtm,
            })
        }
    }

    pub fn set_enabled(&self, enabled: bool) {
        unsafe { CGEventTapEnable(self.event_tap, enabled) };
    }

    pub fn is_enabled(&self) -> bool {
        unsafe { CGEventTapIsEnabled(self.event_tap) }
    }
}

impl crate::input_recovery::Monitor for KeyboardMonitor {
    fn is_enabled(&self) -> bool {
        self.is_enabled()
    }

    fn set_enabled(&self, enabled: bool) {
        self.set_enabled(enabled);
    }
}

impl Drop for KeyboardMonitor {
    fn drop(&mut self) {
        self.set_enabled(false);
        unsafe {
            CFRunLoopRemoveSource(self.runloop, self.runloop_source, kCFRunLoopCommonModes);
            CFMachPortInvalidate(self.event_tap);
            CFRelease(self.runloop_source);
            CFRelease(self.event_tap);
        }
    }
}
