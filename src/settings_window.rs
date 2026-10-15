//! Preferences window with shared settings and permission-independent audio preview.

use std::ffi::CStr;
use std::path::{Path, PathBuf};

use objc2::define_class;
use objc2::msg_send;
use objc2::rc::{Retained, autoreleasepool};
use objc2::runtime::AnyObject;
use objc2::sel;
use objc2::{ClassType, MainThreadOnly};
use objc2_foundation::{
    MainThreadMarker, NSBundle, NSObject, NSObjectProtocol, NSPoint, NSRect, NSSize,
};

use crate::cocoa_util::{l10n_str, nsstr, nsstring_to_string, scheme_display_name};
use crate::launcher::LoginItemStatus;
use crate::tickeys::{AudioScheme, MAX_PITCH, MIN_PITCH, OutputStatus, SchemeStatus};

static mut PREFS_WINDOW: *mut AnyObject = core::ptr::null_mut();
static mut PREFS_CONTROLLER: *mut SettingsController = core::ptr::null_mut();
static mut VOL_LABEL: *mut AnyObject = core::ptr::null_mut();
static mut PITCH_LABEL: *mut AnyObject = core::ptr::null_mut();
static mut START_AT_LOGIN_CHECKBOX: *mut AnyObject = core::ptr::null_mut();
static mut SCHEME_POPUP: *mut AnyObject = core::ptr::null_mut();
static mut VOL_SLIDER: *mut AnyObject = core::ptr::null_mut();
static mut PITCH_SLIDER: *mut AnyObject = core::ptr::null_mut();
static mut AUDIO_STATUS_LABEL: *mut AnyObject = core::ptr::null_mut();
static mut INPUT_STATUS_LABEL: *mut AnyObject = core::ptr::null_mut();
static mut INPUT_HELP_LABEL: *mut AnyObject = core::ptr::null_mut();
static mut TEST_SOUND_BUTTON: *mut AnyObject = core::ptr::null_mut();
static mut STATUS_TIMER: *mut AnyObject = core::ptr::null_mut();

