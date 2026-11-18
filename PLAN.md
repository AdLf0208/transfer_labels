# Transfer Labels slice 1 — implementation plan

> **For the implementing agent:** superpowers `test-driven-development` governs
> the runtime, `verification-before-completion` governs every claim.

**Goal:** an `itemstack_extract_transfer_label` that attaches to a block face and
moves items from that block into the container across the face, on its own tick,
at the jar's rate and under the jar's filter/quantity rules.

**Architecture:** pure rules in `mixins/transfer_labels/runtime.rs` (rate, amount,
filter, regulating budget, the command grammar, the saved encoding); game glue in
`support.rs` (a level attachment holding the label set, `level_tick_pre`,
`right_click_block`, `left_click_block`, the command); the content (one item,
its model, texture, recipe, lang, creative tab) is a `.hmod` embedded in the
`.hnmod` by the AE2 pattern.

**Spec:** `FINDINGS.md` §2 (behaviour) and the crate pin
`1d63e0a5a37a7a708eb0b32b49bf0dfeb28ed8a9`.

## Global constraints

- Pin `1d63e0a5a37a7a708eb0b32b49bf0dfeb28ed8a9`, mixins
  `5a64af938ec86b00d53d16a408aabc0038fb3e57`.
- Only stamped hooks: `level_tick_pre`, `right_click_block`, `left_click_block`,
  `commands`, `register_commands`, `perform_command`, and the
  `mods::content_root` injection target used by the AE2 pattern.
- Every number comes from `Config.java` or `LabelInstance`/`ItemFilter` — 20-tick
  base interval, `1 + amount` items, `min(speed,38)/2`, `min(amount,63)`,
  `filterSlots = 20`, regulating cap 4.
- No `unwrap()` outside tests; `cargo fmt`, `cargo clippy --all-targets -D
  warnings`, `cargo test` clean.
- The port ships exactly one deliverable: `ports/transfer_labels/transfer_labels.hnmod`.

## Review focus

1. A label whose filter does not match must move **nothing**, while the same
   label with a matching filter moves — a filter that silently passes everything
   would still "work".
2. The rate: with no upgrades exactly one item per 20 game ticks; the count over
   a window must equal the number of multiples of the interval inside it.
3. Nothing is ever lost: a full target must leave the source untouched.
4. A label on a face whose neighbour has no container does nothing (and does not
   crash the tick).
5. The state survives a save and reload (level attachment codec).

---

### Task 1: the pure runtime

**Files:** Create `mixins/transfer_labels/runtime.rs` (+ `Cargo.toml` lib path).

**Produces** (used by the glue and the helpers):
`Face`, `FilterMode`, `FilterKind`, `FilterEntry`, `LabelConfig`,
`LabelKey`, `LabelStore`, `Slot`, `Plan`;
`LabelConfig::interval_ticks`, `::transfer_amount`, `::passes_filter`,
`::budget`, `::plan`, `::is_due`, `::encode`, `::decode`;
`LabelKey::text`/`parse`; `LabelStore::{insert, remove, get, get_mut, iter, encode, decode}`;
`Command` + `Command::parse`.

- [ ] Write the failing tests first (rate, amount, filter modes, regulating cap,
      slot choice, encode/decode round trip, command grammar).
- [ ] `cargo test` → red (the types do not exist).
- [ ] Implement to green.

### Task 2: the glue

**Files:** `mixins/transfer_labels/support.rs` + one helper per injection.

- `LEVEL_LABELS: AttachmentType<LabelStore>` (namespace `transfer_labels`, name
  `labels`, codec = the runtime's encode/decode, no sync, no death copy).
- `on_level_tick_pre(game, dimension)`: read the store, for each label compute
  the two blocks, snapshot both containers with `native_container_slots`, ask the
  runtime for a plan, run `native_move_items`.
- `on_right_click_block`: held item is the label → attach (refuse if one is
  already there), consume one, cancel the click.
- `on_left_click_block`: held item is the label and a label is on that face →
  remove it, give the label back, cancel the click.
- `on_commands` / `on_register_commands` / `on_perform_command`:
  `/transfer_labels info|list|filter|type|entry|amount|speed|remove …`.

### Task 3: the content

**Files:** `content/` — `horizon-mod.toml`, the one item declaration, its model,
texture, lang keys, recipe (hopper/chest/redstone → 2), creative tab, and the
mineable/building tag entries the gui texture needs. Built to
`content/transfer_labels.hmod`, copied to `mixins/transfer_labels/data/`.

### Task 4: the ladder and the package

`stage → check → test → build` through `ports/tools/build-slot.sh`, then
`tools/package.sh` (content build + `cargo horizon-mod check` of both artifacts).

### Task 5: the e2e

`tools/e2e.sh`, three legs:

- **A (default, content as `--dev-mod`)**: chest A with a label on its top face,
  chest B above it; before the label nothing moves (control); after it, exactly
  `floor(T1/20) - floor(T0/20)` items over the window; a whitelist that does not
  match moves nothing; a matching whitelist moves; `speed 38` gives one item a
  tick; `amount 63` moves 64 at once; `regulating 10` stops at exactly 10 and
  never moves more than 4 per activation; the label survives a save + reload.
- **B (`EMBEDDED=1`)**: the same game launched with **no `--dev-mod`**, plus a
  control run with `.horizon-cache` unusable where the item id must be unknown.
- **C (`SELFTEST=1`)**: flips the rate assertion; the run must exit non-zero.
