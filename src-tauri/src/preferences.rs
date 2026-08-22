use crate::models::LaunchConfig;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs, path::PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct LearnedProfile {
    pub(crate) max_size: u32,
    pub(crate) max_fps: u32,
    pub(crate) codec: String,
    pub(crate) audio: bool,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct ProfileStore {
    #[serde(default)]
    profiles: HashMap<String, LearnedProfile>,
}

fn profile_key(serial: &str, mode: &str) -> String {
    format!("{serial}\u{1f}{mode}")
}

fn store_path() -> Result<PathBuf, String> {
    let base = dirs::config_local_dir()
        .or_else(dirs::data_local_dir)
        .or_else(dirs::home_dir)
        .ok_or_else(|| "Could not locate a local settings directory.".to_string())?;
    let folder = base.join("SCRCPY Studio");
    fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
    Ok(folder.join("learned-profiles.json"))
}

fn read_store() -> Result<ProfileStore, String> {
    let path = store_path()?;
    if !path.exists() {
        return Ok(ProfileStore::default());
    }
    let raw = fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_json::from_str(&raw).map_err(|e| format!("Could not read learned profiles: {e}"))
}

pub(crate) fn load_learned_profile(serial: &str, mode: &str) -> Option<LearnedProfile> {
    let store = read_store().ok()?;
    store.profiles.get(&profile_key(serial, mode)).cloned()
}

pub(crate) fn remember_successful_profile(config: &LaunchConfig) -> Result<(), String> {
    let mut store = read_store().unwrap_or_default();
    store.profiles.insert(
        profile_key(&config.serial, &config.mode),
        LearnedProfile {
            max_size: config.max_size,
            max_fps: config.max_fps,
            codec: config.codec.clone(),
            audio: config.audio,
        },
    );

    let path = store_path()?;
    let json = serde_json::to_string_pretty(&store).map_err(|e| e.to_string())?;
    fs::write(path, json).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn learned_profile_json_round_trip() {
        let profile = LearnedProfile {
            max_size: 1920,
            max_fps: 60,
            codec: "h265".into(),
            audio: true,
        };
        let json = serde_json::to_string(&profile).unwrap();
        let decoded: LearnedProfile = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, profile);
    }

    #[test]
    fn device_and_mode_form_distinct_keys() {
        assert_ne!(profile_key("ABC", "mirror"), profile_key("ABC", "creator"));
        assert_ne!(profile_key("ABC", "mirror"), profile_key("XYZ", "mirror"));
    }
}
