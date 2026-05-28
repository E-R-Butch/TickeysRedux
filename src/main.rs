//! Tickeys Redux — arm64 port with objc2 + rodio.

use std::ffi::c_void;

use objc2::{define_class, msg_send, sel, ClassType, MainThreadOnly};
use objc2::rc::{autoreleasepool, Retained};
use objc2::runtime::ProtocolObject;
use objc2_foundation::{
    MainThreadMarker, NSNotification, NSObject, NSObjectProtocol, NSString,
};
use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy, NSApplicationDelegate};

mod cocoa_util;
mod consts;
mod core_foundation;
mod core_graphics;
mod event_tap;
mod iokit;
mod pref;
mod settings_ui;
mod tickeys;

use crate::cocoa_util::*;
use crate::core_foundation::*;
use crate::core_graphics::*;
use crate::pref::Pref;
use crate::tickeys::{AudioScheme, Tickeys};

// ── Globals ──────────────────────────────────────────────────────────────────

static mut TICKEYS_PTR: Option<*mut Tickeys> = None;

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

            println!("Tickeys Redux launching...");

            let (audio_tx, _worker) =
                tickeys::spawn_audio_worker().expect("failed to start audio worker");

            let schemes = Self::load_schemes();
            let pref = Pref::load(&schemes);

            let mut tickeys_box = Box::new(Tickeys::new(schemes, audio_tx));
            tickeys_box.load_scheme(
                &get_res_path(&format!("data/{}", &pref.scheme)),
                &pref.scheme,
            );
            tickeys_box.set_volume(pref.volume);
            tickeys_box.set_pitch(pref.pitch);

            let ptr = Box::into_raw(tickeys_box);
            unsafe { TICKEYS_PTR = Some(ptr); }

            AppDelegate::start_keyboard_monitor();
            settings_ui::setup_menu(mtm, ptr);
            println!("Tickeys Redux running.");
        }

        #[unsafe(method(applicationWillTerminate:))]
        fn will_terminate(&self, _notification: &NSNotification) {
            unsafe {
                if let Some(ptr) = TICKEYS_PTR.take() {
                    drop(Box::from_raw(ptr));
                }
            }
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

    fn start_keyboard_monitor() {
        use crate::event_tap::KeyboardMonitor;

        extern "C" fn handle_keyboard_event(
            _proxy: CGEventTapProxy,
            _etype: CGEventType,
            event: CGEventRef,
            _refcon: *mut c_void,
        ) -> CGEventRef {
            let keycode = unsafe {
                CGEventGetIntegerValueField(event, CGEventField::kCGKeyboardEventKeycode)
            } as u8;
            unsafe {
                if let Some(ptr) = TICKEYS_PTR {
                    let tickeys: &mut Tickeys = &mut *ptr;
                    tickeys.handle_keydown(keycode);
                }
            }
            event
        }

        std::thread::spawn(move || {
            match KeyboardMonitor::new(handle_keyboard_event, std::ptr::null_mut()) {
                Ok(_monitor) => {
                    println!("KeyboardMonitor started");
                    unsafe { CFRunLoopRun(); }
                }
                Err(e) => eprintln!("KeyboardMonitor failed: {}", e),
            }
        });
    }
}

// ── IOKit power monitoring ───────────────────────────────────────────────────

fn monitor_os_power_event() {
    extern "C" fn power_callback(
        _ref_con: *mut c_void,
        _service: iokit::io_service_t,
        msg: u32,
        _msg_args: *mut c_void,
    ) {
        if msg == iokit::kIOMessageSystemHasPoweredOn {
            app_relaunch_self();
        }
    }

    unsafe {
        let mut notify_port_ref: iokit::IONotificationPortRef = std::ptr::null_mut();
        let mut notifier_object: iokit::io_object_t = 0;
        let root_port = iokit::IORegisterForSystemPower(
            std::ptr::null_mut(),
            &mut notify_port_ref as *mut _,
            power_callback,
            &mut notifier_object as *mut _,
        );
        if root_port == 0 { return; }
        CFRunLoopAddSource(
            CFRunLoopGetCurrent(),
            iokit::IONotificationPortGetRunLoopSource(notify_port_ref) as CFRunLoopSourceRef,
            kCFRunLoopCommonModes,
        );
    }
}

// ── main ─────────────────────────────────────────────────────────────────────

fn main() {
    monitor_os_power_event();

    let mtm = MainThreadMarker::new().expect("must be on main thread");

    autoreleasepool(|_pool| {
        let delegate: Retained<AppDelegate> = unsafe {
            msg_send![AppDelegate::class(), new]
        };

        let app = NSApplication::sharedApplication(mtm);
        app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);
        unsafe {
            app.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));
        }
        app.run();
    });
}