pub fn show_prefs_window(mtm: MainThreadMarker) {
    crate::recheck_input_monitoring(mtm);
    unsafe {
        if !PREFS_WINDOW.is_null() {
            refresh_start_at_login_checkbox();
            let _: () =
                msg_send![PREFS_WINDOW, makeKeyAndOrderFront: std::ptr::null::<AnyObject>()];
            activate_app();
            refresh(mtm);
            schedule_status_refresh(&*PREFS_CONTROLLER);
            return;
        }
        autoreleasepool(|_pool| {
            build_and_show_window();
        });
        refresh(mtm);
        schedule_status_refresh(&*PREFS_CONTROLLER);
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
            crate::app_state::set_scheme(self.mtm(), idx as usize);
        }

        #[unsafe(method(changeVolume:))]
        fn change_volume(&self, sender: &AnyObject) {
            let raw: f32 = unsafe { msg_send![sender, floatValue] };
            crate::app_state::set_volume(self.mtm(), raw);
        }

        #[unsafe(method(changePitch:))]
        fn change_pitch(&self, sender: &AnyObject) {
            let pitch: f32 = unsafe { msg_send![sender, floatValue] };
            crate::app_state::set_pitch(self.mtm(), pitch);
        }

        #[unsafe(method(testSound:))]
        fn test_sound(&self, _sender: &AnyObject) {
            crate::app_state::test_sound();
        }

        #[unsafe(method(openInputSettings:))]
        fn open_input_settings(&self, _sender: &AnyObject) {
            crate::open_input_monitoring_settings(self.mtm());
        }

        #[unsafe(method(openAboutLink:))]
        fn open_about_link(&self, sender: &AnyObject) {
            let tag: isize = unsafe { msg_send![sender, tag] };
            if let Some(link) = AboutLink::from_tag(tag) {
                unsafe { open_about_link(link) };
            }
        }

        #[unsafe(method(refreshStatus:))]
        fn refresh_status(&self, _timer: &AnyObject) {
            unsafe {
                STATUS_TIMER = std::ptr::null_mut();
                let visible: bool = msg_send![PREFS_WINDOW, isVisible];
                if visible {
                    refresh_audio_status();
                    schedule_status_refresh(self);
                }
            }
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
        let about_view = build_about_view(&controller);

        // Tab view
        let tab: *mut AnyObject = msg_send![class("NSTabView"), alloc];
        let tab: *mut AnyObject = msg_send![tab, initWithFrame: NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(420.0, 300.0))];
        let _: () = msg_send![tab, setTabViewType: 0usize];
        let _: () = msg_send![tab, addTabViewItem: create_tab_item("sound", &nsstring_to_string(&l10n_str("tab_sound")), sound_view)];
        let _: () = msg_send![tab, addTabViewItem: create_tab_item("general", &nsstring_to_string(&l10n_str("tab_general")), general_view)];
        let _: () = msg_send![tab, addTabViewItem: create_tab_item("about", &nsstring_to_string(&l10n_str("tab_about")), about_view)];

        // Window
        let window: *mut AnyObject = msg_send![class("NSWindow"), alloc];
        let window: *mut AnyObject = msg_send![
            window, initWithContentRect: NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(440.0, 360.0)),
            styleMask: 7usize, backing: 2usize, defer: false
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
    let schemes = crate::app_state::schemes();
    let pref = crate::app_state::preferences();
    let pref_scheme = pref.scheme;
    let pref_vol = pref.volume;
    let pref_pitch = pref.pitch;

    unsafe {
        let container: *mut AnyObject = msg_send![class("NSView"), alloc];
        let container: *mut AnyObject = msg_send![container, initWithFrame: NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(420.0, 300.0))];

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
        SCHEME_POPUP = popup;
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
        VOL_SLIDER = slider;
        set_target(slider, ctrl, sel!(changeVolume:));
        let _: () = msg_send![container, addSubview: slider];
        let vol_label = add_label(
            container,
            &format!("{pref_vol:.2}%"),
            330.0,
            y - 6.0,
            75.0,
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
        let slider = make_slider(
            100.0,
            y - 4.0,
            220.0,
            f64::from(MIN_PITCH),
            f64::from(MAX_PITCH),
            pref_pitch,
        );
        PITCH_SLIDER = slider;
        set_target(slider, ctrl, sel!(changePitch:));
        let _: () = msg_send![container, addSubview: slider];
        let pitch_label = add_label(
            container,
            &format!("{pref_pitch:.2}×"),
            330.0,
            y - 6.0,
            75.0,
            20.0,
        );
        PITCH_LABEL = pitch_label;

        AUDIO_STATUS_LABEL = add_label(container, "", 30.0, 130.0, 365.0, 20.0);
        INPUT_STATUS_LABEL = add_label(container, "", 30.0, 103.0, 365.0, 20.0);
        let test: *mut AnyObject = msg_send![class("NSButton"), alloc];
        let test: *mut AnyObject = msg_send![test, initWithFrame: NSRect::new(NSPoint::new(26.0, 48.0), NSSize::new(150.0, 32.0))];
        let _: () = msg_send![test, setTitle: &*l10n_str("test_sound")];
        let _: () = msg_send![test, setBezelStyle: 1usize];
        let _: () = msg_send![test, setToolTip: &*l10n_str("test_sound_hint")];
        set_target(test, ctrl, sel!(testSound:));
        let _: () = msg_send![container, addSubview: test];
        TEST_SOUND_BUTTON = test;

        let input_settings: *mut AnyObject = msg_send![class("NSButton"), alloc];
        let input_settings: *mut AnyObject = msg_send![input_settings, initWithFrame: NSRect::new(NSPoint::new(180.0, 48.0), NSSize::new(215.0, 32.0))];
        let _: () = msg_send![input_settings, setTitle: &*l10n_str("input_settings")];
        let _: () = msg_send![input_settings, setBezelStyle: 1usize];
        let _: () = msg_send![input_settings, setToolTip: &*l10n_str("permission_message")];
        set_target(input_settings, ctrl, sel!(openInputSettings:));
        let _: () = msg_send![container, addSubview: input_settings];

        INPUT_HELP_LABEL = add_label(
            container,
            &l10n_str("input_permission_hint").to_string(),
            30.0,
            8.0,
            365.0,
            32.0,
        );
        let help_font: *mut AnyObject = msg_send![class("NSFont"), systemFontOfSize: 11.0f64];
        let _: () = msg_send![INPUT_HELP_LABEL, setFont: help_font];
        let _: () = msg_send![INPUT_HELP_LABEL, setLineBreakMode: 0usize];
        let _: () = msg_send![INPUT_HELP_LABEL, setMaximumNumberOfLines: 2isize];
        let _: () = msg_send![INPUT_HELP_LABEL, setToolTip: &*l10n_str("permission_message")];

        container
    }
}

