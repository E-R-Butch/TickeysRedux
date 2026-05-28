//! Settings UI — menu bar with scheme/volume/pitch controls.
//! Uses NSStatusBar item + NSMenu, with a MenuHandler target for actions.
//! Menu is rebuilt on every action to keep checkmarks current.

use objc2::{define_class, msg_send, sel, ClassType, MainThreadOnly};
use objc2::rc::Retained;
use objc2_app_kit::{NSMenu, NSMenuItem, NSStatusBar, NSStatusBarButton, NSStatusItem, NSVariableStatusItemLength};
use objc2_foundation::{MainThreadMarker, NSObject, NSObjectProtocol, NSString, NSUserDefaults};

use crate::cocoa_util::*;
use crate::tickeys::{AudioScheme, Tickeys};

static mut MENU_TICKEYS: *mut Tickeys = core::ptr::null_mut();
// Store the NSStatusItem as a raw pointer so rebuild() can update its menu
// without creating duplicate items.
static mut MENU_ITEM: *mut NSStatusItem = core::ptr::null_mut();

// Volume tags: 0=25%, 1=50%, 2=75%, 3=100%
const VOL_VALUES: [f32; 4] = [0.25, 0.5, 0.75, 1.0];
const PITCH_VALUES: [f32; 5] = [0.5, 0.75, 1.0, 1.5, 2.0];

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
            if idx >= VOL_VALUES.len() { return; }
            let vol = VOL_VALUES[idx];

            unsafe { if !MENU_TICKEYS.is_null() { (*MENU_TICKEYS).set_volume(vol); } }
            save_float("audio_volume", vol);
            let schemes = load_schemes();
            rebuild(self, &schemes, self.mtm());
        }

        #[unsafe(method(setPitch:))]
        fn set_pitch(&self, sender: &NSMenuItem) {
            let idx = sender.tag() as usize;
            if idx >= PITCH_VALUES.len() { return; }
            let pitch = PITCH_VALUES[idx];

            unsafe { if !MENU_TICKEYS.is_null() { (*MENU_TICKEYS).set_pitch(pitch); } }
            save_float("audio_pitch", pitch);
            let schemes = load_schemes();
            rebuild(self, &schemes, self.mtm());
        }
    }
);

/// Rebuild the menu from scratch, setting checkmarks on the current selection.
fn rebuild(handler: &MenuHandler, schemes: &[AudioScheme], mtm: MainThreadMarker) {
    let pref_scheme = load_pref_scheme(schemes);
    let pref_vol = load_pref_float("audio_volume");
    let pref_pitch = load_pref_float("audio_pitch");

    unsafe {
        if MENU_ITEM.is_null() { return; }
        let item: &NSStatusItem = &*MENU_ITEM;

        let menu = NSMenu::initWithTitle(NSMenu::alloc(mtm), &NSString::from_str(""));

        // Scheme submenu
        let si = NSMenuItem::initWithTitle_action_keyEquivalent(
            NSMenuItem::alloc(mtm),
            &NSString::from_str("Sound Scheme"), None, &NSString::from_str(""),
        );
        let sm = NSMenu::initWithTitle(NSMenu::alloc(mtm), &NSString::from_str(""));
        for (i, scheme) in schemes.iter().enumerate() {
            let cm = if scheme.name == pref_scheme { "\u{2713} " } else { "  " };
            let title = format!("{}{}", cm, scheme.display_name);
            let mi = NSMenuItem::initWithTitle_action_keyEquivalent(
                NSMenuItem::alloc(mtm), &NSString::from_str(&title),
                Some(sel!(changeScheme:)), &NSString::from_str(""),
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
            &NSString::from_str("Volume"), None, &NSString::from_str(""),
        );
        let vm = NSMenu::initWithTitle(NSMenu::alloc(mtm), &NSString::from_str(""));
        for (i, (label, v)) in [("25%", 0.25f32), ("50%", 0.5), ("75%", 0.75), ("100%", 1.0)].iter().enumerate() {
            let cm = if (*v - pref_vol).abs() < 0.01 { "\u{2713} " } else { "  " };
            let title = format!("{}{}", cm, label);
            let mi = NSMenuItem::initWithTitle_action_keyEquivalent(
                NSMenuItem::alloc(mtm), &NSString::from_str(&title),
                Some(sel!(setVolume:)), &NSString::from_str(""),
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
            &NSString::from_str("Pitch"), None, &NSString::from_str(""),
        );
        let pm = NSMenu::initWithTitle(NSMenu::alloc(mtm), &NSString::from_str(""));
        for (i, (label, p)) in [("0.5x", 0.5f32), ("0.75x", 0.75), ("1.0x", 1.0), ("1.5x", 1.5), ("2.0x", 2.0)].iter().enumerate() {
            let cm = if (*p - pref_pitch).abs() < 0.01 { "\u{2713} " } else { "  " };
            let title = format!("{}{}", cm, label);
            let mi = NSMenuItem::initWithTitle_action_keyEquivalent(
                NSMenuItem::alloc(mtm), &NSString::from_str(&title),
                Some(sel!(setPitch:)), &NSString::from_str(""),
            );
            mi.setTag(i as isize);
            mi.setTarget(Some(handler));
            pm.addItem(&mi);
        }
        pi.setSubmenu(Some(&pm));
        menu.addItem(&pi);

        // Quit
        menu.addItem(&NSMenuItem::separatorItem(mtm));
        let q = NSMenuItem::initWithTitle_action_keyEquivalent(
            NSMenuItem::alloc(mtm),
            &NSString::from_str("Quit Tickeys"),
            Some(sel!(terminate:)), &NSString::from_str("q"),
        );
        menu.addItem(&q);

        item.setMenu(Some(&menu));

        std::mem::forget(sm);
        std::mem::forget(vm);
        std::mem::forget(pm);
        // Note: item lives forever via MENU_ITEM, don't forget it here
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn save_string(key: &str, val: &str) {
    let ud = unsafe { NSUserDefaults::standardUserDefaults() };
    let k = NSString::from_str(key);
    let v = NSString::from_str(val);
    unsafe { let _: () = msg_send![&ud, setObject: &*v forKey: &*k]; }
}

fn save_float(key: &str, val: f32) {
    let ud = unsafe { NSUserDefaults::standardUserDefaults() };
    let k = NSString::from_str(key);
    unsafe { let _: () = msg_send![&ud, setFloat: val forKey: &*k]; }
}

// ── Public API ───────────────────────────────────────────────────────────────

pub fn setup_menu(mtm: MainThreadMarker, tickeys_ptr: *mut Tickeys) {
    unsafe { MENU_TICKEYS = tickeys_ptr; }

    let schemes = load_schemes();
    let handler: Retained<MenuHandler> = unsafe { msg_send![MenuHandler::alloc(mtm), init] };

    // Create the status bar item once and store it.
    // rebuild() will update its menu on subsequent calls.
    unsafe {
        let status_bar = NSStatusBar::systemStatusBar();
        let item: Retained<NSStatusItem> = msg_send![&status_bar, statusItemWithLength: NSVariableStatusItemLength];
        let button = item.button(mtm).expect("must have button");
        button.setTitle(&NSString::from_str("\u{1F3B9}"));
        let raw = &*item as *const NSStatusItem as *mut NSStatusItem;
        MENU_ITEM = raw;
        std::mem::forget(item);
    }

    rebuild(&handler, &schemes, mtm);
    std::mem::forget(handler);
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
        msg_send![&ud, floatForKey: &*k]
    }
}
