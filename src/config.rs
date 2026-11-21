//! java: ConfigManager.java and Config.java, including the v2 legacy
//! migration.

use std::path::Path;

use crate::packs;

/// java: class Config — enabledPacks keeps the file's order (the original
/// uses a LinkedHashSet) and version is always 1.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Config {
    #[serde(rename = "enabledPacks", default)]
    pub enabled_packs: Vec<String>,
    #[serde(default = "default_version")]
    pub version: i32,
}

fn default_version() -> i32 {
    1
}

impl Default for Config {
    fn default() -> Self {
        Config {
            enabled_packs: Vec::new(),
            version: 1,
        }
    }
}

/// java: ConfigManager.CONFIG_FILE — config.json under the mod's config
/// base; the port uses the mod id (config/emoji_type/) where the original
/// used config/emojitype/.
pub fn config_file(base: &Path) -> std::path::PathBuf {
    base.join("config.json")
}

/// java: ConfigManager.load — the legacy file migrates when the new config
/// does not exist yet; a config that fails to parse becomes the default.
pub fn load(base: &Path, legacy_path: &Path) -> Config {
    if legacy_path.exists() && !config_file(base).exists() {
        migrate_from_old_location(base, legacy_path);
        return load(base, legacy_path);
    }
    match std::fs::read_to_string(config_file(base)) {
        Ok(json) => serde_json::from_str::<Config>(&json).unwrap_or_default(),
        Err(_) => Config::default(),
    }
}

/// java: ConfigManager.save — true when the file wrote.
pub fn save(base: &Path, config: &Config) -> bool {
    let path = config_file(base);
    if let Some(parent) = path.parent() {
        if std::fs::create_dir_all(parent).is_err() {
            return false;
        }
    }
    match serde_json::to_string_pretty(config) {
        Ok(json) => std::fs::write(&path, json).is_ok(),
        Err(_) => false,
    }
}

/// java: KNOWN_DEFAULT_CONFIG_HASHES — the three md5 hashes of default v2
/// configs, safe to replace with the defaults.
const KNOWN_DEFAULT_CONFIG_HASHES: [&str; 3] = [
    "58e0494c51d30eb3494f7c9198986bb9",
    "d06f855de8fc0fe01eec5c4f104ec177",
    "3395d53b28f43b4f368a6a86705df98c",
];

fn md5_hex(input: &str) -> String {
    use md5::{Digest, Md5};
    let mut hasher = Md5::new();
    hasher.update(input.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// java: migrateFromOldLocation — a hash that matches a known default
/// migrates to the defaults; anything else becomes a legacy pack. The old
/// file is backed up and removed either way.
fn migrate_from_old_location(base: &Path, legacy_path: &Path) {
    let json = std::fs::read_to_string(legacy_path).unwrap_or_default();
    let migrated = if KNOWN_DEFAULT_CONFIG_HASHES.contains(&md5_hex(&json).as_str()) {
        Config::default()
    } else {
        migrate_very_old_legacy_config(base, &json)
    };
    save(base, &migrated);
    backup_legacy_config(legacy_path);
}

/// java: migrateVeryOldLegacyConfig — the old config is a list of
/// `code;emoji` entries; they become a "Legacy Emojis" local pack that is
/// the only enabled pack. An empty or unparsable list falls back to the
/// defaults.
fn migrate_very_old_legacy_config(base: &Path, json: &str) -> Config {
    let entries: Vec<String> = match serde_json::from_str(json) {
        Ok(entries) => entries,
        Err(_) => return Config::default(),
    };
    if entries.is_empty() {
        return Config::default();
    }
    let mut emojis = Vec::new();
    for entry in entries {
        if let Some((code, emoji)) = entry.split_once(';') {
            emojis.push((code.to_string(), emoji.to_string()));
        }
    }
    create_legacy_pack(base, emojis)
}

/// java: createLegacyPack — a "Legacy Emojis" local pack saved as
/// legacy_emojis.yaml, every default pack disabled and the legacy pack the
/// only one enabled. A failed pack or config write rolls back to defaults.
fn create_legacy_pack(base: &Path, emojis: Vec<(String, String)>) -> Config {
    let mut config = Config::default();
    let legacy = packs::Pack {
        name: "Legacy Emojis".to_string(),
        description: "Your custom emojis migrated from EmojiType v2".to_string(),
        pack_type: packs::PackType::Local,
        emojis,
        file_name: String::new(),
    };
    let file_name = "legacy_emojis.yaml";
    if packs::save_pack(base, &legacy, file_name) {
        config.enabled_packs.push(file_name.to_string());
        if save(base, &config) {
            return config;
        }
        // java: the config save failed, delete the pack we just wrote
        let _ = packs::delete_pack(base, file_name);
    }
    Config::default()
}

/// java: backupLegacyConfig — the old config copies to emojitype.legacy.json
/// once, then the original goes.
fn backup_legacy_config(legacy_path: &Path) {
    let backup = legacy_path.with_extension("legacy.json");
    if legacy_path.exists() && !backup.exists() {
        let _ = std::fs::copy(legacy_path, &backup);
    }
    let _ = std::fs::remove_file(legacy_path);
}
