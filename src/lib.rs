//! Exact 1:1 Rust translation of Emoji Type by Norbiros
//! (github.com/Norbiros/emojitype, GPL-3.0-only; co-author CyborgCabbage).
//! Type a `:shortcode:` in chat and the sent message carries its emoji
//! instead. The packs system is `packs` <- PackLoader.java, EmojiPack.java
//! with its BundledPack and LocalPack types, DefaultPackGenerator.java and
//! PackType.java; the config, v2 legacy migration included, is `config` <->
//! ConfigManager.java and Config.java; the codes and the replacement are
//! `emoji` <-> EmojiCode.java and reloadActiveEmojiCodes; the mod API wiring
//! is here, for EmojiType.java.
//!
//! Wired on the mod API: the client_setup hook runs the original's init
//! (config load with the legacy migration, the three bundled packs copied
//! into the packs folder, every pack loaded, the active codes built), and
//! the client_chat hook replaces every active `:code:` in the outgoing
//! message. The replacement rides its own subscriber, so it works with no
//! other mod installed (the review's follow-up on the pinned sender); slash
//! commands keep bypassing it, as in the original, where the NeoForge
//! ClientChatEvent does not fire for commands.
//!
//! Seams (upstream behaviors the mod API does not expose yet, tracked):
//! - the original replaces codes while you type and offers chat
//!   suggestions (a `:` opens the code list, a match fills the box); the
//!   mod API has no edit-box text or suggestion hook, so the replacement
//!   happens when the message is sent instead.
//! - the in-game pack editor and config screen are unported; packs are
//!   edited in the YAML files under config/emoji_type/emoji_packs.
//!
//! Licence: GPL-3.0-only, carried from the original (see LICENSE).

pub mod config;
pub mod emoji;
pub mod packs;
#[cfg(test)]
pub mod tests;

use std::cell::RefCell;
use std::path::PathBuf;

use mod_api::{ClientChatEvent, ClientHost, ClientSetupEvent, Flow};

/// java: EmojiType.CONFIG_BASE_DIR — the original keeps everything under
/// config/emojitype/; the port uses the mod id, config/emoji_type/.
pub fn game_dir() -> PathBuf {
    std::env::var("XI_GAME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
}

pub fn config_base() -> PathBuf {
    game_dir().join("config").join("emoji_type")
}

/// java: EmojiType's loaded state — the config, every pack and the active
/// codes. The game drives these hooks from one thread, like the original's
/// static fields.
#[derive(Default)]
struct State {
    config: config::Config,
    packs: packs::Packs,
    codes: Vec<emoji::EmojiCode>,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
}

fn set_state(state: State) {
    STATE.with(|cell| *cell.borrow_mut() = state);
}

#[derive(Default)]
struct Mod;

#[mod_api::horizon_mod]
impl mod_api::Hooks for Mod {
    /// java: EmojiType's init — load the config (with the v2 legacy
    /// migration), generate the bundled defaults, load every pack, fill an
    /// empty enabledPacks with every pack name and build the active codes.
    fn client_setup(&self, _host: &mut dyn ClientHost, _event: &mut ClientSetupEvent) {
        let base = config_base();
        let legacy = game_dir()
            .join("config")
            .join("emojitype.json");
        let mut state = State::default();
        state.config = config::load(&base, &legacy);
        state.packs = packs::load_all_packs(&base);
        // java: the default config fills with every available pack name on
        // first load and saves
        if state.config.enabled_packs.is_empty() {
            state.config.enabled_packs = state.packs.0.keys().cloned().collect();
            config::save(&base, &state.config);
        }
        state.codes = emoji::reload_active(&state.config, &state.packs);
        set_state(state);
    }

    /// java: the send-time equivalent of the original's in-field
    /// replacement — every active `:shortcode:` in the outgoing message
    /// becomes its emoji. The message is never cancelled; slash commands
    /// keep bypassing the hook, as in the original.
    fn client_chat(&self, _host: &mut dyn ClientHost, event: &mut ClientChatEvent) -> Flow {
        STATE.with(|cell| {
            let state = cell.borrow();
            *event.message = emoji::transform(&event.message, &state.codes);
        });
        Flow::Continue
    }
}