unsafe fn build_general_view(ctrl: &SettingsController) -> *mut AnyObject {
    let show_in_menu_bar = load_pref_bool("show_in_menu_bar", true);

    unsafe {
        let container: *mut AnyObject = msg_send![class("NSView"), alloc];
        let container: *mut AnyObject = msg_send![container, initWithFrame: NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(420.0, 300.0))];

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(isize)]
enum AboutLink {
    Original = 0,
    Redux = 1,
    Releases = 2,
    License = 3,
    ThirdParty = 4,
}

impl AboutLink {
    fn from_tag(tag: isize) -> Option<Self> {
        match tag {
            0 => Some(Self::Original),
            1 => Some(Self::Redux),
            2 => Some(Self::Releases),
            3 => Some(Self::License),
            4 => Some(Self::ThirdParty),
            _ => None,
        }
    }

    fn url(self) -> &'static str {
        match self {
            Self::Original => "https://github.com/yingDev/Tickeys",
            Self::Redux => "https://github.com/E-R-Butch/TickeysRedux",
            Self::Releases => "https://github.com/E-R-Butch/TickeysRedux/releases",
            Self::License => "https://github.com/E-R-Butch/TickeysRedux/blob/master/LICENSE",
            Self::ThirdParty => {
                "https://github.com/E-R-Butch/TickeysRedux/blob/master/THIRD_PARTY_NOTICES.md"
            }
        }
    }

    fn bundled_filename(self) -> Option<&'static str> {
        match self {
            Self::License => Some("LICENSE"),
            Self::ThirdParty => Some("THIRD_PARTY_NOTICES.md"),
            _ => None,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum AboutTarget {
    File(PathBuf),
    Web(&'static str),
}

// Local legal documents remain readable offline. Merely opening Preferences
// does not resolve a URL or make a network request; this runs after a click.
fn about_target(link: AboutLink, resources: Option<&Path>) -> AboutTarget {
    if let Some(path) = resources
        .zip(link.bundled_filename())
        .map(|(dir, name)| dir.join(name))
        .filter(|path| path.is_file())
    {
        AboutTarget::File(path)
    } else {
        AboutTarget::Web(link.url())
    }
}

unsafe fn open_about_link(link: AboutLink) {
    let resources = NSBundle::mainBundle()
        .resourcePath()
        .map(|path| PathBuf::from(path.to_string()));
    unsafe {
        let url: *mut AnyObject = match about_target(link, resources.as_deref()) {
            AboutTarget::File(path) => {
                msg_send![class("NSURL"), fileURLWithPath: &*nsstr(&path.to_string_lossy())]
            }
            AboutTarget::Web(url) => {
                msg_send![class("NSURL"), URLWithString: &*nsstr(url)]
            }
        };
        let workspace: *mut AnyObject = msg_send![class("NSWorkspace"), sharedWorkspace];
        let opened = !url.is_null() && {
            let opened: bool = msg_send![workspace, openURL: url];
            opened
        };
        if !opened {
            let alert: Retained<AnyObject> = msg_send![class("NSAlert"), new];
            let _: () = msg_send![&alert, setMessageText: &*l10n_str("about_link_error_title")];
            let _: () =
                msg_send![&alert, setInformativeText: &*l10n_str("about_link_error_message")];
            let _: *mut AnyObject = msg_send![&alert, addButtonWithTitle: &*l10n_str("ok")];
            let _: isize = msg_send![&alert, runModal];
        }
    }
}

unsafe fn build_about_view(ctrl: &SettingsController) -> *mut AnyObject {
    unsafe {
        let container: *mut AnyObject = msg_send![class("NSView"), alloc];
        let container: *mut AnyObject = msg_send![container, initWithFrame: NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(420.0, 300.0))];
        let bold_font: *mut AnyObject = msg_send![class("NSFont"), boldSystemFontOfSize: 16.0f64];
        let small_font: *mut AnyObject = msg_send![class("NSFont"), systemFontOfSize: 11.0f64];
        let body_font: *mut AnyObject = msg_send![class("NSFont"), systemFontOfSize: 12.0f64];

        let title = add_label(container, "Tickeys Redux", 30.0, 264.0, 380.0, 24.0);
        let _: () = msg_send![title, setFont: bold_font];
        let _: () = msg_send![title, setAlignment: 1isize];
        let version = add_label(
            container,
            &format!(
                "v{}  ·  macOS 13+  ·  Apple Silicon",
                env!("CARGO_PKG_VERSION")
            ),
            30.0,
            242.0,
            380.0,
            18.0,
        );
        let _: () = msg_send![version, setFont: small_font];
        let _: () = msg_send![version, setAlignment: 1isize];

        let description = add_label(
            container,
            &l10n_str("about_description").to_string(),
            30.0,
            204.0,
            380.0,
            30.0,
        );
        let _: () = msg_send![description, setFont: body_font];
        let _: () = msg_send![description, setAlignment: 1isize];
        let _: () = msg_send![description, setLineBreakMode: 0usize];
        let _: () = msg_send![description, setMaximumNumberOfLines: 2isize];

        let rows = [
            (
                "about_original_author",
                "about_original_link",
                AboutLink::Original,
            ),
            ("about_redux_author", "about_redux_link", AboutLink::Redux),
            ("about_updates", "about_releases_link", AboutLink::Releases),
            ("about_license", "about_license_link", AboutLink::License),
            (
                "about_third_party",
                "about_third_party_link",
                AboutLink::ThirdParty,
            ),
        ];
        for (index, (label_key, link_key, link)) in rows.into_iter().enumerate() {
            let y = 176.0 - index as f64 * 26.0;
            let label = add_label(
                container,
                &l10n_str(label_key).to_string(),
                30.0,
                y,
                130.0,
                20.0,
            );
            let _: () = msg_send![label, setFont: body_font];
            let button: *mut AnyObject = msg_send![class("NSButton"), alloc];
            let button: *mut AnyObject = msg_send![button, initWithFrame: NSRect::new(NSPoint::new(160.0, y - 1.0), NSSize::new(230.0, 22.0))];
            let _: () = msg_send![button, setTitle: &*l10n_str(link_key)];
            let _: () = msg_send![button, setFont: body_font];
            let _: () = msg_send![button, setBordered: false];
            let _: () = msg_send![button, setAlignment: 0isize];
            let color: *mut AnyObject = msg_send![class("NSColor"), linkColor];
            let _: () = msg_send![button, setContentTintColor: color];
            let _: () = msg_send![button, setTag: link as isize];
            if link.bundled_filename().is_some() {
                let _: () = msg_send![button, setToolTip: &*l10n_str("about_local_document_hint")];
            } else {
                let _: () = msg_send![button, setToolTip: &*nsstr(link.url())];
            }
            set_target(button, ctrl, sel!(openAboutLink:));
            let _: () = msg_send![container, addSubview: button];
        }

        let footer = add_label(
            container,
            &l10n_str("about_built_with").to_string(),
            30.0,
            24.0,
            380.0,
            32.0,
        );
        let _: () = msg_send![footer, setFont: small_font];
        let _: () = msg_send![footer, setAlignment: 1isize];
        let _: () = msg_send![footer, setLineBreakMode: 0usize];
        let _: () = msg_send![footer, setMaximumNumberOfLines: 2isize];
        container
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

pub fn refresh(_mtm: MainThreadMarker) {
    unsafe {
        if PREFS_WINDOW.is_null() {
            return;
        }
        let pref = crate::app_state::preferences();
        if let Some(index) = crate::app_state::schemes()
            .iter()
            .position(|s| s.name == pref.scheme)
        {
            let _: () = msg_send![SCHEME_POPUP, selectItemAtIndex: index as isize];
        }
        let _: () = msg_send![VOL_SLIDER, setFloatValue: pref.volume];
        let _: () = msg_send![PITCH_SLIDER, setFloatValue: pref.pitch];
        let _: () = msg_send![VOL_LABEL, setStringValue: &*nsstr(&format!("{:.2}%", pref.volume))];
        let _: () = msg_send![PITCH_LABEL, setStringValue: &*nsstr(&format!("{:.2}×", pref.pitch))];
        refresh_audio_status();
    }
}

unsafe fn refresh_audio_status() {
    unsafe {
        let Some(status) = crate::app_state::audio_status() else {
            return;
        };
        let (key, detail) = match (&status.scheme, &status.output) {
            (SchemeStatus::Failed(error), _) => ("audio_scheme_failed", error.as_str()),
            (SchemeStatus::Empty | SchemeStatus::Loading, _) => ("audio_loading", ""),
            (_, OutputStatus::Starting) => ("audio_starting", ""),
            (_, OutputStatus::Retrying(error)) => ("audio_retrying", error.as_str()),
            (_, OutputStatus::Ready) => ("audio_ready", ""),
        };
        let _: () = msg_send![AUDIO_STATUS_LABEL, setStringValue: &*l10n_str(key)];
        let _: () = msg_send![AUDIO_STATUS_LABEL, setToolTip: &*nsstr(detail)];
        let input_active = crate::input_monitoring_active();
        let input_key = if input_active {
            "input_ready"
        } else {
            "input_unavailable"
        };
        let _: () = msg_send![INPUT_STATUS_LABEL, setStringValue: &*l10n_str(input_key)];
        let _: () = msg_send![INPUT_HELP_LABEL, setHidden: input_active];
        let ready = status.scheme == SchemeStatus::Ready && status.output == OutputStatus::Ready;
        let _: () = msg_send![TEST_SOUND_BUTTON, setEnabled: ready];
    }
}

unsafe fn schedule_status_refresh(controller: &SettingsController) {
    unsafe {
        if !STATUS_TIMER.is_null() {
            return;
        }
        // A one-shot timer rearms only while the window is visible. Closing the
        // preferences window therefore stops status polling entirely.
        STATUS_TIMER = msg_send![class("NSTimer"), scheduledTimerWithTimeInterval: 0.5f64,
            target: controller, selector: sel!(refreshStatus:),
            userInfo: std::ptr::null::<AnyObject>(), repeats: false];
        let _: () = msg_send![STATUS_TIMER, setTolerance: 0.1f64];
    }
}

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
        let _: *mut AnyObject =
            msg_send![&alert, addButtonWithTitle: &*l10n_str("open_login_items_settings")];
        let _: *mut AnyObject = msg_send![&alert, addButtonWithTitle: &*l10n_str("cancel")];
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
        let _: *mut AnyObject = msg_send![&alert, addButtonWithTitle: &*l10n_str("ok")];
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
        let tf: *mut AnyObject =
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
        let pb: *mut AnyObject = msg_send![pb, initWithFrame: NSRect::new(NSPoint::new(x, y), NSSize::new(w, 24.0)), pullsDown: false];
        for (i, scheme) in schemes.iter().enumerate() {
            let _: () = msg_send![pb, addItemWithTitle: &*scheme_display_name(scheme)];
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
        let slider: *mut AnyObject =
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
        let btn: *mut AnyObject = msg_send![btn, initWithFrame: NSRect::new(NSPoint::new(x, y), NSSize::new(300.0, 24.0))];
        let _: () = msg_send![btn, setTitle: &*nsstr(title)];
        let _: () = msg_send![btn, setButtonType: 3usize]; // NSSwitchButton = checkbox
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
    use std::collections::BTreeMap;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{AboutLink, AboutTarget, LoginItemStatus, about_target, login_item_checkbox_state};

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

    #[test]
    fn about_legal_links_prefer_existing_bundled_documents() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir =
            std::env::temp_dir().join(format!("tickeys-about-{}-{unique}", std::process::id()));
        std::fs::create_dir(&dir).unwrap();
        for link in [AboutLink::License, AboutLink::ThirdParty] {
            assert_eq!(about_target(link, Some(&dir)), AboutTarget::Web(link.url()));
            let path = dir.join(link.bundled_filename().unwrap());
            std::fs::write(&path, "Bundled license text").unwrap();
            assert_eq!(about_target(link, Some(&dir)), AboutTarget::File(path));
        }
        // External project links never become local documents, and source
        // builds without an app bundle still have usable explicit links.
        for link in [AboutLink::Original, AboutLink::Redux, AboutLink::Releases] {
            assert_eq!(about_target(link, Some(&dir)), AboutTarget::Web(link.url()));
        }
        assert_eq!(
            about_target(AboutLink::License, None),
            AboutTarget::Web("https://github.com/E-R-Butch/TickeysRedux/blob/master/LICENSE")
        );
        assert_eq!(AboutLink::from_tag(-1), None);
        assert_eq!(AboutLink::from_tag(5), None);
        std::fs::remove_dir_all(dir).unwrap();
    }

    fn translations(source: &str) -> BTreeMap<&str, &str> {
        let mut entries = BTreeMap::new();
        for line in source
            .lines()
            .map(str::trim)
            .filter(|line| line.starts_with('"'))
        {
            let (key, value) = line.split_once(" = ").expect("localized key-value entry");
            let key = key.strip_prefix('"').unwrap().strip_suffix('"').unwrap();
            let value = value
                .strip_prefix('"')
                .unwrap()
                .strip_suffix("\";")
                .unwrap();
            assert!(!value.is_empty(), "empty translation for {key}");
            assert!(entries.insert(key, value).is_none(), "duplicate key: {key}");
        }
        entries
    }

    #[test]
    fn english_and_chinese_cover_the_same_preferences_and_about_strings() {
        let en = translations(include_str!(
            "../assets/lproj/Base.lproj/Localizable.strings"
        ));
        let zh = translations(include_str!(
            "../assets/lproj/zh-Hans.lproj/Localizable.strings"
        ));
        assert_eq!(en.keys().collect::<Vec<_>>(), zh.keys().collect::<Vec<_>>());
        for key in [
            "about_description",
            "about_original_author",
            "about_original_link",
            "about_redux_author",
            "about_redux_link",
            "about_updates",
            "about_releases_link",
            "about_license",
            "about_license_link",
            "about_third_party",
            "about_third_party_link",
            "about_built_with",
            "about_local_document_hint",
            "about_link_error_title",
            "about_link_error_message",
            "input_settings",
            "input_permission_hint",
            "permission_message",
        ] {
            assert!(en.contains_key(key), "missing translation: {key}");
        }
        for key in [
            "typewriter",
            "sword",
            "Cherry_G80_3000",
            "Cherry_G80_3494",
            "starwars",
        ] {
            assert!(
                en[key].starts_with("Synth "),
                "synthetic sound must be labeled: {key}"
            );
            assert!(
                zh[key].starts_with("合成"),
                "synthetic sound must be labeled: {key}"
            );
        }
    }
}
