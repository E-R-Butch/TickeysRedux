//! Tickeys Redux — arm64 port with objc2 + rodio.

use std::ffi::c_void;

use objc2::rc::{Retained, autoreleasepool};
use objc2::runtime::{AnyObject, ProtocolObject};
use objc2::{ClassType, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy, NSApplicationDelegate};
use objc2_foundation::{MainThreadMarker, NSNotification, NSObject, NSObjectProtocol};

mod cocoa_util;
mod core_foundation;
mod core_graphics;
mod event_tap;
mod iokit;
mod launcher;
mod pref;
mod settings_ui;
mod settings_window;
mod tickeys;

use crate::cocoa_util::*;
use crate::core_graphics::*;
use crate::pref::Pref;
use crate::tickeys::{AudioScheme, Tickeys};

// ── Globals ──────────────────────────────────────────────────────────────────

static mut TICKEYS_PTR: Option<*mut Tickeys> = None;
static mut KEYBOARD_MONITOR: Option<crate::event_tap::KeyboardMonitor> = None;
static mut POWER_MONITOR: Option<iokit::SystemPowerMonitor> = None;
static mut INPUT_MONITORING_ALERT_VISIBLE: bool = false;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum KeyboardMonitorEnsurePlan {
    ReuseEnabled,
    ReenableExisting,
    CreateNew,
    WaitForAlert,
}

fn keyboard_monitor_ensure_plan(
    has_monitor: bool,
    is_enabled: bool,
    alert_visible: bool,
) -> KeyboardMonitorEnsurePlan {
    if has_monitor {
        if is_enabled {
            KeyboardMonitorEnsurePlan::ReuseEnabled
        } else {
            KeyboardMonitorEnsurePlan::ReenableExisting
        }
    } else if alert_visible {
        KeyboardMonitorEnsurePlan::WaitForAlert
    } else {
        KeyboardMonitorEnsurePlan::CreateNew
    }
}

fn should_present_input_monitoring_alert(alert_visible: bool) -> bool {
    !alert_visible
}

unsafe fn set_keyboard_monitor_enabled(enabled: bool) {
    unsafe {
        let slot = &raw mut KEYBOARD_MONITOR;
        if let Some(monitor) = (*slot).as_ref() {
            monitor.set_enabled(enabled);
            if enabled && !monitor.is_enabled() {
                eprintln!("KeyboardMonitor could not be re-enabled");
            }
        }
    }
}

// CGEventTap callback — called on main run loop thread
extern "C" fn handle_keyboard_event(
    _proxy: CGEventTapProxy,
    etype: CGEventType,
    event: CGEventRef,
    _refcon: *mut c_void,
) -> CGEventRef {
    match etype {
        CGEventType::kCGEventTapDisabledByTimeout | CGEventType::kCGEventTapDisabledByUserInput => {
            unsafe { set_keyboard_monitor_enabled(true) };
            return event;
        }
        CGEventType::kCGEventKeyDown => {}
        _ => return event,
    }

    let keycode =
        unsafe { CGEventGetIntegerValueField(event, CGEventField::kCGKeyboardEventKeycode) } as u8;
    unsafe {
        if let Some(ptr) = TICKEYS_PTR {
            let tickeys: &mut Tickeys = &mut *ptr;
            tickeys.handle_keydown(keycode);
        }
    }
    event
}

extern "C" fn handle_power_event(root_port: iokit::io_connect_t, msg: u32, msg_args: *mut c_void) {
    match msg {
        iokit::kIOMessageCanSystemSleep => unsafe {
            let _ = iokit::IOAllowPowerChange(root_port, msg_args as isize);
        },
        iokit::kIOMessageSystemWillSleep => unsafe {
            set_keyboard_monitor_enabled(false);
            let _ = iokit::IOAllowPowerChange(root_port, msg_args as isize);
        },
        iokit::kIOMessageSystemHasPoweredOn => unsafe {
            set_keyboard_monitor_enabled(true);
        },
        _ => {}
    }
}

// ── AppDelegate ──────────────────────────────────────────────────────────────

