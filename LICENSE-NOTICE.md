# Licence notice

This port is a clean-room Rust rewrite of **Transfer Labels 0.1.9** by Buuz135
(`com.buuz135.transfer_labels`), read from the pack jar
`ports/_shared/licence-cache/transfer_labels-0.1.9.jar` and the public
repository <https://github.com/Buuz135/Transfer-Labels>.

- **Licence:** MIT.
- **Read from:** the jar's `META-INF/neoforge.mods.toml` —
  `license = "MIT"`, `modId = "transfer_labels"`, `version = "0.1.9"`,
  `authors = "Buuz135"`. The jar carries no other `LICENSE`/`NOTICE` file and no
  separate assets licence, so the item texture, model, lang strings and recipe
  are MIT as well.
- **Rank:** 0 (permissive). Local-only until the maintainer approves publishing
  (campaign ARCHITECTURE.md §8); publishing keeps this notice and the original
  copyright.
- **Carried from upstream:** the item texture
  `assets/transfer_labels/textures/item/itemstack_extract_transfer_label.png`
  and the model/lang/recipe data, re-expressed in the 26.3 formats.
- **Not carried:** any Java source. The behaviour was reimplemented in Rust from
  reading the public repository (recorded in `FINDINGS.md`).
