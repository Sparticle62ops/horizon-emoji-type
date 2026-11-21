//! Tests for the packs, config and transform layers, each one standing in
//! for the original's behavior.

use std::path::PathBuf;

use crate::{config, emoji, packs};

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "emoji_type_test_{}_{}",
        tag,
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn bundled_packs_parse_and_generate_idempotently() {
    for (name, content) in packs::BUNDLED_PACKS {
        let pack = packs::parse(content);
        assert!(!pack.emojis.is_empty(), "{} has emojis", name);
        assert_eq!(pack.pack_type, packs::PackType::Bundled, "{} is bundled", name);
        assert!(!pack.name.is_empty(), "{} has a name", name);
    }
    let base = temp_dir("generate");
    packs::generate_default_packs(&base);
    let first = packs::load_all_packs(&base);
    assert_eq!(first.0.len(), 3, "three bundled packs load");
    // java: copyBundledPackToLocal never overwrites
    let letters = base.join("emoji_packs").join("letters.yaml");
    std::fs::write(&letters, "name: user edit\nemojis:\n  a: \"A\"\n").unwrap();
    packs::generate_default_packs(&base);
    let again = packs::load_all_packs(&base);
    assert_eq!(
        again.0.get("letters.yaml").unwrap().name, "user edit",
        "an edited bundled pack is not overwritten"
    );
}

#[test]
fn parse_reads_header_and_emoji_entries_in_order() {
    let content = "name: Test\ndescription: A test\ntype: local\nemojis:\n  # note\n  smile: \"😀\"\n  frown: '🙁'\n  wink: 🙂\n";
    let pack = packs::parse(content);
    assert_eq!(pack.name, "Test");
    assert_eq!(pack.description, "A test");
    assert_eq!(pack.pack_type, packs::PackType::Local);
    assert_eq!(
        pack.emojis,
        vec![
            ("smile".to_string(), "😀".to_string()),
            ("frown".to_string(), "🙁".to_string()),
            ("wink".to_string(), "🙂".to_string()),
        ]
    );
    // java: PackType.fromId — unknown falls back to local
    assert_eq!(packs::PackType::from_id("nonsense"), packs::PackType::Local);
    assert_eq!(packs::PackType::from_id(""), packs::PackType::Local);
}

#[test]
fn transform_replaces_active_codes_in_pack_order() {
    let mut packs = packs::Packs::default();
    packs.0.insert(
        "a.yaml".to_string(),
        packs::Pack {
            name: "A".into(),
            emojis: vec![
                ("smile".into(), "😀".into()),
                ("heart".into(), "❤".into()),
            ],
            pack_type: packs::PackType::Bundled,
            file_name: "a.yaml".into(),
            ..Default::default()
        },
    );
    packs.0.insert(
        "b.yaml".to_string(),
        packs::Pack {
            name: "B".into(),
            emojis: vec![("heart".into(), "💜".into())],
            pack_type: packs::PackType::Local,
            file_name: "b.yaml".into(),
            ..Default::default()
        },
    );
    let mut config = config::Config::default();
    config.enabled_packs = vec!["a.yaml".to_string()];
    let codes = emoji::reload_active(&config, &packs);
    assert_eq!(codes.len(), 2, "disabled packs contribute nothing");
    assert_eq!(
        emoji::transform("hi :smile: :heart: :none:", &codes),
        "hi 😀 ❤ :none:"
    );
    config.enabled_packs = vec!["b.yaml".to_string()];
    let codes = emoji::reload_active(&config, &packs);
    assert_eq!(emoji::transform(":heart:", &codes), "💜", "later packs win");
    // java: reloadActiveEmojiCodes drops an entry with an empty side
    packs.0.insert(
        "c.yaml".to_string(),
        packs::Pack {
            name: "C".into(),
            emojis: vec![("empty".into(), String::new()), ("x".into(), "X".into())],
            file_name: "c.yaml".into(),
            ..Default::default()
        },
    );
    config.enabled_packs = vec!["c.yaml".to_string()];
    let codes = emoji::reload_active(&config, &packs);
    assert_eq!(codes.len(), 1);
}

#[test]
fn config_round_trips_and_defaults() {
    let base = temp_dir("config");
    let legacy = base.join("emojitype.json");
    // java: no config file and no legacy file — the default config
    let config = config::load(&base, &legacy);
    assert!(config.enabled_packs.is_empty());
    assert_eq!(config.version, 1);
    let mut config = config::Config::default();
    config.enabled_packs = vec!["minecraft.yaml".to_string()];
    assert!(config::save(&base, &config));
    let reloaded = config::load(&base, &legacy);
    assert_eq!(reloaded.enabled_packs, vec!["minecraft.yaml".to_string()]);
    // java: an unparsable config becomes the default
    std::fs::write(config::config_file(&base), "not json").unwrap();
    assert!(config::load(&base, &legacy).enabled_packs.is_empty());
}

#[test]
fn legacy_migration_writes_the_legacy_pack() {
    let base = temp_dir("legacy");
    let legacy = base.join("emojitype.json");
    // java: an empty legacy list becomes the default config, the file is
    // backed up and removed
    std::fs::write(&legacy, "[]").unwrap();
    let config = config::load(&base, &legacy);
    assert!(config.enabled_packs.is_empty());
    assert!(!legacy.exists(), "the legacy file is removed");
    assert!(base.join("emojitype.legacy.json").exists(), "backup written");
    // java: custom entries become a "Legacy Emojis" pack, the only enabled
    // one (a fresh base: the legacy file only migrates when config.json is
    // not there yet, like the original)
    let base = temp_dir("legacy_pack");
    let legacy = base.join("emojitype.json");
    std::fs::write(&legacy, "[\"smile;😀\", \"wave;👋\", \"bad-entry\"]").unwrap();
    let config = config::load(&base, &legacy);
    assert_eq!(config.enabled_packs, vec!["legacy_emojis.yaml".to_string()]);
    let pack = packs::load_all_packs(&base).0.remove("legacy_emojis.yaml").unwrap();
    assert_eq!(pack.name, "Legacy Emojis");
    assert_eq!(pack.description, "Your custom emojis migrated from EmojiType v2");
    assert_eq!(
        pack.emojis,
        vec![
            ("smile".to_string(), "😀".to_string()),
            ("wave".to_string(), "👋".to_string()),
        ]
    );
    // java: the round trip — the pack file reloads the same emojis
    let yaml = packs::parse(&pack.to_yaml());
    assert_eq!(yaml.emojis, pack.emojis);
}
