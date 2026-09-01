//! Settings window with two tabs and wired controls.

use std::ffi::CStr;

use objc2::define_class;
use objc2::msg_send;
use objc2::rc::{Retained, autoreleasepool};
use objc2::runtime::AnyObject;
use objc2::sel;
use objc2::{ClassType, MainThreadOnly};
use objc2_foundation::{MainThreadMarker, NSObject, NSObjectProtocol, NSPoint, NSRect, NSSize};

use crate::cocoa_util::{get_res_path, l10n_str, nsstr, nsstring_to_string};
use crate::launcher::LoginItemStatus;
use crate::tickeys::{AudioScheme, Tickeys};

static mut SOUND_TICKEYS_PTR: *mut Tickeys = core::ptr::null_mut();
static mut PREFS_WINDOW: *mut AnyObject = core::ptr::null_mut();
static mut PREFS_CONTROLLER: *mut SettingsController = core::ptr::null_mut();
static mut VOL_LABEL: *mut AnyObject = core::ptr::null_mut();
static mut PITCH_LABEL: *mut AnyObject = core::ptr::null_mut();
static mut START_AT_LOGIN_CHECKBOX: *mut AnyObject = core::ptr::null_mut();

pub fn show_prefs_window(_mtm: MainThreadMarker, tickeys_ptr: *mut Tickeys) {
    unsafe {
        SOUND_TICKEYS_PTR = tickeys_ptr;
        if !PREFS_WINDOW.is_null() {
            refresh_start_at_login_checkbox();
            let _: () =
                msg_send![PREFS_WINDOW, makeKeyAndOrderFront: std::ptr::null::<AnyObject>()];
            activate_app();
            return;
        }
        autoreleasepool(|_pool| {
            build_and_show_window();
        });
    }
}

define_class!(
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    struct SettingsController;
    unsafe impl NSObjectProtocol for SettingsController {}
    impl SettingsController {
        #[unsafe(method(changeScheme:))]
        fn change_scheme(&self, sender: &AnyObject) {
            let idx: isize = unsafe { msg_send![sender, indexOfSelectedItem] };
            let schemes = load_schemes();
            if (idx as usize) < schemes.len() {
                let name = &schemes[idx as usize].name;
                unsafe {
                    if !SOUND_TICKEYS_PTR.is_null() {
                        let dir = get_res_path(&format!("data/{}", name));
                        (*SOUND_TICKEYS_PTR).load_scheme(&dir, name);
                    }
                }
                save_pref("audio_scheme", name);
            }
        }

        #[unsafe(method(changeVolume:))]
        fn change_volume(&self, sender: &AnyObject) {
            let raw: f32 = unsafe { msg_send![sender, floatValue] };
            let vol = (raw / 100.0).clamp(0.0, 1.0);
            unsafe {
                if !SOUND_TICKEYS_PTR.is_null() { (*SOUND_TICKEYS_PTR).set_volume(vol); }
                if !VOL_LABEL.is_null() {
                    let _: () = msg_send![VOL_LABEL, setStringValue: &*nsstr(&format!("{}", raw as i32))];
                }
            }
            save_pref_float("audio_volume", raw);
        }

        #[unsafe(method(changePitch:))]
        fn change_pitch(&self, sender: &AnyObject) {
            let pitch: f32 = unsafe { msg_send![sender, floatValue] };
            let pitch = pitch.clamp(0.25, 2.0);
            unsafe {
                if !SOUND_TICKEYS_PTR.is_null() { (*SOUND_TICKEYS_PTR).set_pitch(pitch); }
                if !PITCH_LABEL.is_null() {
                    let _: () = msg_send![PITCH_LABEL, setStringValue: &*nsstr(&format!("{:.1}x", pitch))];
                }
            }
            save_pref_float("audio_pitch", pitch);
        }

        #[unsafe(method(toggleStartAtLogin:))]
        fn toggle_start_at_login(&self, sender: &AnyObject) {
            let state: isize = unsafe { msg_send![sender, state] };
            match crate::launcher::set_start_at_login(state == 1) {
                Ok(status) => unsafe {
                    refresh_start_at_login_checkbox();
                    if status == LoginItemStatus::RequiresApproval {
                        show_login_item_approval_alert();
                    }
                },
                Err(error) => unsafe {
                    refresh_start_at_login_checkbox();
                    show_login_item_error(&error);
                },
            }
        }

        #[unsafe(method(toggleShowInMenuBar:))]
        fn toggle_show_in_menu_bar(&self, sender: &AnyObject) {
            let state: isize = unsafe { msg_send![sender, state] };
            let visible = state == 1;
            save_pref_bool("show_in_menu_bar", visible);
            crate::settings_ui::set_menu_bar_visible(self.mtm(), visible);
        }
    }
);

