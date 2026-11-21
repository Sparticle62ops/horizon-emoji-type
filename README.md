# Emoji Type for Horizon

An exact 1:1 Rust port of [Emoji Type](https://github.com/Norbiros/emojitype)
by Norbiros (co-author CyborgCabbage), GPL-3.0-only, carried from the
original with credit. Type a `:shortcode:` in chat and the sent message
carries its emoji instead.

## How it works

- **Packs** (`packs.rs` <- PackLoader.java, EmojiPack.java, BundledPack,
  LocalPack, DefaultPackGenerator.java, PackType.java): every `*.yaml`/
  `*.yml` file in `config/emoji_type/emoji_packs` is a pack keyed by file
  name, with the same header (`name`, `description`, `type`) and
  `code: "emoji"` entries as the original. The three bundled packs
  (letters, Minecraft symbols, text-art faces) copy into the folder on
  first run and are never overwritten.
- **Config** (`config.rs` <- ConfigManager.java, Config.java):
  `config/emoji_type/config.json` holds `enabledPacks` (in file order) and
  `version`; an empty list fills with every pack name on first load. The
  original's v2 legacy migration is included: an old `config/emojitype.json`
  with a known-default hash migrates to the defaults, anything else becomes
  a `legacy_emojis.yaml` "Legacy Emojis" pack, and the old file is backed up
  to `emojitype.legacy.json` and removed.
- **Replacement** (`emoji.rs` <- EmojiCode.java, reloadActiveEmojiCodes):
  every active `:shortcode:` in the outgoing chat message becomes its
  emoji, in pack order; entries with an empty code or emoji are dropped,
  as there.

## Wiring

The mod API runs the original's init on `client_setup` and the replacement
on `client_chat`, so it works with no other mod installed (the previous
build's replacement rode a pinned mixin that needed another mod's chat
subscriber). Slash commands keep bypassing the hook, as in the original,
where the NeoForge `ClientChatEvent` does not fire for commands.

## Seams

- The original replaces codes while you type and offers chat suggestions;
  the mod API has no edit-box text or suggestion hook, so the replacement
  happens when the message is sent instead.
- The in-game pack editor and config screen are unported; packs are edited
  in the YAML files.
