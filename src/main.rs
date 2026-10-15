//! Tickeys Redux — arm64 port with objc2 + rodio.

use std::cell::RefCell;
use std::ffi::c_void;
use std::sync::atomic::{AtomicU8, Ordering};

use objc2::rc::{Retained, autoreleasepool};
use objc2::runtime::{AnyObject, ProtocolObject};
use objc2::{ClassType, MainThreadOnly, define_class, msg_send, sel};
use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy, NSApplicationDelegate};
use objc2_foundation::{MainThreadMarker, NSNotification, NSObject, NSObjectProtocol};

mod app_state;
mod cocoa_util;
mod core_foundation;
mod core_graphics;
mod event_tap;
mod input_recovery;
mod iokit;
mod launcher;
mod pref;
mod settings_ui;
mod settings_window;
use tickeys_lib as tickeys;

use crate::cocoa_util::*;
use crate::core_graphics::*;
use crate::input_recovery::{RecoveryOutcome, RecoveryPolicy, recover_monitor};
use crate::tickeys::AudioScheme;

// ── Globals ──────────────────────────────────────────────────────────────────

static mut KEYBOARD_MONITOR: Option<crate::event_tap::KeyboardMonitor> = None;
static mut POWER_MONITOR: Option<iokit::SystemPowerMonitor> = None;
// All mutable input state is owned by the main run loop. Only the permission
// request's completion flag crosses threads.
thread_local! {
    static INPUT_RECOVERY_POLICY: RefCell<RecoveryPolicy> = const { RefCell::new(RecoveryPolicy::new()) };
}
static mut INPUT_RECOVERY_TARGET: *const AppDelegate = std::ptr::null();
static mut INPUT_RECOVERY_TIMER: *mut AnyObject = std::ptr::null_mut();
const PERMISSION_IDLE: u8 = 0;
const PERMISSION_PENDING: u8 = 1;
const PERMISSION_FINISHED: u8 = 2;
static INPUT_PERMISSION_REQUEST: AtomicU8 = AtomicU8::new(PERMISSION_IDLE);

/// Queue recovery instead of running it in the event tap / IOKit callback.
/// The one-shot timer is retained by the main run loop until it fires.
fn schedule_input_recovery(_mtm: MainThreadMarker, delay: f64) {
    unsafe {
        if INPUT_RECOVERY_TARGET.is_null() || !INPUT_RECOVERY_TIMER.is_null() {
            return;
        }
        if INPUT_RECOVERY_POLICY.with(|policy| policy.borrow().is_sleeping()) {
            return;
        }
        INPUT_RECOVERY_TIMER = msg_send![
            objc2::runtime::AnyClass::get(c"NSTimer").unwrap(),
            scheduledTimerWithTimeInterval: delay,
            target: &*INPUT_RECOVERY_TARGET,
            selector: sel!(recoverInputMonitoring:),
            userInfo: std::ptr::null::<AnyObject>(),
            repeats: false
        ];
    }
}

/// Shared entry point for startup, activation, reopen, wake and Preferences.
/// It only schedules work, so callers never nest permission dialogs or drop an
/// event tap while its callback is still on the stack.
pub fn recheck_input_monitoring(mtm: MainThreadMarker) {
    INPUT_RECOVERY_POLICY.with(|policy| policy.borrow_mut().begin_recheck());
    schedule_input_recovery(mtm, 0.01);
}

fn request_input_permission_once(mtm: MainThreadMarker) {
    let should_request =
        INPUT_RECOVERY_POLICY.with(|policy| policy.borrow_mut().request_permission_once());
    if !should_request {
        return;
    }
    INPUT_PERMISSION_REQUEST.store(PERMISSION_PENDING, Ordering::Release);
    // CoreGraphics' permission request can wait for user interaction. Keep it
    // off the application thread; only the system prompt runs here, no AppKit.
    let request = std::thread::Builder::new()
        .name("input-permission".into())
        .spawn(|| {
            autoreleasepool(|_| unsafe { CGRequestListenEventAccess() });
            INPUT_PERMISSION_REQUEST.store(PERMISSION_FINISHED, Ordering::Release);
        });
    if let Err(error) = request {
        INPUT_PERMISSION_REQUEST.store(PERMISSION_FINISHED, Ordering::Release);
        eprintln!("Input Monitoring permission request failed: {error}");
    }
    // Preferences supplies a persistent explanation, status and Settings link.
    // It remains interactive and can be closed without dismissing a modal alert.
    settings_window::show_prefs_window(mtm);
}

pub fn open_input_monitoring_settings(mtm: MainThreadMarker) {
    request_input_permission_once(mtm);
    unsafe {
        let workspace: *mut AnyObject = msg_send![
            objc2::runtime::AnyClass::get(c"NSWorkspace").unwrap(),
            sharedWorkspace
        ];
        let url: *mut AnyObject = msg_send![
            objc2::runtime::AnyClass::get(c"NSURL").unwrap(),
            URLWithString: &*nsstr(
                "x-apple.systempreferences:com.apple.preference.security?Privacy_ListenEvent"
            )
        ];
        let _: bool = msg_send![workspace, openURL: url];
    }
    recheck_input_monitoring(mtm);
}

