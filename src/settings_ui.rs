//! Settings UI — menu bar with scheme/volume/pitch controls.
//! Uses NSStatusBar item + NSMenu, with a MenuHandler target for actions.
//! Menu is rebuilt on every action to keep checkmarks current.

use objc2::rc::Retained;
use objc2::{MainThreadOnly, define_class, msg_send, sel};
use objc2_app_kit::{NSMenu, NSMenuItem, NSStatusBar, NSStatusItem, NSVariableStatusItemLength};
use objc2_foundation::{MainThreadMarker, NSObject, NSObjectProtocol, NSString, NSUserDefaults};

use crate::cocoa_util::*;
use crate::tickeys::{AudioScheme, Tickeys};

static mut MENU_TICKEYS: *mut Tickeys = core::ptr::null_mut();
// Store the NSStatusItem as a raw pointer so rebuild() can update its menu
// without creating duplicate items.
static mut MENU_ITEM: *mut NSStatusItem = core::ptr::null_mut();

// Volume tags: 0=25%, 1=50%, 2=75%, 3=100%
const VOL_KEYS: [(&str, f32); 4] = [
    ("vol_25", 25.0),
    ("vol_50", 50.0),
    ("vol_75", 75.0),
    ("vol_100", 100.0),
];
const PITCH_KEYS: [(&str, f32); 5] = [
    ("pitch_05", 0.5),
    ("pitch_075", 0.75),
    ("pitch_10", 1.0),
    ("pitch_15", 1.5),
    ("pitch_20", 2.0),
];

// ── MenuHandler ──────────────────────────────────────────────────────────────

define_class!(
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[derive(Debug)]
    struct MenuHandler;

    unsafe impl NSObjectProtocol for MenuHandler {}

    impl MenuHandler {
        #[unsafe(method(changeScheme:))]
        fn change_scheme(&self, sender: &NSMenuItem) {
            let idx = sender.tag() as usize;
            let schemes = load_schemes();
            if idx >= schemes.len() { return; }
            let name = &schemes[idx].name;

            unsafe {
                if !MENU_TICKEYS.is_null() {
                    let dir = get_res_path(&format!("data/{}", name));
                    (*MENU_TICKEYS).load_scheme(&dir, name);
                }
            }
            save_string("audio_scheme", name);
            rebuild(self, &schemes, self.mtm());
        }

        #[unsafe(method(setVolume:))]
        fn set_volume(&self, sender: &NSMenuItem) {
            let idx = sender.tag() as usize;
            if idx >= VOL_KEYS.len() { return; }
            let volume_percent = VOL_KEYS[idx].1;

            unsafe {
                if !MENU_TICKEYS.is_null() {
                    (*MENU_TICKEYS).set_volume(volume_percent / 100.0);
                }
            }
            save_float("audio_volume", volume_percent);
            let schemes = load_schemes();
            rebuild(self, &schemes, self.mtm());
        }

        #[unsafe(method(setPitch:))]
        fn set_pitch(&self, sender: &NSMenuItem) {
            let idx = sender.tag() as usize;
            if idx >= PITCH_KEYS.len() { return; }
            let pitch = PITCH_KEYS[idx].1;

            unsafe { if !MENU_TICKEYS.is_null() { (*MENU_TICKEYS).set_pitch(pitch); } }
            save_float("audio_pitch", pitch);
            let schemes = load_schemes();
            rebuild(self, &schemes, self.mtm());
        }

        #[unsafe(method(openPreferences:))]
        fn open_preferences(&self, _sender: &NSMenuItem) {
            let mtm = self.mtm();
            unsafe {
                if !MENU_TICKEYS.is_null() {
                    crate::settings_window::show_prefs_window(mtm, MENU_TICKEYS);
                }
            }
        }
    }
);

