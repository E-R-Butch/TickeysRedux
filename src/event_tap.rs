use std::ffi::c_void;
use crate::core_graphics::*;
use crate::core_foundation::*;

pub struct KeyboardMonitor {
    event_tap: CFMachPortRef,
    runloop_source: CFRunLoopSourceRef,
}

impl KeyboardMonitor {

    pub fn new(handler: CGEventTapCallBack, user_data: *mut c_void) -> Result<KeyboardMonitor, String> {
        unsafe {
            if !CGPreflightListenEventAccess() && !CGRequestListenEventAccess() {
                return Err("Input Monitoring permission is not granted".to_string());
            }

            let event_tap = CGEventTapCreate(
                CGEventTapLocation::kCGHIDEventTap,
                CGEventTapPlacement::kCGHeadInsertEventTap,
                CGEventTapOptions::kCGEventTapOptionListenOnly,
                CGEventMaskBit!(CGEventType::kCGEventKeyDown),
                handler,
                user_data,
            );

            if event_tap == (0 as CFMachPortRef) {
                return Err("failed to CGEventTapCreate".to_string());
            }

            let run_loop_source = CFMachPortCreateRunLoopSource(kCFAllocatorDefault, event_tap, 0);
            if run_loop_source == (0 as CFRunLoopSourceRef) {
                CFRelease(event_tap);
                return Err("failed to CFMachPortCreateRunLoopSource".to_string());
            }

            CFRunLoopAddSource(CFRunLoopGetCurrent(), run_loop_source, kCFRunLoopCommonModes);
            CGEventTapEnable(event_tap, true);

            if !CGEventTapIsEnabled(event_tap) {
                CFRunLoopRemoveSource(CFRunLoopGetCurrent(), run_loop_source, kCFRunLoopCommonModes);
                CFRelease(event_tap);
                CFRelease(run_loop_source);
                return Err("CGEventTap was created but is disabled".to_string());
            }

            Ok(KeyboardMonitor { event_tap, runloop_source: run_loop_source })
        }
    }

    #[allow(dead_code)]
    pub fn set_enabled(&mut self, enabled: bool) {
        unsafe { CGEventTapEnable(self.event_tap, enabled) };
    }

    #[allow(dead_code)]
    pub fn is_enabled(&mut self) -> bool {
        unsafe { CGEventTapIsEnabled(self.event_tap) }
    }
}

impl Drop for KeyboardMonitor {
    fn drop(&mut self) {
        self.set_enabled(false);
        unsafe {
            CFRunLoopRemoveSource(CFRunLoopGetCurrent(), self.runloop_source, kCFRunLoopCommonModes);
            CFRelease(self.event_tap);
            CFRelease(self.runloop_source);
        }
    }
}
