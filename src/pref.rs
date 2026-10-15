//! User preferences — NSUserDefaults via objc2.

use crate::tickeys::AudioScheme;
use objc2::msg_send;
use objc2::rc::Retained;
use objc2_foundation::{NSString, NSUserDefaults};

const AUDIO_PREF_SCHEMA_VERSION: isize = 1;

#[derive(Clone, Debug)]
pub struct Pref {
    pub scheme: String,
    pub volume: f32,
    pub pitch: f32,
}

impl Pref {
    pub fn load(schemes: &[AudioScheme]) -> Pref {
        unsafe {
            let ud = NSUserDefaults::standardUserDefaults();
            let pref_exists_key = NSString::from_str("pref_exists");

            let pref = Pref {
                scheme: schemes
                    .first()
                    .expect("at least one audio scheme is required")
                    .name
                    .clone(),
                volume: 50.0,
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
            let volume_key = NSString::from_str("audio_volume");
            let pitch_key = NSString::from_str("audio_pitch");

            let audio_scheme: Option<Retained<NSString>> =
                msg_send![&ud, stringForKey: &*audio_scheme_key];

            // Use objectForKey to detect missing keys (floatForKey returns 0 for missing)
            let vol_obj: *mut objc2::runtime::AnyObject =
                msg_send![&ud, objectForKey: &*volume_key];
            let stored_volume: f32 = if !vol_obj.is_null() {
                msg_send![&ud, floatForKey: &*volume_key]
            } else {
                pref.volume
            };
            let schema_version: isize =
                msg_send![&ud, integerForKey: &*NSString::from_str("audio_pref_schema_version")];
            // Unversioned 0.5 is ambiguous: a pre-1.0.7 fraction OR a valid
            // 0.5% slider value. Never guess by multiplying the user's volume.
            // Preserve percentage points and stamp the schema on this load.
            let volume = normalize_volume_percent(stored_volume);
            if schema_version < AUDIO_PREF_SCHEMA_VERSION {
                eprintln!("audio preferences: preserving unversioned volume as percentage points");
            }

            let pitch_obj: *mut objc2::runtime::AnyObject =
                msg_send![&ud, objectForKey: &*pitch_key];
            let pitch: f32 = if !pitch_obj.is_null() {
                msg_send![&ud, floatForKey: &*pitch_key]
            } else {
                pref.pitch
            };

            let mut scheme_str = audio_scheme.map(|s| s.to_string()).unwrap_or_default();

            // Validate scheme
            if !schemes.iter().any(|s| s.name == scheme_str) {
                scheme_str = pref.scheme.clone();
            }

            let pref = Pref {
                scheme: scheme_str,
                volume,
                pitch: crate::tickeys::normalize_pitch(pitch),
            };
            // Write the marker even if no normalization was necessary. Future
            // migrations can distinguish these canonical percentage settings.
            pref.save();
            pref
        }
    }

    pub fn save(&self) {
        unsafe {
            let ud = NSUserDefaults::standardUserDefaults();

            let audio_scheme_key = NSString::from_str("audio_scheme");
            let volume_key = NSString::from_str("audio_volume");
            let pitch_key = NSString::from_str("audio_pitch");
            let pref_exists_key = NSString::from_str("pref_exists");

            let _: () = msg_send![
                &ud,
                setObject: &*NSString::from_str(&self.scheme),
                forKey: &*audio_scheme_key
            ];

            // Use setDouble:forKey: — NSUserDefaults stores numbers as double internally
            let _: () = msg_send![&ud, setDouble: self.volume as f64, forKey: &*volume_key];
            let _: () = msg_send![&ud, setDouble: self.pitch as f64, forKey: &*pitch_key];
            let _: () = msg_send![&ud, setInteger: AUDIO_PREF_SCHEMA_VERSION, forKey: &*NSString::from_str("audio_pref_schema_version")];

            let _: () = msg_send![&ud, setObject: &*pref_exists_key, forKey: &*pref_exists_key];
        }
    }
}

/// Runtime values always use percentage points. Migration is startup-only.
pub(crate) fn normalize_volume_percent(raw: f32) -> f32 {
    if !raw.is_finite() {
        return 50.0;
    }
    raw.clamp(0.0, 100.0)
}

#[cfg(test)]
mod tests {
    use super::normalize_volume_percent;

    #[test]
    fn low_percentage_values_are_preserved_even_without_a_version_marker() {
        for raw in [0.25, 0.5, 0.75, 1.0] {
            assert_eq!(normalize_volume_percent(raw), raw);
            assert_eq!(normalize_volume_percent(normalize_volume_percent(raw)), raw);
        }
    }

    #[test]
    fn preserves_percentage_values_and_clamps_bad_input() {
        assert_eq!(normalize_volume_percent(50.0), 50.0);
        assert_eq!(normalize_volume_percent(0.0), 0.0);
        assert_eq!(normalize_volume_percent(0.8), 0.8);
        assert_eq!(normalize_volume_percent(150.0), 100.0);
        assert_eq!(normalize_volume_percent(-10.0), 0.0);
        assert_eq!(normalize_volume_percent(f32::NAN), 50.0);
    }
}