fn show_input_monitoring_permission_alert() {
    unsafe {
        let alert_visible = &raw mut INPUT_MONITORING_ALERT_VISIBLE;
        if !should_present_input_monitoring_alert(*alert_visible) {
            return;
        }
        *alert_visible = true;

        // LSUIElement apps do not have a Dock icon, so explicitly activate the
        // app to keep this recovery prompt visible even if the menu icon is hidden.
        let app: *mut AnyObject = msg_send![
            objc2::runtime::AnyClass::get(c"NSApplication").unwrap(),
            sharedApplication
        ];
        let _: () = msg_send![app, activateIgnoringOtherApps: true];

        let alert: Retained<AnyObject> = msg_send![
            msg_send![objc2::runtime::AnyClass::get(c"NSAlert").unwrap(), alloc],
            init
        ];
        let _: () = msg_send![&alert, setMessageText: &*l10n_str("permission_title")];
        let _: () = msg_send![&alert, setInformativeText: &*l10n_str("permission_message")];
        let _: () = msg_send![
            &alert,
            addButtonWithTitle: &*l10n_str("open_input_monitoring_settings")
        ];
        let _: () = msg_send![&alert, addButtonWithTitle: &*l10n_str("permission_not_now")];

        let response: isize = msg_send![&alert, runModal];
        *alert_visible = false;
        if response == 1000 {
            let workspace_class = objc2::runtime::AnyClass::get(c"NSWorkspace").unwrap();
            let workspace: *mut AnyObject = msg_send![workspace_class, sharedWorkspace];
            let url_class = objc2::runtime::AnyClass::get(c"NSURL").unwrap();
            let url: *mut AnyObject = msg_send![
                url_class,
                URLWithString: &*nsstr(
                    "x-apple.systempreferences:com.apple.preference.security?Privacy_ListenEvent",
                )
            ];
            let _: bool = msg_send![workspace, openURL: url];
        }
    }
}

fn ensure_keyboard_monitor() -> bool {
    let (has_monitor, is_enabled, alert_visible) = unsafe {
        let monitor_slot = &raw const KEYBOARD_MONITOR;
        let alert_slot = &raw const INPUT_MONITORING_ALERT_VISIBLE;
        match (*monitor_slot).as_ref() {
            Some(monitor) => (true, monitor.is_enabled(), *alert_slot),
            None => (false, false, *alert_slot),
        }
    };

    match keyboard_monitor_ensure_plan(has_monitor, is_enabled, alert_visible) {
        KeyboardMonitorEnsurePlan::ReuseEnabled => return true,
        KeyboardMonitorEnsurePlan::WaitForAlert => return false,
        KeyboardMonitorEnsurePlan::ReenableExisting => {
            let reenabled = unsafe {
                let monitor_slot = &raw mut KEYBOARD_MONITOR;
                if let Some(monitor) = (*monitor_slot).as_ref() {
                    monitor.set_enabled(true);
                    monitor.is_enabled()
                } else {
                    false
                }
            };
            if reenabled {
                println!("KeyboardMonitor re-enabled");
                return true;
            }

            // ensure_keyboard_monitor is called from AppDelegate methods on
            // the main thread, never from the event callback. It is therefore
            // safe to remove the stale tap before constructing a replacement.
            unsafe {
                KEYBOARD_MONITOR = None;
            }
            if alert_visible {
                return false;
            }
        }
        KeyboardMonitorEnsurePlan::CreateNew => {}
    }

    use crate::event_tap::KeyboardMonitor;
    match KeyboardMonitor::new(handle_keyboard_event, std::ptr::null_mut()) {
        Ok(monitor) => {
            unsafe {
                KEYBOARD_MONITOR = Some(monitor);
            }
            println!("KeyboardMonitor started");
            true
        }
        Err(error) => {
            eprintln!("KeyboardMonitor failed: {error}");
            show_input_monitoring_permission_alert();
            false
        }
    }
}

define_class!(
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[derive(Debug)]
    struct AppDelegate;

    unsafe impl NSObjectProtocol for AppDelegate {}

    unsafe impl NSApplicationDelegate for AppDelegate {
        #[unsafe(method(applicationDidFinishLaunching:))]
        fn did_finish_launching(&self, _notification: &NSNotification) {
            let mtm = self.mtm();

            println!("{}", nsstring_to_string(&l10n_str("launching")));

            let (audio_tx, _worker) =
                tickeys::spawn_audio_worker().expect("failed to start audio worker");

            let schemes = Self::load_schemes();
            let pref = Pref::load(&schemes);

            let mut tickeys_box = Box::new(Tickeys::new(schemes, audio_tx));
            tickeys_box.load_scheme(
                &get_res_path(&format!("data/{}", &pref.scheme)),
                &pref.scheme,
            );
            tickeys_box.set_volume(pref.volume / 100.0);
            tickeys_box.set_pitch(pref.pitch);

            let ptr = Box::into_raw(tickeys_box);
            unsafe {
                TICKEYS_PTR = Some(ptr);
            }

            // Create the keyboard monitor on the main thread. If permission is
            // missing, keep running so reopen can retry after the user grants it.
            ensure_keyboard_monitor();

            match iokit::SystemPowerMonitor::new(handle_power_event) {
                Ok(monitor) => unsafe {
                    POWER_MONITOR = Some(monitor);
                },
                Err(error) => eprintln!("Power monitor failed: {error}"),
            }
            settings_ui::setup_menu(mtm, ptr);
            println!("{}", nsstring_to_string(&l10n_str("running")));
        }

        #[unsafe(method(applicationWillTerminate:))]
        fn will_terminate(&self, _notification: &NSNotification) {
            unsafe {
                POWER_MONITOR = None;
                // Drop keyboard monitor first (disables event tap)
                KEYBOARD_MONITOR = None;
                let ptr = std::ptr::replace(&raw mut TICKEYS_PTR, None);
                if let Some(p) = ptr {
                    drop(Box::from_raw(p));
                }
            }
        }

        #[unsafe(method(applicationShouldHandleReopen:hasVisibleWindows:))]
        fn should_handle_reopen(&self, _app: &AnyObject, _flag: bool) -> bool {
            // Reopening the already-running app is the recovery action after
            // granting Input Monitoring permission. This is idempotent.
            ensure_keyboard_monitor();
            unsafe {
                if let Some(ptr) = TICKEYS_PTR {
                    settings_window::show_prefs_window(self.mtm(), ptr);
                }
            }
            true
        }
    }
);

