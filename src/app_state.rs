//! Main-thread settings shared by the menu and preferences window.

use objc2_foundation::MainThreadMarker;
use std::cell::RefCell;
use std::rc::Rc;

use crate::cocoa_util::get_res_path;
use crate::pref::{Pref, normalize_volume_percent};
use crate::tickeys::{AudioController, AudioScheme, AudioStatus, Tickeys, normalize_pitch};

struct AppState {
    pref: Pref,
    schemes: Rc<[AudioScheme]>,
    tickeys: Tickeys,
}

thread_local! {
    static STATE: RefCell<Option<AppState>> = const { RefCell::new(None) };
}

pub fn initialize(schemes: Vec<AudioScheme>, audio: AudioController) {
    let pref = Pref::load(&schemes);
    let mut tickeys = Tickeys::new(schemes.clone(), audio);
    tickeys.set_volume(pref.volume / 100.0);
    tickeys.set_pitch(pref.pitch);
    tickeys
        .load_scheme(
            &get_res_path(&format!("data/{}", pref.scheme)),
            &pref.scheme,
        )
        .expect("bundled scheme must have a valid mapping");
    STATE.with(|state| {
        *state.borrow_mut() = Some(AppState {
            pref,
            schemes: schemes.into(),
            tickeys,
        })
    });
}

pub fn shutdown() {
    STATE.with(|state| {
        state.borrow_mut().take();
    });
}

pub fn preferences() -> Pref {
    STATE.with(|state| {
        state
            .borrow()
            .as_ref()
            .expect("initialized settings")
            .pref
            .clone()
    })
}

pub fn schemes() -> Rc<[AudioScheme]> {
    STATE.with(|state| {
        state
            .borrow()
            .as_ref()
            .expect("initialized settings")
            .schemes
            .clone()
    })
}

pub fn handle_keydown(keycode: u8) {
    STATE.with(|state| {
        if let Some(state) = state.borrow().as_ref() {
            state.tickeys.handle_keydown(keycode);
        }
    });
}

pub fn test_sound() {
    STATE.with(|state| {
        if let Some(state) = state.borrow().as_ref() {
            state.tickeys.test_sound();
        }
    });
}

pub fn audio_status() -> Option<AudioStatus> {
    STATE.with(|state| {
        state
            .borrow()
            .as_ref()
            .map(|state| state.tickeys.audio_status())
    })
}

fn update(mtm: MainThreadMarker, change: impl FnOnce(&mut AppState)) {
    STATE.with(|state| {
        if let Some(state) = state.borrow_mut().as_mut() {
            change(state);
            state.pref.save();
        }
    });
    // Release the state borrow before calling AppKit: both surfaces read it.
    crate::settings_ui::refresh(mtm);
    crate::settings_window::refresh(mtm);
}

pub fn set_scheme(mtm: MainThreadMarker, index: usize) {
    update(mtm, |state| {
        if let Some(scheme) = state.schemes.get(index) {
            let directory = get_res_path(&format!("data/{}", scheme.name));
            match state.tickeys.load_scheme(&directory, &scheme.name) {
                Ok(()) => state.pref.scheme = scheme.name.clone(),
                Err(error) => eprintln!("could not select scheme: {error}"),
            }
        }
    });
}

pub fn set_volume(mtm: MainThreadMarker, percent: f32) {
    update(mtm, |state| {
        state.pref.volume = normalize_volume_percent(percent);
        state.tickeys.set_volume(state.pref.volume / 100.0);
    });
}

pub fn set_pitch(mtm: MainThreadMarker, pitch: f32) {
    update(mtm, |state| {
        state.pref.pitch = normalize_pitch(pitch);
        state.tickeys.set_pitch(state.pref.pitch);
    });
}