/// Rebuild the menu from scratch, setting checkmarks on the current selection.
fn rebuild(handler: &MenuHandler, schemes: &[AudioScheme], mtm: MainThreadMarker) {
    let pref_scheme = load_pref_scheme(schemes);
    let pref_vol = load_pref_float("audio_volume");
    let pref_pitch = load_pref_float("audio_pitch");

    unsafe {
        if MENU_ITEM.is_null() {
            return;
        }
        let item: &NSStatusItem = &*MENU_ITEM;

        let menu = NSMenu::initWithTitle(NSMenu::alloc(mtm), &NSString::from_str(""));

        // Scheme submenu
        let si = NSMenuItem::initWithTitle_action_keyEquivalent(
            NSMenuItem::alloc(mtm),
            &l10n_str("sound_scheme"),
            None,
            &NSString::from_str(""),
        );
        let sm = NSMenu::initWithTitle(NSMenu::alloc(mtm), &NSString::from_str(""));
        for (i, scheme) in schemes.iter().enumerate() {
            let cm = if scheme.name == pref_scheme {
                "\u{2713} "
            } else {
                "  "
            };
            let disp = nsstring_to_string(&l10n_str(&scheme.name));
            let title = format!("{}{}", cm, disp);
            let mi = NSMenuItem::initWithTitle_action_keyEquivalent(
                NSMenuItem::alloc(mtm),
                &NSString::from_str(&title),
                Some(sel!(changeScheme:)),
                &NSString::from_str(""),
            );
            mi.setTag(i as isize);
            mi.setTarget(Some(handler));
            sm.addItem(&mi);
        }
        si.setSubmenu(Some(&sm));
        menu.addItem(&si);

        // Volume submenu
        let vi = NSMenuItem::initWithTitle_action_keyEquivalent(
            NSMenuItem::alloc(mtm),
            &l10n_str("volume"),
            None,
            &NSString::from_str(""),
        );
        let vm = NSMenu::initWithTitle(NSMenu::alloc(mtm), &NSString::from_str(""));
        for (i, (key, v)) in VOL_KEYS.iter().enumerate() {
            let cm = if (*v - pref_vol).abs() < 0.01 {
                "\u{2713} "
            } else {
                "  "
            };
            let label = nsstring_to_string(&l10n_str(key));
            let title = format!("{}{}", cm, label);
            let mi = NSMenuItem::initWithTitle_action_keyEquivalent(
                NSMenuItem::alloc(mtm),
                &NSString::from_str(&title),
                Some(sel!(setVolume:)),
                &NSString::from_str(""),
            );
            mi.setTag(i as isize);
            mi.setTarget(Some(handler));
            vm.addItem(&mi);
        }
        vi.setSubmenu(Some(&vm));
        menu.addItem(&vi);

        // Pitch submenu
        let pi = NSMenuItem::initWithTitle_action_keyEquivalent(
            NSMenuItem::alloc(mtm),
            &l10n_str("pitch"),
            None,
            &NSString::from_str(""),
        );
        let pm = NSMenu::initWithTitle(NSMenu::alloc(mtm), &NSString::from_str(""));
        for (i, (key, p)) in PITCH_KEYS.iter().enumerate() {
            let cm = if (*p - pref_pitch).abs() < 0.01 {
                "\u{2713} "
            } else {
                "  "
            };
            let label = nsstring_to_string(&l10n_str(key));
            let title = format!("{}{}", cm, label);
            let mi = NSMenuItem::initWithTitle_action_keyEquivalent(
                NSMenuItem::alloc(mtm),
                &NSString::from_str(&title),
                Some(sel!(setPitch:)),
                &NSString::from_str(""),
            );
            mi.setTag(i as isize);
            mi.setTarget(Some(handler));
            pm.addItem(&mi);
        }
        pi.setSubmenu(Some(&pm));
        menu.addItem(&pi);

        // Preferences
        let pref = NSMenuItem::initWithTitle_action_keyEquivalent(
            NSMenuItem::alloc(mtm),
            &l10n_str("preferences"),
            Some(sel!(openPreferences:)),
            &NSString::from_str(""),
        );
        pref.setTarget(Some(handler));
        menu.addItem(&pref);

        // Quit
        menu.addItem(&NSMenuItem::separatorItem(mtm));
        let q = NSMenuItem::initWithTitle_action_keyEquivalent(
            NSMenuItem::alloc(mtm),
            &l10n_str("quit_tickeys"),
            Some(sel!(terminate:)),
            &NSString::from_str("q"),
        );
        menu.addItem(&q);

        item.setMenu(Some(&menu));

        // The parent menu retains each submenu. Let the local Retained values
        // drop so rebuilding this menu does not leak three objects per action.
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn save_string(key: &str, val: &str) {
    let ud = NSUserDefaults::standardUserDefaults();
    let k = NSString::from_str(key);
    let v = NSString::from_str(val);
    unsafe {
        let _: () = msg_send![&ud, setObject: &*v, forKey: &*k];
    }
}

fn save_float(key: &str, val: f32) {
    let ud = NSUserDefaults::standardUserDefaults();
    let k = NSString::from_str(key);
    unsafe {
        let _: () = msg_send![&ud, setDouble: val as f64, forKey: &*k];
    }
}

// ── Public API ───────────────────────────────────────────────────────────────

pub fn setup_menu(mtm: MainThreadMarker, tickeys_ptr: *mut Tickeys) {
    unsafe {
        MENU_TICKEYS = tickeys_ptr;
    }

    let schemes = load_schemes();
    let handler: Retained<MenuHandler> = unsafe { msg_send![MenuHandler::alloc(mtm), init] };

    // Create the status bar item once and store it.
    unsafe {
        let status_bar = NSStatusBar::systemStatusBar();
        let item: Retained<NSStatusItem> =
            msg_send![&status_bar, statusItemWithLength: NSVariableStatusItemLength];
        let button = item.button(mtm).expect("must have button");
        button.setTitle(&NSString::from_str("\u{1F3B9}"));
        let raw = &*item as *const NSStatusItem as *mut NSStatusItem;
        MENU_ITEM = raw;
        std::mem::forget(item);
    }

    rebuild(&handler, &schemes, mtm);
    std::mem::forget(handler);

    // Respect initial preference: hide if user previously turned it off
    let show = load_pref_bool("show_in_menu_bar", true);
    if !show {
        set_menu_bar_visible(mtm, false);
    }
}

/// Show or hide the menu bar icon.
pub fn set_menu_bar_visible(mtm: MainThreadMarker, visible: bool) {
    unsafe {
        if MENU_ITEM.is_null() {
            return;
        }
        let item: &NSStatusItem = &*MENU_ITEM;
        let button = item.button(mtm);
        if let Some(btn) = button {
            btn.setHidden(!visible);
        }
        // Save preference
        let ud = NSUserDefaults::standardUserDefaults();
        let k = NSString::from_str("show_in_menu_bar");
        let v: *mut objc2::runtime::AnyObject =
            msg_send![objc2::runtime::AnyClass::get(c"NSNumber").unwrap(), numberWithBool: visible];
        let _: () = msg_send![&ud, setObject: v, forKey: &*k];
    }
}

fn load_pref_bool(key: &str, default: bool) -> bool {
    unsafe {
        let ud = NSUserDefaults::standardUserDefaults();
        let k = NSString::from_str(key);
        let val: *mut objc2::runtime::AnyObject = msg_send![&ud, objectForKey: &*k];
        if val.is_null() {
            default
        } else {
            msg_send![val, boolValue]
        }
    }
}

// ── Preferences loading ──────────────────────────────────────────────────────

fn load_schemes() -> Vec<AudioScheme> {
    let path = get_res_path("data/schemes.json");
    let mut f = std::fs::File::open(&path).unwrap();
    let mut s = String::new();
    std::io::Read::read_to_string(&mut f, &mut s).unwrap();
    serde_json::from_str(&s).unwrap()
}

fn load_pref_scheme(schemes: &[AudioScheme]) -> String {
    load_pref_string("audio_scheme").unwrap_or_else(|| schemes[0].name.clone())
}

fn load_pref_string(key: &str) -> Option<String> {
    unsafe {
        let ud = NSUserDefaults::standardUserDefaults();
        let k = NSString::from_str(key);
        let val: Option<Retained<NSString>> = msg_send![&ud, stringForKey: &*k];
        val.map(|s| s.to_string())
    }
}

fn load_pref_float(key: &str) -> f32 {
    unsafe {
        let ud = NSUserDefaults::standardUserDefaults();
        let k = NSString::from_str(key);
        let value: f32 = msg_send![&ud, floatForKey: &*k];
        if key == "audio_volume" {
            crate::pref::normalize_volume_percent(value).0
        } else {
            value
        }
    }
}