impl AppDelegate {
    fn load_schemes() -> Vec<AudioScheme> {
        let path = get_res_path("data/schemes.json");
        let mut file = std::fs::File::open(&path)
            .unwrap_or_else(|e| panic!("schemes.json not found at {}: {}", path, e));
        let mut json_str = String::with_capacity(512);
        std::io::Read::read_to_string(&mut file, &mut json_str)
            .expect("Failed to read schemes.json");
        serde_json::from_str(&json_str).expect("Failed to parse schemes.json")
    }
}

// ── main ─────────────────────────────────────────────────────────────────────

fn main() {
    let mtm = MainThreadMarker::new().expect("must be on main thread");

    autoreleasepool(|_pool| {
        let delegate: Retained<AppDelegate> = unsafe { msg_send![AppDelegate::class(), new] };

        let app = NSApplication::sharedApplication(mtm);
        app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);
        app.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));
        app.run();
    });
}

#[cfg(test)]
mod tests {
    use super::{
        KeyboardMonitorEnsurePlan, keyboard_monitor_ensure_plan,
        should_present_input_monitoring_alert,
    };

    const EN_LOCALIZATION: &str = include_str!("../assets/lproj/Base.lproj/Localizable.strings");
    const ZH_LOCALIZATION: &str = include_str!("../assets/lproj/zh-Hans.lproj/Localizable.strings");

    #[test]
    fn keyboard_monitor_recovery_avoids_duplicate_work() {
        assert_eq!(
            keyboard_monitor_ensure_plan(true, true, false),
            KeyboardMonitorEnsurePlan::ReuseEnabled
        );
        assert_eq!(
            keyboard_monitor_ensure_plan(true, false, false),
            KeyboardMonitorEnsurePlan::ReenableExisting
        );
        assert_eq!(
            keyboard_monitor_ensure_plan(false, false, false),
            KeyboardMonitorEnsurePlan::CreateNew
        );
        assert_eq!(
            keyboard_monitor_ensure_plan(false, false, true),
            KeyboardMonitorEnsurePlan::WaitForAlert
        );
        assert_eq!(
            keyboard_monitor_ensure_plan(true, false, true),
            KeyboardMonitorEnsurePlan::ReenableExisting
        );

        assert!(should_present_input_monitoring_alert(false));
        assert!(!should_present_input_monitoring_alert(true));
    }

    #[test]
    fn input_monitoring_recovery_copy_is_complete_in_both_localizations() {
        for localization in [EN_LOCALIZATION, ZH_LOCALIZATION] {
            for key in [
                "permission_title",
                "permission_message",
                "open_input_monitoring_settings",
                "permission_not_now",
            ] {
                assert!(
                    localization.contains(&format!("\"{key}\" =")),
                    "missing localization key {key}"
                );
            }
        }

        assert!(EN_LOCALIZATION.contains("remove the old Tickeys Redux entry"));
        assert!(ZH_LOCALIZATION.contains("移除旧的 Tickeys Redux 条目"));
        assert!(EN_LOCALIZATION.contains("open Tickeys Redux again"));
        assert!(ZH_LOCALIZATION.contains("再次打开 Tickeys Redux"));
        assert!(EN_LOCALIZATION.contains("quit Tickeys Redux completely"));
        assert!(ZH_LOCALIZATION.contains("完全退出 Tickeys Redux"));
    }
}
