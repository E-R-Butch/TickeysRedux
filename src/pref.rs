//! User preferences — NSUserDefaults via objc2.

use crate::tickeys::AudioScheme;
use objc2::msg_send;
use objc2::rc::Retained;
use objc2_foundation::{NSString, NSUserDefaults};

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
            let (volume, volume_changed) = normalize_volume_percent(stored_volume);
            if volume_changed {
                let _: () = msg_send![&ud, setDouble: volume as f64, forKey: &*volume_key];
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

            if volume_changed {
                let _: bool = msg_send![&ud, synchronize];
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

            let _: () = msg_send![&ud, setObject: &*pref_exists_key, forKey: &*pref_exists_key];
            let _: bool = msg_send![&ud, synchronize];
        }
    }
}

/// Preferences use percentage points (0..=100). Versions up to 1.0.6 could
/// save menu selections as 0..=1, so migrate those values on load.
pub(crate) fn normalize_volume_percent(raw: f32) -> (f32, bool) {
    if !raw.is_finite() {
        return (50.0, true);
    }

    let migrated = [0.25_f32, 0.5, 0.75, 1.0]
        .iter()
        .any(|legacy| (raw - legacy).abs() < 0.0001);
    let converted = if migrated { raw * 100.0 } else { raw };
    let normalized = converted.clamp(0.0, 100.0);
    let changed = migrated || (normalized - raw).abs() > f32::EPSILON;
    (normalized, changed)
}

#[cfg(test)]
mod tests {
    use super::normalize_volume_percent;

    #[test]
    fn migrates_legacy_fractional_volume_values() {
        assert_eq!(normalize_volume_percent(0.25), (25.0, true));
        assert_eq!(normalize_volume_percent(0.5), (50.0, true));
        assert_eq!(normalize_volume_percent(0.75), (75.0, true));
        assert_eq!(normalize_volume_percent(1.0), (100.0, true));
    }

    #[test]
    fn preserves_percentage_values_and_clamps_bad_input() {
        assert_eq!(normalize_volume_percent(50.0), (50.0, false));
        assert_eq!(normalize_volume_percent(0.0), (0.0, false));
        assert_eq!(normalize_volume_percent(0.8), (0.8, false));
        assert_eq!(normalize_volume_percent(150.0), (100.0, true));
        assert_eq!(normalize_volume_percent(f32::NAN), (50.0, true));
    }
}
