//! java: EmojiCode.java and the active-codes logic in EmojiType.java
//! (reloadActiveEmojiCodes).

/// java: record EmojiCode(String code, String emoji).
#[derive(Clone, Debug, PartialEq)]
pub struct EmojiCode {
    pub code: String,
    pub emoji: String,
}

/// java: reloadActiveEmojiCodes — the codes of every enabled pack, in pack
/// order, dropping an entry whose code or emoji is empty.
pub fn reload_active(config: &config::Config, packs: &packs::Packs) -> Vec<EmojiCode> {
    let mut codes = Vec::new();
    for file_name in &config.enabled_packs {
        if let Some(pack) = packs.0.get(file_name) {
            for (code, emoji) in &pack.emojis {
                if code.is_empty() || emoji.is_empty() {
                    continue;
                }
                codes.push(EmojiCode {
                    code: code.clone(),
                    emoji: emoji.clone(),
                });
            }
        }
    }
    codes
}

/// java: the send-time equivalent of the original's in-field replacement:
/// every active `:shortcode:` in the message becomes its emoji, in pack
/// order. A message with no match comes back unchanged.
pub fn transform(message: &str, codes: &[EmojiCode]) -> String {
    let mut out = message.to_string();
    for entry in codes {
        let token = format!(":{}:", entry.code);
        if out.contains(&token) {
            out = out.replace(&token, &entry.emoji);
        }
    }
    out
}

use crate::{config, packs};
