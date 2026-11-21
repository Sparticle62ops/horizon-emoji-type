//! java: PackLoader.java, EmojiPack.java with its BundledPack and LocalPack
//! types, DefaultPackGenerator.java and PackType.java.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::emoji::EmojiCode;

/// java: enum PackType — bundled (shipped with the mod) or local (a user
/// file in the packs folder).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PackType {
    Bundled,
    /// java: PackType.fromId falls back to local, and the default follows.
    #[default]
    Local,
}

impl PackType {
    /// java: PackType.fromId — unknown, null or empty fall back to local.
    pub fn from_id(id: &str) -> Self {
        match id {
            "bundled" => PackType::Bundled,
            _ => PackType::Local,
        }
    }

    pub fn id(&self) -> &'static str {
        match self {
            PackType::Bundled => "bundled",
            PackType::Local => "local",
        }
    }
}

/// java: EmojiPack — the yaml's name, description and type, its emojis in
/// file order, and the file it loaded from.
#[derive(Clone, Debug, Default)]
pub struct Pack {
    pub name: String,
    pub description: String,
    pub pack_type: PackType,
    pub emojis: Vec<(String, String)>,
    pub file_name: String,
}

impl Pack {
    /// java: rebuildEmojiCodes — the (code, emoji) pairs in map order.
    pub fn codes(&self) -> Vec<EmojiCode> {
        self.emojis
            .iter()
            .map(|(code, emoji)| EmojiCode {
                code: code.clone(),
                emoji: emoji.clone(),
            })
            .collect()
    }

    /// java: EmojiPack.toYaml — the header lines, then every emoji entry.
    pub fn to_yaml(&self) -> String {
        let mut out = format!(
            "name: {}\ndescription: {}\ntype: {}\nemojis:\n",
            self.name,
            self.description,
            self.pack_type.id()
        );
        for (code, emoji) in &self.emojis {
            out.push_str(&format!(
                "  {}: \"{}\"\n",
                code,
                emoji.replace('\\', "\\\\").replace('"', "\\\"")
            ));
        }
        out
    }
}

/// java: the LinkedHashMap<fileName, EmojiPack>; BTreeMap keeps a
/// deterministic order for the enabled-packs fill, which is the only place
/// the map's order matters.
#[derive(Debug, Default)]
pub struct Packs(pub BTreeMap<String, Pack>);

/// java: PackLoader.PACKS_DIRECTORY.
pub fn packs_directory(base: &Path) -> PathBuf {
    base.join("emoji_packs")
}

/// java: PackLoader.loadAllPacks — generate the bundled defaults, then every
/// `*.yaml`/`*.yml` file in the packs folder becomes a pack keyed by file
/// name. A file that yields no emojis is skipped, like the original's
/// null-map guard; the name falls back to the file name without its
/// extension and the type to local, also as there.
pub fn load_all_packs(base: &Path) -> Packs {
    generate_default_packs(base);
    let mut packs = Packs::default();
    let dir = packs_directory(base);
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return packs;
    };
    let mut names: Vec<String> = entries
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.file_name().to_string_lossy().to_string())
        .filter(|name| name.ends_with(".yaml") || name.ends_with(".yml"))
        .collect();
    names.sort();
    for name in names {
        let Ok(content) = std::fs::read_to_string(dir.join(&name)) else {
            continue;
        };
        let mut pack = parse(&content);
        if pack.emojis.is_empty() {
            continue;
        }
        if pack.name.is_empty() {
            pack.name = name
                .strip_suffix(".yaml")
                .or_else(|| name.strip_suffix(".yml"))
                .unwrap_or(&name)
                .to_string();
        }
        if pack.pack_type == PackType::Local && content.contains("type: bundled") {
            pack.pack_type = PackType::Bundled;
        }
        pack.file_name = name.clone();
        packs.0.insert(name, pack);
    }
    packs
}

/// java: PackLoader.loadPackFromPath — the header lines, then the emojis
/// section's `code: "emoji"` entries. Quoted values lose their quotes;
/// comment lines are skipped.
pub fn parse(content: &str) -> Pack {
    let mut pack = Pack::default();
    let mut in_emojis = false;
    for line in content.lines() {
        if !in_emojis {
            if line.trim_start().starts_with('#') || line.trim().is_empty() {
                continue;
            }
            if line.starts_with("emojis:") {
                in_emojis = true;
                continue;
            }
            if let Some((key, value)) = split_field(line) {
                match key.as_str() {
                    "name" => pack.name = value,
                    "description" => pack.description = value,
                    "type" => pack.pack_type = PackType::from_id(&value),
                    _ => {}
                }
            }
        } else if let Some((code, emoji)) = split_entry(line) {
            pack.emojis.push((code, emoji));
        }
    }
    pack
}

fn split_field(line: &str) -> Option<(String, String)> {
    if line.starts_with(' ') || line.starts_with('\t') {
        return None;
    }
    let (key, value) = line.split_once(':')?;
    Some((key.trim().to_string(), clean(value)))
}

fn split_entry(line: &str) -> Option<(String, String)> {
    let trimmed = line.trim_start();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return None;
    }
    let (key, value) = trimmed.split_once(':')?;
    Some((key.trim().to_string(), clean(value)))
}

/// java: snakeyaml's scalar unquoting, for the quote forms these files use.
fn clean(value: &str) -> String {
    let value = value.trim();
    if value.len() >= 2
        && ((value.starts_with('"') && value.ends_with('"'))
            || (value.starts_with('\'') && value.ends_with('\'')))
    {
        value[1..value.len() - 1].to_string()
    } else {
        value.to_string()
    }
}

/// java: DefaultPackGenerator with BundledPack.copyBundledPackToLocal — the
/// three packs shipped in the mod's assets, copied into the packs folder
/// when the file is not there yet; never overwritten.
pub const BUNDLED_PACKS: &[(&str, &str)] = &[
    ("letters.yaml", include_str!("emoji_packs/letters.yaml")),
    ("minecraft.yaml", include_str!("emoji_packs/minecraft.yaml")),
    (
        "text_art_faces.yaml",
        include_str!("emoji_packs/text_art_faces.yaml"),
    ),
];

pub fn generate_default_packs(base: &Path) {
    for (name, content) in BUNDLED_PACKS {
        let target = packs_directory(base).join(name);
        if !target.exists() {
            if std::fs::create_dir_all(packs_directory(base)).is_ok() {
                let _ = std::fs::write(&target, content);
            }
        }
    }
}

/// java: PackLoader.savePack — writes the yaml into the packs folder.
pub fn save_pack(base: &Path, pack: &Pack, file_name: &str) -> bool {
    let dir = packs_directory(base);
    if std::fs::create_dir_all(&dir).is_err() {
        return false;
    }
    std::fs::write(dir.join(file_name), pack.to_yaml()).is_ok()
}

/// java: PackLoader.deletePack — true only when the file existed and went.
pub fn delete_pack(base: &Path, file_name: &str) -> bool {
    std::fs::remove_file(packs_directory(base).join(file_name)).is_ok()
}