unsafe fn build_and_show_window() {
    unsafe {
        let controller: Retained<SettingsController> = msg_send![SettingsController::class(), new];

        let sound_view = build_sound_view(&controller);
        let general_view = build_general_view(&controller);
        let about_view = build_about_view();

        // Tab view
        let tab: *mut AnyObject = msg_send![class("NSTabView"), alloc];
        let _: () = msg_send![tab, initWithFrame: NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(420.0, 300.0))];
        let _: () = msg_send![tab, setTabViewType: 0u32];
        let _: () = msg_send![tab, addTabViewItem: create_tab_item("sound", &nsstring_to_string(&l10n_str("tab_sound")), sound_view)];
        let _: () = msg_send![tab, addTabViewItem: create_tab_item("general", &nsstring_to_string(&l10n_str("tab_general")), general_view)];
        let _: () = msg_send![tab, addTabViewItem: create_tab_item("about", &nsstring_to_string(&l10n_str("tab_about")), about_view)];

        // Window
        let window: *mut AnyObject = msg_send![class("NSWindow"), alloc];
        let window: *mut AnyObject = msg_send![
            window, initWithContentRect: NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(440.0, 360.0)),
            styleMask: 15u32, backing: 2u32, defer: false
        ];
        let _: () = msg_send![window, setTitle: &*nsstr("Tickeys Redux")];
        let _: () = msg_send![window, setReleasedWhenClosed: false];
        let _: () = msg_send![window, setContentView: tab];
        let _: () = msg_send![window, center];
        let _: () = msg_send![window, makeKeyAndOrderFront: std::ptr::null::<AnyObject>()];
        activate_app();

        PREFS_WINDOW = window;
        PREFS_CONTROLLER = &*controller as *const SettingsController as *mut SettingsController;
        std::mem::forget(controller);
    }
}

unsafe fn build_sound_view(ctrl: &SettingsController) -> *mut AnyObject {
    let schemes = load_schemes();
    let pref_scheme = load_pref_string("audio_scheme").unwrap_or_else(|| schemes[0].name.clone());
    let pref_vol = load_pref_float("audio_volume");
    let pref_pitch = load_pref_float("audio_pitch");

    unsafe {
        let container: *mut AnyObject = msg_send![class("NSView"), alloc];
        let _: () = msg_send![container, initWithFrame: NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(420.0, 300.0))];

        let mut y: f64 = 265.0;

        add_label(
            container,
            &nsstring_to_string(&l10n_str("sound_scheme")),
            30.0,
            y,
            100.0,
            20.0,
        );
        let popup = make_popup(140.0, y - 4.0, 240.0, &schemes, &pref_scheme, ctrl);
        let _: () = msg_send![container, addSubview: popup];
        y -= 40.0;

        add_label(
            container,
            &nsstring_to_string(&l10n_str("volume")),
            30.0,
            y,
            80.0,
            20.0,
        );
        let slider = make_slider(100.0, y - 4.0, 220.0, 0.0, 100.0, pref_vol);
        set_target(slider, ctrl, sel!(changeVolume:));
        let _: () = msg_send![container, addSubview: slider];
        let vol_label = add_label(
            container,
            &format!("{}", pref_vol as i32),
            330.0,
            y - 6.0,
            50.0,
            20.0,
        );
        VOL_LABEL = vol_label;
        y -= 40.0;

        add_label(
            container,
            &nsstring_to_string(&l10n_str("pitch")),
            30.0,
            y,
            80.0,
            20.0,
        );
        let slider = make_slider(100.0, y - 4.0, 220.0, 0.1, 2.0, pref_pitch);
        set_target(slider, ctrl, sel!(changePitch:));
        let _: () = msg_send![container, addSubview: slider];
        let pitch_label = add_label(
            container,
            &format!("{:.1}x", pref_pitch),
            330.0,
            y - 6.0,
            50.0,
            20.0,
        );
        PITCH_LABEL = pitch_label;

        container
    }
}

