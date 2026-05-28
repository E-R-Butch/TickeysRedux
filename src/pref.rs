//! User preferences — NSUserDefaults via objc2.

use objc2::msg_send;
use objc2::rc::Retained;
use objc2_foundation::{NSUserDefaults, NSString};
use crate::tickeys::AudioScheme;

pub struct Pref {
    pub scheme: String,
    pub volume: f32,
    pub pitch: f32,
}

impl Pref {
    pub fn load(schemes: &Vec<AudioScheme>) -> Pref {
        unsafe {
            let ud = NSUserDefaults::standardUserDefaults();
            let pref_exists_key = NSString::from_str("pref_exists");

            let pref = Pref {
                scheme: schemes[0].name.clone(),
                volume: 0.5,
                pitch: 1.0,
            };

            let pref_exists: Option<Retained<NSString>> =
                msg_send![&ud, stringForKey: &*pref_exists_key];

            if pref_exists.is_none() {
                // First run
                pref.save();
                return pref;
            }

            let audio_scheme_key = NSString::from_str("audio_scheme");
            let volume_key = NSString::from_str("volume");
            let pitch_key = NSString::from_str("pitch");

            let audio_scheme: Option<Retained<NSString>> =
                msg_send![&ud, stringForKey: &*audio_scheme_key];
            let volume: f32 = msg_send![&ud, floatForKey: &*volume_key];
            let pitch: f32 = msg_send![&ud, floatForKey: &*pitch_key];

            let mut scheme_str = audio_scheme
                .map(|s| s.to_string())
                .unwrap_or_default();

            // Validate scheme
            if !schemes.iter().any(|s| s.name == scheme_str) {
                scheme_str = pref.scheme.clone();
            }

            Pref {
                scheme: scheme_str,
                volume,
                pitch,
            }
        }
    }

    pub fn save(&self) {
        unsafe {
            let ud = NSUserDefaults::standardUserDefaults();

            let audio_scheme_key = NSString::from_str("audio_scheme");
            let volume_key = NSString::from_str("volume");
            let pitch_key = NSString::from_str("pitch");
            let pref_exists_key = NSString::from_str("pref_exists");

            let _: () = msg_send![
                &ud,
                setObject: &*NSString::from_str(&self.scheme),
                forKey: &*audio_scheme_key
            ];
            let _: () = msg_send![&ud, setFloat: self.volume, forKey: &*volume_key];
            let _: () = msg_send![&ud, setFloat: self.pitch, forKey: &*pitch_key];
            let _: () = msg_send![&ud, setObject: &*pref_exists_key, forKey: &*pref_exists_key];
            let _: bool = msg_send![&ud, synchronize];
        }
    }
}