fn recover_input_monitoring(mtm: MainThreadMarker) {
    if INPUT_RECOVERY_POLICY.with(|policy| policy.borrow().is_sleeping()) {
        return;
    }
    // Consume completion before preflight. A request finishing during preflight
    // retains FINISHED and schedules one final check, avoiding a lost grant.
    let _ = INPUT_PERMISSION_REQUEST.compare_exchange(
        PERMISSION_FINISHED,
        PERMISSION_IDLE,
        Ordering::AcqRel,
        Ordering::Acquire,
    );
    let outcome = unsafe {
        let slot = &raw mut KEYBOARD_MONITOR;
        recover_monitor(&mut *slot, CGPreflightListenEventAccess(), || {
            event_tap::KeyboardMonitor::new(mtm, handle_keyboard_event, std::ptr::null_mut())
        })
    };
    match outcome {
        RecoveryOutcome::Active => {}
        RecoveryOutcome::PermissionMissing => {
            request_input_permission_once(mtm);
            // Poll only while the one permitted system request is outstanding.
            // A denial ends polling; granting access later is picked up on
            // activation, reopening Preferences or wake.
            if INPUT_PERMISSION_REQUEST.load(Ordering::Acquire) != PERMISSION_IDLE {
                schedule_input_recovery(mtm, 0.5);
            }
        }
        RecoveryOutcome::Failed(error) => {
            eprintln!("KeyboardMonitor recovery failed: {error}");
            let retry_delay =
                INPUT_RECOVERY_POLICY.with(|policy| policy.borrow_mut().next_retry_delay());
            if let Some(delay) = retry_delay {
                schedule_input_recovery(mtm, delay);
            }
        }
    }
    settings_window::refresh(mtm);
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
            if let Some(mtm) = MainThreadMarker::new() {
                recheck_input_monitoring(mtm);
            }
            return event;
        }
        CGEventType::kCGEventKeyDown => {}
        _ => return event,
    }

    let keycode =
        unsafe { CGEventGetIntegerValueField(event, CGEventField::kCGKeyboardEventKeycode) } as u8;
    app_state::handle_keydown(keycode);
    event
}

extern "C" fn handle_power_event(root_port: iokit::io_connect_t, msg: u32, msg_args: *mut c_void) {
    match msg {
        iokit::kIOMessageCanSystemSleep => unsafe {
            let _ = iokit::IOAllowPowerChange(root_port, msg_args as isize);
        },
        iokit::kIOMessageSystemWillSleep => unsafe {
            INPUT_RECOVERY_POLICY.with(|policy| policy.borrow_mut().set_sleeping(true));
            let slot = &raw const KEYBOARD_MONITOR;
            if let Some(monitor) = (*slot).as_ref() {
                monitor.set_enabled(false);
            }
            let _ = iokit::IOAllowPowerChange(root_port, msg_args as isize);
        },
        iokit::kIOMessageSystemHasPoweredOn => {
            INPUT_RECOVERY_POLICY.with(|policy| policy.borrow_mut().set_sleeping(false));
            if let Some(mtm) = MainThreadMarker::new() {
                recheck_input_monitoring(mtm);
            }
        }
        _ => {}
    }
}

// ── AppDelegate ──────────────────────────────────────────────────────────────

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
            app_state::initialize(schemes, audio_tx);

            unsafe { INPUT_RECOVERY_TARGET = self };
            recheck_input_monitoring(mtm);

            match iokit::SystemPowerMonitor::new(handle_power_event) {
                Ok(monitor) => unsafe {
                    POWER_MONITOR = Some(monitor);
                },
                Err(error) => eprintln!("Power monitor failed: {error}"),
            }
            settings_ui::setup_menu(mtm);
            println!("{}", nsstring_to_string(&l10n_str("running")));
        }

        #[unsafe(method(applicationWillTerminate:))]
        fn will_terminate(&self, _notification: &NSNotification) {
            unsafe {
                INPUT_RECOVERY_TARGET = std::ptr::null();
                if !INPUT_RECOVERY_TIMER.is_null() {
                    let _: () = msg_send![INPUT_RECOVERY_TIMER, invalidate];
                    INPUT_RECOVERY_TIMER = std::ptr::null_mut();
                }
                POWER_MONITOR = None;
                // Drop keyboard monitor first (disables event tap)
                KEYBOARD_MONITOR = None;
                app_state::shutdown();
            }
        }

        #[unsafe(method(applicationDidBecomeActive:))]
        fn did_become_active(&self, _notification: &NSNotification) {
            recheck_input_monitoring(self.mtm());
        }

        #[unsafe(method(applicationShouldHandleReopen:hasVisibleWindows:))]
        fn should_handle_reopen(&self, _app: &AnyObject, _flag: bool) -> bool {
            // Reopening the already-running app is the recovery action after
            // granting Input Monitoring permission. This is idempotent.
            recheck_input_monitoring(self.mtm());
            settings_window::show_prefs_window(self.mtm());
            true
        }
    }

    impl AppDelegate {
        #[unsafe(method(recoverInputMonitoring:))]
        fn recover_input_monitoring(&self, _timer: &AnyObject) {
            unsafe { INPUT_RECOVERY_TIMER = std::ptr::null_mut() };
            recover_input_monitoring(self.mtm());
        }
    }
);

pub fn input_monitoring_active() -> bool {
    unsafe {
        let monitor = &raw const KEYBOARD_MONITOR;
        CGPreflightListenEventAccess()
            && (*monitor)
                .as_ref()
                .is_some_and(|monitor| monitor.is_enabled())
    }
}

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
    const EN_LOCALIZATION: &str = include_str!("../assets/lproj/Base.lproj/Localizable.strings");
    const ZH_LOCALIZATION: &str = include_str!("../assets/lproj/zh-Hans.lproj/Localizable.strings");

    #[test]
    fn input_monitoring_recovery_copy_is_complete_in_both_localizations() {
        for localization in [EN_LOCALIZATION, ZH_LOCALIZATION] {
            for key in [
                "permission_title",
                "permission_message",
                "open_input_monitoring_settings",
                "permission_not_now",
                "test_sound",
                "test_sound_hint",
                "audio_loading",
                "audio_starting",
                "audio_ready",
                "audio_retrying",
                "audio_scheme_failed",
                "input_ready",
                "input_unavailable",
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