unsafe fn build_general_view(ctrl: &SettingsController) -> *mut AnyObject {
    let show_in_menu_bar = load_pref_bool("show_in_menu_bar", true);

    unsafe {
        let container: *mut AnyObject = msg_send![class("NSView"), alloc];
        let _: () = msg_send![container, initWithFrame: NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(420.0, 300.0))];

        let mut y: f64 = 265.0;

        let btn = make_checkbox(
            &nsstring_to_string(&l10n_str("start_at_login")),
            30.0,
            y,
            false,
        );
        set_target(btn, ctrl, sel!(toggleStartAtLogin:));
        let _: () = msg_send![container, addSubview: btn];
        START_AT_LOGIN_CHECKBOX = btn;
        refresh_start_at_login_checkbox();
        y -= 35.0;

        let btn = make_checkbox(
            &nsstring_to_string(&l10n_str("show_in_menu_bar")),
            30.0,
            y,
            show_in_menu_bar,
        );
        set_target(btn, ctrl, sel!(toggleShowInMenuBar:));
        let _: () = msg_send![container, addSubview: btn];
        // y tracking is maintained for potential future controls

        container
    }
}

unsafe fn build_about_view() -> *mut AnyObject {
    unsafe {
        let container: *mut AnyObject = msg_send![class("NSView"), alloc];
        let _: () = msg_send![container, initWithFrame: NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(420.0, 300.0))];

        let bold_font: *mut AnyObject = msg_send![class("NSFont"), boldSystemFontOfSize: 16.0f64];
        let small_font: *mut AnyObject = msg_send![class("NSFont"), systemFontOfSize: 11.0f64];
        let body_font: *mut AnyObject = msg_send![class("NSFont"), systemFontOfSize: 12.0f64];

        let mut y: f64 = 260.0;

        // App name
        let label = add_label(container, "Tickeys Redux", 160.0, y, 120.0, 22.0);
        let _: () = msg_send![label, setFont: bold_font];
        let _: () = msg_send![label, setAlignment: 1u32]; // center
        y -= 28.0;

        // Version
        let ver = add_label(
            container,
            &format!("v{}  •  macOS 13+  •  arm64", env!("CARGO_PKG_VERSION")),
            80.0,
            y,
            280.0,
            18.0,
        );
        let _: () = msg_send![ver, setFont: small_font];
        let _: () = msg_send![ver, setAlignment: 1u32];
        y -= 30.0;

        // Description
        let desc = add_label(
            container,
            "机械键盘音效模拟工具 — 每次击键都带来真实的打字体验。",
            30.0,
            y,
            380.0,
            34.0,
        );
        let _: () = msg_send![desc, setFont: body_font];
        y -= 44.0;

        // Separator
        let sep: *mut AnyObject = msg_send![class("NSBox"), alloc];
        let _: () = msg_send![sep, initWithFrame: NSRect::new(NSPoint::new(30.0, y), NSSize::new(380.0, 2.0))];
        let _: () = msg_send![sep, setBoxType: 2u32];
        let _: () = msg_send![container, addSubview: sep];
        y -= 20.0;

        // Credits
        let sec_font: *mut AnyObject = msg_send![class("NSFont"), boldSystemFontOfSize: 12.0f64];

        let h1 = add_label(container, "原作者", 30.0, y, 80.0, 18.0);
        let _: () = msg_send![h1, setFont: sec_font];
        let a1 = add_label(
            container,
            "应元东 — github.com/yingDev/Tickeys",
            120.0,
            y,
            290.0,
            18.0,
        );
        let _: () = msg_send![a1, setFont: small_font];
        y -= 22.0;

        let h2 = add_label(container, "Redux 移植", 30.0, y, 80.0, 18.0);
        let _: () = msg_send![h2, setFont: sec_font];
        let a2 = add_label(
            container,
            "Sinclair Liu — github.com/E-R-Butch/TickeysRedux",
            120.0,
            y,
            290.0,
            18.0,
        );
        let _: () = msg_send![a2, setFont: small_font];
        y -= 22.0;

        let h3 = add_label(container, "许可证", 30.0, y, 80.0, 18.0);
        let _: () = msg_send![h3, setFont: sec_font];
        let a3 = add_label(container, "MIT License", 120.0, y, 200.0, 18.0);
        let _: () = msg_send![a3, setFont: small_font];
        y -= 30.0;

        // Built with
        let bw = add_label(
            container,
            "Built with Rust • objc2 • rodio • 致敬 yingDev 的 Tickeys",
            30.0,
            y,
            380.0,
            16.0,
        );
        let _: () = msg_send![bw, setFont: small_font];

        container
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn login_item_checkbox_state(status: LoginItemStatus) -> isize {
    if status.is_registered() { 1 } else { 0 }
}

unsafe fn refresh_start_at_login_checkbox() {
    unsafe {
        if START_AT_LOGIN_CHECKBOX.is_null() {
            return;
        }

        match crate::launcher::start_at_login_status() {
            Ok(status) => {
                // RequiresApproval is still a registered login item. Keep it
                // checked so the user can click again to unregister it.
                let state = login_item_checkbox_state(status);
                let _: () = msg_send![START_AT_LOGIN_CHECKBOX, setState: state];
                let _: () = msg_send![START_AT_LOGIN_CHECKBOX, setEnabled: true];
            }
            Err(_) => {
                let _: () = msg_send![START_AT_LOGIN_CHECKBOX, setState: 0isize];
                let _: () = msg_send![START_AT_LOGIN_CHECKBOX, setEnabled: false];
            }
        }
    }
}

unsafe fn show_login_item_approval_alert() {
    unsafe {
        let alert: Retained<AnyObject> = msg_send![class("NSAlert"), new];
        let _: () = msg_send![&alert, setMessageText: &*l10n_str("login_item_approval_title")];
        let _: () =
            msg_send![&alert, setInformativeText: &*l10n_str("login_item_approval_message")];
        let _: () = msg_send![&alert, addButtonWithTitle: &*l10n_str("open_login_items_settings")];
        let _: () = msg_send![&alert, addButtonWithTitle: &*l10n_str("cancel")];
        let response: isize = msg_send![&alert, runModal];
        if response == 1000 {
            if let Err(error) = crate::launcher::open_login_items_settings() {
                show_login_item_error(&error);
            }
        }
    }
}

unsafe fn show_login_item_error(error: &str) {
    unsafe {
        let alert: Retained<AnyObject> = msg_send![class("NSAlert"), new];
        let _: () = msg_send![&alert, setMessageText: &*l10n_str("login_item_error_title")];
        let _: () = msg_send![&alert, setInformativeText: &*nsstr(error)];
        let _: () = msg_send![&alert, addButtonWithTitle: &*l10n_str("ok")];
        let _: isize = msg_send![&alert, runModal];
    }
}

unsafe fn activate_app() {
    unsafe {
        let app: *mut AnyObject = msg_send![class("NSApplication"), sharedApplication];
        let _: () = msg_send![app, activateIgnoringOtherApps: true];
    }
}

unsafe fn add_label(
    parent: *mut AnyObject,
    text: &str,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
) -> *mut AnyObject {
    unsafe {
        let tf: *mut AnyObject = msg_send![class("NSTextField"), alloc];
        let _: () =
            msg_send![tf, initWithFrame: NSRect::new(NSPoint::new(x, y), NSSize::new(w, h))];
        let _: () = msg_send![tf, setStringValue: &*nsstr(text)];
        let _: () = msg_send![tf, setBezeled: false];
        let _: () = msg_send![tf, setDrawsBackground: false];
        let _: () = msg_send![tf, setEditable: false];
        let _: () = msg_send![tf, setSelectable: false];
        let _: () = msg_send![parent, addSubview: tf];
        tf
    }
}

unsafe fn make_popup(
    x: f64,
    y: f64,
    w: f64,
    schemes: &[AudioScheme],
    current: &str,
    ctrl: &SettingsController,
) -> *mut AnyObject {
    unsafe {
        let pb: *mut AnyObject = msg_send![class("NSPopUpButton"), alloc];
        let _: () = msg_send![pb, initWithFrame: NSRect::new(NSPoint::new(x, y), NSSize::new(w, 24.0)), pullsDown: false];
        for (i, scheme) in schemes.iter().enumerate() {
            let _: () = msg_send![pb, addItemWithTitle: &*nsstr(&nsstring_to_string(&l10n_str(&scheme.name)))];
            if scheme.name == current {
                let _: () = msg_send![pb, selectItemAtIndex: i as isize];
            }
        }
        set_target(pb, ctrl, sel!(changeScheme:));
        pb
    }
}

unsafe fn make_slider(x: f64, y: f64, w: f64, min: f64, max: f64, value: f32) -> *mut AnyObject {
    unsafe {
        let slider: *mut AnyObject = msg_send![class("NSSlider"), alloc];
        let _: () =
            msg_send![slider, initWithFrame: NSRect::new(NSPoint::new(x, y), NSSize::new(w, 24.0))];
        let _: () = msg_send![slider, setMinValue: min];
        let _: () = msg_send![slider, setMaxValue: max];
        let _: () = msg_send![slider, setFloatValue: value];
        let _: () = msg_send![slider, setContinuous: true];
        slider
    }
}

unsafe fn make_checkbox(title: &str, x: f64, y: f64, checked: bool) -> *mut AnyObject {
    unsafe {
        let btn: *mut AnyObject = msg_send![class("NSButton"), alloc];
        let _: () = msg_send![btn, initWithFrame: NSRect::new(NSPoint::new(x, y), NSSize::new(300.0, 24.0))];
        let _: () = msg_send![btn, setTitle: &*nsstr(title)];
        let _: () = msg_send![btn, setButtonType: 3u32]; // NSSwitchButton = checkbox
        let _: () = msg_send![btn, setState: if checked { 1isize } else { 0isize }];
        btn
    }
}

unsafe fn set_target(
    control: *mut AnyObject,
    ctrl: &SettingsController,
    action: objc2::runtime::Sel,
) {
    unsafe {
        let _: () = msg_send![control, setTarget: ctrl];
        let _: () = msg_send![control, setAction: action];
    }
}

unsafe fn create_tab_item(ident: &str, label: &str, view: *mut AnyObject) -> *mut AnyObject {
    unsafe {
        let item: *mut AnyObject = msg_send![class("NSTabViewItem"), alloc];
        let item: *mut AnyObject = msg_send![item, initWithIdentifier: &*nsstr(ident)];
        let _: () = msg_send![item, setLabel: &*nsstr(label)];
        let _: () = msg_send![item, setView: view];
        item
    }
}

// ── Prefs ────────────────────────────────────────────────────────────────────

fn load_schemes() -> Vec<AudioScheme> {
    let path = get_res_path("data/schemes.json");
    let mut f = std::fs::File::open(&path).unwrap();
    let mut s = String::new();
    std::io::Read::read_to_string(&mut f, &mut s).unwrap();
    serde_json::from_str(&s).unwrap()
}

fn load_pref_string(key: &str) -> Option<String> {
    unsafe {
        let ud: *mut AnyObject = msg_send![class("NSUserDefaults"), standardUserDefaults];
        let val: *mut AnyObject = msg_send![ud, stringForKey: &*nsstr(key)];
        if val.is_null() {
            None
        } else {
            Some(
                CStr::from_ptr(msg_send![val, UTF8String])
                    .to_string_lossy()
                    .to_string(),
            )
        }
    }
}

fn load_pref_float(key: &str) -> f32 {
    unsafe {
        let ud: *mut AnyObject = msg_send![class("NSUserDefaults"), standardUserDefaults];
        let val: *mut AnyObject = msg_send![ud, objectForKey: &*nsstr(key)];
        if val.is_null() {
            match key {
                "audio_volume" => 50.0,
                "audio_pitch" => 1.0,
                _ => 0.0,
            }
        } else {
            let v: f32 = msg_send![ud, floatForKey: &*nsstr(key)];
            if key == "audio_volume" {
                crate::pref::normalize_volume_percent(v).0
            } else {
                v
            }
        }
    }
}

fn load_pref_bool(key: &str, default: bool) -> bool {
    unsafe {
        let ud: *mut AnyObject = msg_send![class("NSUserDefaults"), standardUserDefaults];
        let val: *mut AnyObject = msg_send![ud, objectForKey: &*nsstr(key)];
        if val.is_null() {
            default
        } else {
            msg_send![val, boolValue]
        }
    }
}

fn save_pref(key: &str, val: &str) {
    unsafe {
        let ud: *mut AnyObject = msg_send![class("NSUserDefaults"), standardUserDefaults];
        let _: () = msg_send![ud, setObject: &*nsstr(val), forKey: &*nsstr(key)];
    }
}

fn save_pref_float(key: &str, val: f32) {
    unsafe {
        let ud: *mut AnyObject = msg_send![class("NSUserDefaults"), standardUserDefaults];
        let _: () = msg_send![ud, setDouble: val as f64, forKey: &*nsstr(key)];
    }
}

fn save_pref_bool(key: &str, val: bool) {
    unsafe {
        let ud: *mut AnyObject = msg_send![class("NSUserDefaults"), standardUserDefaults];
        let num: *mut AnyObject = msg_send![class("NSNumber"), numberWithBool: val];
        let _: () = msg_send![ud, setObject: num, forKey: &*nsstr(key)];
    }
}

fn class(name: &str) -> &'static objc2::runtime::AnyClass {
    let s = format!("{}\0", name);
    objc2::runtime::AnyClass::get(CStr::from_bytes_with_nul(s.as_bytes()).unwrap()).unwrap()
}

#[cfg(test)]
mod tests {
    use super::{LoginItemStatus, login_item_checkbox_state};

    #[test]
    fn login_item_status_projects_to_checkbox_state() {
        assert_eq!(login_item_checkbox_state(LoginItemStatus::NotRegistered), 0);
        assert_eq!(login_item_checkbox_state(LoginItemStatus::Enabled), 1);
        assert_eq!(
            login_item_checkbox_state(LoginItemStatus::RequiresApproval),
            1
        );
        assert_eq!(login_item_checkbox_state(LoginItemStatus::NotFound), 0);
        assert_eq!(login_item_checkbox_state(LoginItemStatus::Unknown(99)), 0);
    }
}
