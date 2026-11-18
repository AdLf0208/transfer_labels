# Transfer Labels — slice 1 (item-stack extract label)

> **Format (2026-11-18): the port is now a flat SDK `.hmod`.** Horizon retired `.hnmod` in the
> mod-API merge (#16941, launcher ≥ 0.2.70); the bundle this file describes below (PR #908, closed)
> was migrated to `horizon-mod.toml` + `Cargo.toml` + `src/lib.rs` (`impl mod_api::Hooks`) +
> `assets/`/`data/` at Horizon **`0b14f8acfd97fa2c750e42795249dcc981178ca4`** / horizon-mixins
> **`e59da9094f0c2fdda6c627fe9eff22dec6a906f0`**, mod API `0.2`, version **0.2.0**. Read
> **`## Migration to the SDK format`** at the end of this file first: it is the current state, the
> file map and the traps. Everything above it is the history of the retired bundle (`1d63e0a5`,
> `.hnmod`, `mixins/`, the staged ladder) and is kept as the record, not as instructions — none of
> `tools/horizon.py`, `tools/package.py`, `compatibility.json`, `mixins/<id>/` or the `.hnmod`
> exists any more.

## What this is

| | |
|---|---|
| Mod | Transfer Labels 0.1.9 (`transfer_labels`), `com.buuz135.transfer_labels` |
| Class / tier | **content / machines** (core, P0) — a label moves items between two containers through the level's item-handler capability and its own tick |
| Original licence | **MIT**, read from the jar's `META-INF/neoforge.mods.toml` (`license = "MIT"`) — **rank 0**, publishable with the maintainer's approval (ARCHITECTURE.md §8). Jar kept at `ports/_shared/licence-cache/transfer_labels-0.1.9.jar` |
| Pin | **now** Horizon `0b14f8acfd97fa2c750e42795249dcc981178ca4`, horizon-mixins `e59da9094f0c2fdda6c627fe9eff22dec6a906f0`, mod API `0.2` (`horizon-mod.toml` `[native]`); the retired bundle pinned `1d63e0a5` / `5a64af93` |
| Upstream source read | `https://github.com/Buuz135/Transfer-Labels` @ `e7b04d013cf8becedc9c644e67e9348662d6b7b3` (analysis only; clone deleted after `FINDINGS.md`) |
| Approach (from `modpacks.md`) | carry-over for the item's assets/data, agent port for the behaviour; both halves now ride inside the one flat `.hmod` |

## Slice boundary

**Implemented:** the **item-stack extract label**
(`transfer_labels:itemstack_extract_transfer_label`).

- Attached by right-clicking a block face with the label in hand (one item used
  up); removed by left-clicking the labelled face with the label in hand (the
  item, and the copies its upgrades stand for, come back). Cancelling the click
  is what keeps a chest from opening and the block from breaking.
- Runs on `level_tick_pre`, once per label whose position the level reaches,
  `gameTime % (20 - min(speed,38)/2) == 0` — **one item every 20 ticks** with no
  upgrade, one a tick at 38 speed upgrades.
- Moves **`1 + amount` items per operation** (1..64), through the pin's
  hopper-rules helpers (`native_container_slots` + `native_move_items`), so an
  item the destination refuses stays in its slot and nothing is lost.
- Filter: `whitelist`/`blacklist` (blacklist is the default, as upstream), with
  the `normal` (same item id), `mod` (same namespace) and `regulating` (keep N on
  the destination, **at most 4 per operation**) kinds, 20 slots, one count per
  slot.
- Labels are level state: one `AttachmentType<LabelStore>` under `Holder::Level`,
  saved to the dimension's `data/horizon/attachments.dat` (upstream's
  `LabelStorage` `SavedData`), so a label survives a save and a reload.
- `/transfer_labels list|info|filter|type|entry|clear|amount|speed|remove`
  configures a label by position and face.

**Gaps (deliberate, each with its reason):**

1. **No block entity.** Upstream has none either (labels live in level saved
   data); the brief's "attached block entity" is not what the artefact does, and
   a native mod cannot give a vanilla chest one. The tier is still `machines`
   because the behaviour is container capability work.
2. **The insert label, all four fluid labels and the label↔label conversions are
   not carried.** The slice is one family; only the extract item exists in this
   port's content, so no recipe can produce an item that does nothing.
3. **No filter/GUI half.** Upstream configures a label through a Titanium addon
   container (filter grid, whitelist button, filter-kind scroll, amount and speed
   upgrade *slots* holding copies of the label item). This port has no menus and
   no screens: **`/transfer_labels` is the configuration path**, and the amount
   and speed upgrades are numbers (0..63, 0..38) rather than items in slots. A
   full menu/screen port is the next slice's largest piece.
4. **`exact_count` and `tag` filter kinds are not implemented.** A native mod has
   no item-tag lookup helper at this pin, and `exact_count` gathers one item from
   several source slots, which `native_move_items` cannot do; naming either kind
   is refused instead of silently moving nothing.
5. **No client half.** Upstream draws the label as a thin billboard on the face
   and ray-traces its own shape so a label on a covered face can still be
   clicked. Horizon has no per-position client shape for a non-block, so a label
   is attached by clicking the *bare* face and configured by command; the e2e
   therefore attaches the label before placing the neighbouring chest.
6. **Labels are not removed when their block is broken or replaced**, and a
   ticking label whose two blocks are not both containers simply does nothing.
   Upstream also keeps stale entries; `block_changed` cleanup is a later slice.
7. **The label item's components are not compared**: the filter matches the item
   id (and the namespace for `mod`), where upstream's `ItemStack.isSameItem`
   also compares data components. Two differently-componented stacks of one item
   are one filter entry here.
8. **The command's answers are literal text**, not translation keys
   (`Text::literal`), as the campaign's other ports do; a lang-key pass belongs
   with the GUI slice, where the strings become visible in a screen rather than
   in chat.
9. **One item per operation, from the first source slot that passes.** Upstream
   breaks out after one slot's transfer; `native_move_items(dim, from, …, item,
   max)` may continue into a later slot of the same container, so it can carry
   the same item from two slots up to the same total. It never moves more than
   the budget.

## Upstream files read (behaviour only, no code copied)

`TransferLabelItem.java` (use-on attach, `Mode`), `ItemStackTransferLabelItem.java`
(filter construction), `LabelInteractEvents.java` (the level tick, the right/left
click paths), `storage/LabelStorage.java` (SavedData, add/remove, save/load),
`storage/LabelBlock.java` (a position's faces, `remove` returning the upgrade
items), `storage/LabelInstance.java` (`work`, `getSpeed`, `getAmount`, the NBT
keys), `filter/ItemFilter.java` (`work`, `transferItems`,
`calculateMaxTransferAmount`, `passesFilter`), `filter/ILabelFilter.java`
(`Type` = whitelist/blacklist predicate), `filter/FilterType.java`,
`filter/extras/NumberFilterExtra.java`, `Config.java`, plus the jar's recipes,
lang and item models. Findings and the jar inventory are in
[`FINDINGS.md`](FINDINGS.md).

## Deviations from the brief

The brief described the shape as "an attached block entity with a direction and
filter rules". The artefact has **no blocks and no block entities** (jar
inventory: five items, no blockstates, no block models; `LabelBlock.java` is a
position record), and the upstream state is a level `SavedData`. The port
therefore stores labels in a level attachment and ticks them from
`level_tick_pre` — same behaviour, same tier (`machines`: container transfer on
the mod's own tick), different container. Recorded here rather than silently
built the other way.

## Hooks used (all stamped at the pin)

| Hook | API | Use |
|---|---|---|
| `level_tick_pre` | 4 | run every due label in the level |
| `right_click_block` | 21 | attach a label to the clicked face, use one item, cancel the click |
| `left_click_block` | 30 | remove a label and give the item back, cancel the click |
| `commands` | 1 | the command's name in the chat tree |
| `register_commands` | 8 | the command's usage lines |
| `perform_command` | 8 | run `/transfer_labels …` |
| `mods::content_root` (HEAD, target) | — | write the embedded `.hmod` out and append it to the dev list (AE2 pattern) |

Helpers used: `native_attachments`, `native_level`, `native_level_game_time`,
`native_level_is_loaded`, `native_container_slots`, `native_move_items`,
`native_held`, `native_take_items`, `native_give_items`, `native_feedback`,
`native_dimension`, `native_data!`.

## Verification (evidence in `ports/_shared/evidence/transfer_labels/`)

- Runtime: **28 unit tests** (`cargo test`, red-first: all 28 failed at `todo!()`
  before the rules were written).
- Ladder: `stage` → `check` → `test` → `build`, remote slot, `cargo_exit 0`,
  **0 warnings**; embedded tests run as `game::transfer_labels::tests`
  (**28 passed**, 1240 filtered out of the game's own suite).
- Deliverable: `transfer_labels.hnmod` in the port root (one file: code + the
  embedded `data/transfer_labels-content.hmod`), `cargo horizon-mod check
  --kind native-mod` green; the content `.hmod` passes `horizon-mod check` as a
  content-only mod.
- `e2e-default.log`: PASS + `EXIT:0` — the rate (`floor(T1/20) - floor(T0/20)`
  items over the window), the whitelist miss/hit, blacklist, `speed 38`,
  `amount 63` (64 items in the first activation), `regulating 10` (≤4 an
  activation, settles at exactly 10), removal giving the item back without
  breaking the block, and a save + relaunch keeping the label.
- `e2e-selftest.log`: FAIL + `EXIT:1` (the rate assertion flipped), so the
  scenario can fail.
- `e2e-embedded.log`: PASS + `EXIT:0` — the same run with **no `--dev-mod`**, the
  content written from the bundle's `data/` and byte-identical to
  `content/transfer_labels.hmod`, plus a control leg where `.horizon-cache` is a
  plain file: no label item, and the id is unknown.

Skills: superpowers/using-superpowers,brainstorming,writing-plans,test-driven-development,systematic-debugging,verification-before-completion; BeautifulRust/rust-best-practices,rust-port

## Things a next slice should pick up

1. The insert label and the four fluid labels (`native_fluid_tanks` /
   `native_move_fluids`, `fluidTransferMultiplier = 100` mB per item amount).
2. A menu + screen for the filter grid, the whitelist button and the upgrade
   slots (API 10/19/20), replacing the command as the configuration path.
3. `exact_count` and `tag` once a tag helper and a multi-slot gather exist.
4. `block_changed` cleanup so a label dies with its block, and a client shape so
   a covered face can be clicked.

## Review repairs (PR #670, 2026-11-11)

Review: `ports/_shared/horizon-mods/reviews/pr-670-transfer_labels.md` (CHANGES_REQUESTED, 3 items).

| Reviewer's item | This job |
|---|---|
| 1. MIT notice missing from both archives | **fixed**: `tools/package.py` writes `mixins/transfer_labels/LICENSE` + `LICENSE-NOTICE.md` into the `.hnmod`; `tools/notice.py` writes them into the embedded `.hmod` |
| 2. relative link `(Buuz135/Transfer-Labels)` and Buuz135 uncredited | link and authors were already absolute/credited; re-checked |
| 3. description stops at "one item every 20 ticks with no"; "Not yet ported: nothing recorded yet" is wrong | **rewritten**: the 20-tick cadence with the speed formula, the filter kinds, and the real gaps (`tag`/`exact_count` refused, no menu, command path does not consume upgrade items) |

The reviewer's additional observation — `/transfer_labels amount|speed` sets upgrade counts
without consuming label items while `remove` refunds them — is recorded in the new
description as part of "not yet ported": that economy path is not a player-facing feature
until the menu slice puts the upgrades in slots.

Artifact: `transfer_labels.hnmod` **25,173 B** `27ed6ef419663f51bb860bd2f8e74269833533e74c459f0ea3db268d6c896b00`
(content `transfer_labels.hmod` **6,784 B** `ab87a153bf4195063fb0da517f8c64cce11871457fc08ddc828bbbda5fcdb712`).

### Verification after the repairs (2026-11-12)

Every step in a `ports/tools/build-slot.sh` lease with `CARGO_INCREMENTAL=0`.
`tools/package.sh` now runs `tools/notice.py` itself, so a rebuild can no longer
produce a content `.hmod` without the notices (before this repair the injection
was a manual step and a rebuild silently dropped it).

| Check | Result |
|---|---|
| `tools/package.sh` twice (`--step package`) | both runs: `content/transfer_labels.hmod` **6,784 B** `ab87a153…b712`, 11 entries (`added=LICENSE,LICENSE-NOTICE.md`), and `transfer_labels.hnmod` **25,173 B** `27ed6ef4…96b00`, 14 entries; `cmp` identical on both artifacts |
| embedded copy | `cmp content/transfer_labels.hmod mixins/transfer_labels/data/transfer_labels-content.hmod` → identical |
| notices in **both** archives | outer `.hnmod`: `mixins/transfer_labels/LICENSE` (1,189 B) + `LICENSE-NOTICE.md` (1,120 B); embedded `.hmod`: `LICENSE` + `LICENSE-NOTICE.md` at the archive root |
| `registry.toml` | `sha256 = 27ed6ef4…96b00` = `sha256sum transfer_labels.hnmod` |
| runtime | `cargo test --locked --offline` **28 passed**, 0 failed; clippy `-D warnings` clean; `cargo fmt --all --check` clean |
| ladder | `stage` → `check` → `test` → `build`, `cargo_exit 0` each, **0 warnings**; embedded tests `game::transfer_labels::tests` **28 passed**, 1240 filtered out |
| e2e, default | `e2e-repair.log`: **`PASS` + `EXIT:0`** — the full default scenario (rate, whitelist/blacklist, speed 38, amount 63, regulating 10, removal, relaunch) |
| e2e, fail-capable | `e2e-repair-selftest.log`: **`FAIL` + `EXIT:1`** — `selftest: 11 items moved, expected the wrong count` |
| log audit | `check-e2e-logs.py transfer_labels --strict` exit 0, **5/5** logs ok |
| skills gate | `check-skills.py --notes ports/transfer_labels` → `SKILLS GATE: PASS` |
| upstream gate | `upstream_ready.py transfer_labels` → `READY`, exit 0 |

Notes on the reviewer's items: (1) the notices are in both archives and the
packager now stamps them automatically; (2) the upstream link is absolute and
Buuz135 is credited in `authors` and in the description's first sentence — it
already was, and this job re-checked it against `registry.toml`; (3) the
description is the rewritten one (complete sentences, the real `tag`/`exact_count`
and menu gaps, the command path's upgrade economy explained). The reviewer's
`run_command` observation is recorded above as a "not yet ported" limitation.

The reviewer's closing line — "README has commands, not a completed validation
report" — is answered twice: `VALIDATION.md` in this tree carries the ladder,
the measured numbers and the can-fail legs, and the publish tree's `README.md`
now ends with a *Validation (measured)* section (it had also been mangled into
one-bullet-per-line with a relative upstream link and a "nothing recorded yet"
line, which is fixed).

## Migration to the SDK format (2026-11-18, SDK `0b14f8a`)

Horizon's mod-API merge (#16941, launcher ≥ 0.2.70, PR #17992) retired the `.hnmod` container, and
the registry closed this port's submission (**PR #908**) on 2026-11-18 with *"republish this as a
native `.hmod` with the SDK from the next Nightly"*. The port is now one flat package at nightly
**`0b14f8acfd97fa2c750e42795249dcc981178ca4`** (2026-11-18), horizon-mixins
**`e59da9094f0c2fdda6c627fe9eff22dec6a906f0`**, mod API **`0.2`**, launcher 0.2.78, version
**0.2.0**.

**The campaign's rule held: `src/runtime.rs` is the old `runtime.rs`, unchanged.** `cmp` against the
retired copy is byte-identical (SHA-256
`88e46c3291b14df3ebce0f008c2fea1471f999ebb017e54d6801e7fa1d73d3f5`, 1,077 lines), including the
file inside the built package, and its **28 tests ran green before any glue existed**
(`evidence/transfer_labels/sdk-0b14f8a/sdk-test-runtime-only.log`: `28 passed; 0 failed`). The crate
now runs **37**: those 28 plus 9 for the glue's conversions (`sdk-test.log`).

**Class + tier unchanged:** class **content**, tier **machines** — the label is carried content (one
item, its model/texture/lang, recipe, creative tab, tag) plus the container-transfer behaviour on the
level's tick. No new machine, menu or deep-tier feature is added.

### Files created, carried and deleted

| Action | Path | Note |
|---|---|---|
| created | `horizon-mod.toml` | the one manifest: id/name/version/description/authors/`api = "0.2"`/side/capabilities + `[native]` |
| rewritten | `Cargo.toml` | one crate, `horizon-mod-api` from `.horizon/sdk/`, `publish = false`; no `[lib] path`, no `[workspace]`, **no `rust-version`** |
| created | `src/runtime.rs` | the retired `mixins/transfer_labels/runtime.rs`, byte-identical (`runtime-carry.log`) |
| created | `src/lib.rs` | the one `#[mod_api::horizon_mod] impl Hooks` (was `support.rs` + 7 hook helpers) + 9 glue tests |
| carried | `assets/transfer_labels/**`, `data/**` | the retired `content/assets` + `content/data`, byte-identical (`content-carried.log`: 8 files `same`) |
| copied in | `icon.png` | the published icon (128×128, 858 B) |
| created | `.gitignore` | `/target`, `/.horizon`, `/Cargo.lock` (trap 4) |
| rewritten | `README.md`, `AGENTS.md` | both described the retired layout; `README.md` ships inside the package |
| deleted | `mixins/transfer_labels/` | `horizon-mixin.toml`, `support.rs`, `runtime.rs`, the 7 helpers, the embedded content `.hmod` |
| deleted | `content/` | the content half is now `assets/` + `data/` at the package root; its second manifest and its own `.hmod` are gone |
| deleted | `compatibility.json`, `rust-toolchain.toml` | the pin is `[native]`; the build refuses a toolchain file |
| deleted | `tools/horizon.py`, `package.py`, `package.sh`, `notice.py`, `audit/` | the staged/mixin pipeline; `cargo horizon-mod build/check/fits` replaces it |
| deleted | `transfer_labels.hnmod`, `Cargo.lock` | the retired container; the old lib lock (a new one is only written by `cargo test`, and never packed) |
| retired | `_shared/evidence/transfer_labels/retired-hnmod-0.1.0/` | `mixins/`, `content/`, `compatibility.json`, `rust-toolchain.toml`, the `.hnmod` (**25,173 B** `27ed6ef4…96b00`), the retired tools, and `artifact-sha256-before.txt` |

Kept on purpose: `tools/e2e.sh` + `tools/parse_info.py` (the scenario and its parser are
format-independent; they need a launcher-built game), `FINDINGS.md`, `PLAN.md` (history of the
retired bundle), `VALIDATION.md` (history), `LICENSE`, `LICENSE-NOTICE.md`, `notes.md`.

### Manifest field mapping

| Retired (`mixins/transfer_labels/horizon-mixin.toml` + `content/horizon-mod.toml`) | Now (`horizon-mod.toml`) |
|---|---|
| `[mod] id = "transfer_labels"` | `id = "transfer_labels"` (also the resource namespace) |
| `[mod] version = "0.1.0"` | `version = "0.2.0"` (a resubmission is a new version) |
| `[mod] priority = 100` | gone (ordering is `#[listener(priority = …)]`, unused here) |
| `[compatibility] horizon_revision = "1d63e0a5…"` | `[native] horizon = "0b14f8ac…"` (exact 40 hex) |
| — | `[native] mixins = "e59da909…"`; `[native] game = "26.3"` |
| `[[module]] runtime` (injected as `crate::game::transfer_labels`) | `src/runtime.rs` + `pub mod runtime` |
| `[[module]] state` (`support.rs`) | the same code as methods and private functions in `src/lib.rs` |
| `[[injection]]` × 6 + the `content_root` target | `impl Hooks` methods, no plan file |
| content inside the bundle (`data/transfer_labels-content.hmod`) | `assets/` + `data/` in the package, nothing embedded |
| `api = "0.2"` in the content manifest | the one `api = "0.2"` |
| — | `capabilities = ["registries"]` (the fixed shared value; needed because the item is registered) |

### Hook → `Hooks` method (every retired injection)

| Retired hook (API) | Retired helper | SDK method / what it became |
|---|---|---|
| `level_tick_pre` (4) | `level_tick_pre.rs` | `fn level_tick_pre(&self, HostSide, &mut LevelTickPreEvent)`: the level arrives as `event.level`, so no `native_level`/`native_dimension`; the server side is the only one that acts |
| `right_click_block` (21) | `right_click_block.rs` | `fn right_click_block(&self, HostSide, &mut RightClickBlockEvent) -> Flow`: the player, hand, stack, position and face are all on `event.click` (`BlockClick`), the cancel is `Flow::Cancel` + `*event.cancellation_result = SuccessServer` |
| `left_click_block` (30) | `left_click_block.rs` | `fn left_click_block(&self, &mut dyn ServerHost, &mut LeftClickBlockEvent) -> Flow`: `Flow::Cancel` stops the dig; the acting player is `host.players().acting_player()` |
| `commands` (1) | `commands.rs` | **removed**: the SDK builds the command tree from the registered usages, so a separate name-only hook has nothing to add |
| `register_commands` (8) | `register_commands.rs` | `fn register_commands(&self, &mut dyn ServerHost, &mut RegisterCommandsEvent)`: the same nine usages, `mod_api::Command::new` |
| `perform_command` (8) | `perform_command.rs` | `fn run_mod_command(&self, &mut dyn ServerHost, &mut RunModCommandEvent) -> Flow`: the line arrives **parsed** (`CommandInvocation`); it is turned back into the words the runtime's grammar reads |
| `mods::content_root` (HEAD, target) | `embedded_content.rs` | **removed, not mapped**: one flat `.hmod` carries `assets/` and `data/`, so there is no second half to write out, no `native_data!` and no `.horizon-cache` |

### Host helper → SDK call (inside the glue)

| Retired (`support.rs`) | Now |
|---|---|
| `static LABELS: AttachmentType<LabelStore>` + `native_attachments(Holder::Level)` | the same `AttachmentType` (namespace `transfer_labels`, name `labels`, same codec) reached through `host.attachments()` + `get_existing(…, Holder::Level)` / `set(…, Holder::Level, …)` |
| `native_level` / `native_level_game_time` / `native_level_is_loaded` | `LevelRef` from `event.level` or `PlayerRef::level(host)`; `game_time(host)`, `is_loaded(host, pos)` |
| `native_container_slots` | `LevelRef::capability(host, &ITEMS, pos, Some(side))` → `Capability<dyn ResourceHandler<ItemResource>>`: `size`, `resource`, `amount` |
| `native_move_items(dim, from, face, to, face, item, max)` | `move_resources(host, &source, &destination, filter, max, tx)` inside one `host.transaction` |
| `native_held` | `LivingRef::hand_item(host, Hand::Main)` / the click's own `stack` |
| `native_take_items(LABEL_ITEM, 1)` | `LivingRef::set_hand_item` with the hand's count minus one (`None` when it empties) |
| `native_give_items(LABEL_ITEM, back)` | `PlayerRef::give_stack`, split by the item's `max_stack_size` (`stack_chunks`) |
| `native_feedback(Text)` | `PlayerRef::tell(host, Text::literal(line))` |
| `native_dimension` | `PlayerRef::level(host)` |
| `native_data!` + the cache write | gone with the embedded content |

### Deviations the SDK mapping forces (each with what a player notices)

1. **A source slot's `can_take` is not a capability query.** `ResourceHandler` has no per-slot
   "may be emptied" read, so the glue hands the runtime `can_take: true` and the game decides at
   `extract`; `move_resources` skips a slot that answers zero and goes on to the next index, which is
   the retired helper's rule. Difference: a container that refuses the *first* passing slot with a
   *different* item below it now moves that item, where the retired plan skipped the slot before
   choosing (the retired notes' gap 9 already allowed the same within one container).
2. **`give_stack` throws what does not fit instead of keeping it.** `/give`'s own path; the retired
   `native_give_items` gave only what fitted and dropped the rest. A removal now always hands back
   all `1 + amount + speed` items (up to 102, in stacks split by `max_stack_size`).
3. **Attach answers `SuccessServer`.** Vanilla's `SUCCESS` makes the server swing the hand; the
   retired port only cancelled the click, so nothing swung. Same visible result as upstream now.
4. **The attach consumes from the hand that clicked.** The retired `native_take_items` took from the
   main hand first and then the rest of the inventory; the click's own stack is the label, so the two
   agree, and the click's hand is what upstream's `ItemStack.consume` shrinks.
5. **`/transfer_labels info` reads its level from the acting player.** The retired code read the
   level in place; for a player's command those are the same level.
6. **Labels are still one attachment per level.** `Holder::Level` is the level the game has in place:
   during `level_tick_pre` the ticking level, inside a player's hook or command that player's level,
   so each dimension keeps its own labels (upstream's per-level `SavedData`).

### Gaps after the migration

1. **No in-game run at `0b14f8a`.** There is no `horizon-launcher` on this box, so the package cannot
   be compiled into a game binary (`horizon-launcher native-build …`) and `tools/e2e.sh` cannot run.
   The behaviour's evidence is the previous slice's fail-capable e2e (`e2e-default.log` PASS +
   `EXIT:0`, `e2e-selftest.log` FAIL + `EXIT:1`, `e2e-embedded.log`), the byte-identical runtime and
   its tests, the byte-identical content, and `check`/`fits`. The first item of the next slice is a
   launcher-built game and `GAME=<that game> tools/e2e.sh` recorded here. The `embedded-*` legs of
   the scenario are retired with the embedded content.
2. **No log service in the SDK**, so the retired bundle's `[transfer_labels] …` lines are gone; the
   scenario already asserts through `/transfer_labels info` and the containers, not through logs.
3. The slice's own gaps are unchanged: the insert label and the four fluid labels, the menu/screen
   (and with it the upgrade economy), `exact_count`/`tag` (still refused, not silently ignored), the
   client billboard and its own click shape, `block_changed` cleanup, and comparing item components.

### Build and check (verbatim)

```sh
# runtime only, before any glue existed (lib.rs was `pub mod runtime;`)
ports/tools/build-slot.sh run --job sdk-tl --port ports/transfer_labels --step test \
    -- bash -c 'cd ports/transfer_labels && cargo test --offline'
# → running 28 tests … test result: ok. 28 passed; 0 failed

# with the glue
ports/tools/build-slot.sh run --job sdk-tl --port ports/transfer_labels --step test \
    -- bash -c 'cd ports/transfer_labels && cargo test --offline'
# → running 37 tests … test result: ok. 37 passed; 0 failed

ports/tools/build-slot.sh run --job sdk-tl --port ports/transfer_labels --step lint \
    -- bash -c 'cd ports/transfer_labels && cargo fmt --check &&
        cargo clippy --all-targets --offline -- -D warnings'
# → FMT_EXIT:0, Finished `dev` profile … (0 warnings), CLIPPY_EXIT:0

ports/tools/build-slot.sh run --job sdk-tl --port ports/transfer_labels --step build \
    -- bash -c 'cd ports/transfer_labels && rm -f Cargo.lock && cargo horizon-mod build'
# → SDK 0b14f8a (build 0b14f8acfd97fa2c750e42795249dcc981178ca4) in ./.horizon/sdk
# → Built ./target/horizon-mod/transfer_labels.hmod

ports/tools/build-slot.sh run --job sdk-tl --port ports/transfer_labels --step check \
    -- bash -c 'cd ports/transfer_labels && cargo horizon-mod check transfer_labels.hmod &&
        cargo horizon-mod fits transfer_labels.hmod --build nightly'
# → transfer_labels 0.2.0 (native-mod, format native-1/0b14f8ac…/e59da909…, for 26.3)
# → 35400 bytes, SHA-256 0e8fa5dd…, unsigned    CHECK_EXIT:0
# → Transfer Labels: / 0b14f8a  made for    FITS_EXIT:0
```

`BUILD_LOCAL=1` on every step: `build-slot.sh` forwards `stage`/`build`/`test`/`check` to the second
box when it is reachable, and this port is not synced there.

**The tool is the new one, run directly in the port.** `build-slot.sh run --step …` is only the
campaign's lease and label here: it has no `stage`/`plan`/`package` notion any more, and the SDK
layout needs none — the command inside the lease is the whole step (`cargo test`, `cargo
horizon-mod build|check|fits`). The runner still does not `cd`, hence the `bash -c 'cd ports/<id> &&
…'` wrapper on every line.

### Traps this migration cost (read before the next one)

1. **`build-slot.sh` does not `cd` into the port.** `--port` is a label (and the cache-link target);
   a bare `cargo test` from the campaign root answers `could not find Cargo.toml`. Wrap every
   command as `bash -c 'cd ports/<id> && …'`.
2. **The build refuses anything outside the package schema, naming the path.** Measured twice:
   `error: unexpected package path: .horizon.seeded/sdk/api/Cargo.toml` — a *dot-directory* beside
   `.horizon` is enough, not just a left-over `mixins/`. Retire the old layout, do not park it in the
   port.
3. **`cargo test` needs `.horizon/sdk/` first.** The `horizon-mod-api` path dependency does not
   exist until a `cargo horizon-mod` command fetches it, so the ladder starts with `build` (or
   `cargo horizon-mod update`) on a fresh clone. The SDK was seeded here from the in-tree cache
   (`ports/_shared/sdk/0b14f8a/`, fetched by another port) and the tool later **fetched it itself**
   when `.horizon` was removed: the two `sdk.json`s carry the same commit and the same
   `api-source.tar.xz` SHA-256, and the rebuild from the fetched copy produced the same bytes.
4. **`Cargo.lock` changes the artifact.** `cargo test` writes one; `cargo horizon-mod build` then
   packs it. The port has no registry dependency of its own, so it is packaged **without** one:
   `rm -f Cargo.lock` right before the build, and `/Cargo.lock` in `.gitignore`.
5. **`cargo fmt --all --check` reports the fetched SDK's files** (`.horizon/sdk` is a path
   dependency). Use `cargo fmt --check`; never "fix" the fetched SDK.
6. **Do not declare `rust-version`.** The scaffold declares none; a `rust-version = "1.99"` is above
   the box's toolchain and cargo refuses the crate.
7. **`HostTransaction` must be imported** for `host.transaction`: it is a supertrait method, and
   without the import the compiler says *trait `HostTransaction` which provides `transaction` is
   implemented but not in scope*.
8. **`BlockClick.player` is a `u128` uuid, not a `PlayerRef`.** Resolve it with `host.player(uuid)`
   before you need `tell`/`give`/`level`.
9. **`JavaString` is not exported by `mod_api`**, and `Compound` is `BTreeMap<JavaString, Tag>`, so a
   compound member cannot be read with a `&str` key (`Tag::get(&str)` needs a `&Tag`, not a
   `&Compound`). Writing works on `&str` (`Tag::compound([(key, tag)])`), so the codec decodes member
   names through a small comparing helper (`member`, with a test) and encodes through `Tag::compound`.
10. **One `block_pos` argument is three words.** `CommandArgumentValue::BlockPos` arrives as one
    `[i32; 3]`; the runtime's grammar reads `x y z`, so `argument_words` spreads it back. The test
    that proves it was proved fail-capable by flipping the spread (`sdk-glue-test-red.log`:
    `left: ["10", "up", "5"] / right: ["10", "64", "-3", "up", "5"]`, `EXIT:101`).
11. **`give_stack` takes one stack.** A removal can hand back up to 102 label items
    (`1 + 63 amount + 38 speed`), so it is split by the item's own `max_stack_size` (`stack_chunks`).
12. **`cargo horizon-mod check --json` answers `capabilities: []` and `side: null` for every
    native-mod** — the verified `imfast.hmod` and `repeatable_trial_vaults.hmod` answer the same. It
    is not a missing manifest field; the same manifest validates as a content package.
13. **No `log` crate in a native mod.** `log::warn!` does not compile; nothing here needs it.
14. **`check` with no file checks wasm** (`imfast` trap 3): always `check <file.hmod>` here, and
    `cargo check`/`cargo test` for the native build.

### Verification of this migration

Skills: superpowers/using-superpowers,writing-plans,test-driven-development,systematic-debugging,verification-before-completion; BeautifulRust/rust-best-practices

| Step | Command | Log | Result |
|---|---|---|---|
| runtime, byte-identical | `cmp src/runtime.rs retired/…/runtime.rs` | `runtime-carry.log` | identical, SHA-256 `88e46c32…3d3f5`, also inside the package |
| runtime tests before glue | `cargo test --offline` with `lib.rs` = `pub mod runtime;` | `sdk-test-runtime-only.log` | **28 passed**, 0 failed, `TEST_EXIT:0` |
| tests with the glue | `cargo test --offline` | `sdk-test.log` | **37 passed**, 0 failed, `TEST_EXIT:0` |
| the glue test can fail | `BlockPos` spread flipped to one word, re-run | `sdk-glue-test-red.log` | **FAILED**, 2 failed, `EXIT:101`; restored, green again |
| formatting | `cargo fmt --check` | `sdk-lint.log` | `FMT_EXIT:0` |
| lints | `cargo clippy --all-targets --offline -- -D warnings` | `sdk-lint.log` | `CLIPPY_EXIT:0`, 0 warnings |
| build | `cargo horizon-mod build` | `sdk-build.log`, `sdk-build-repeat.log` | `Built ./target/horizon-mod/transfer_labels.hmod`, twice identical |
| registry check | `cargo horizon-mod check transfer_labels.hmod` | `sdk-check.log`, `sdk-final-gate.log` | `0.2.0 (native-mod, format native-1/… , for 26.3)`, **35,400 B**, **EXIT 0** |
| fits | `cargo horizon-mod fits transfer_labels.hmod --build nightly` | `sdk-final-gate.log` | `0b14f8a  made for`, **EXIT 0** |
| uses | `unzip -p transfer_labels.hmod uses.json` | `sdk-uses.json` | 274 surface entries naming `hook:level_tick_pre`, `hook:right_click_block`, `hook:left_click_block`, `hook:register_commands`, `hook:run_mod_command`, `type:Holder.Level`, `handle:AttachmentType::set/get_existing` |
| content carried | `diff -r` against the retired `content/` | `content-carried.log` | every entry `same` (8 files), and the same bytes inside the package |
| fresh clone | `target/`, `.horizon/`, `Cargo.lock` removed, then `cargo test` + `cargo horizon-mod build` | `sdk-fresh-clone.log` | SDK fetched by the tool; 37 passed; same **35,400 B** / `0e8fa5dd…` |
| package hash | `sha256sum transfer_labels.hmod` | `sdk-sha256.txt` | `0e8fa5ddcdbdc2e89b76f00c3cd30345ec67d407b5e02f1a87d8bfd20c23e599` |
| package entries | `unzip -l` | `sdk-package-entries.log` | **17** entries: manifest, `Cargo.toml`, `LICENSE`, `LICENSE-NOTICE.md`, `README.md`, `icon.png`, `src/lib.rs`, `src/runtime.rs`, `uses.json`, 8 `assets/`+`data/` files |
| skills line | `python3 ports/tools/check-skills.py --notes ports/transfer_labels` | — | `SKILLS GATE: PASS` |

**Deliverable:** `transfer_labels.hmod` in the port root, **35,208 B**, SHA-256
`0e8fa5ddcdbdc2e89b76f00c3cd30345ec67d407b5e02f1a87d8bfd20c23e599`. The MIT notices
(`LICENSE` 1,189 B, `LICENSE-NOTICE.md` 1,120 B) travel **inside** it, and no `Cargo.lock` is packed.

Evidence: [`ports/_shared/evidence/transfer_labels/sdk-0b14f8a/`](../../_shared/evidence/transfer_labels/sdk-0b14f8a/)
(`sdk-test-runtime-only.log`, `sdk-test.log`, `sdk-glue-test-red.log`, `sdk-lint.log`, `sdk-build.log`,
`sdk-build-repeat.log`, `sdk-check.log`, `sdk-final-gate.log`, `sdk-uses.json`, `runtime-carry.log`,
`content-carried.log`, `sdk-fresh-clone.log`, `sdk-sha256.txt`, `sdk-package-entries.log`) plus the
retired bundle under `retired-hnmod-0.1.0/`.

### Changelog for the resubmission

> **Supersedes #908**, closed because Horizon retired the `.hnmod` container (#16941 / #17992).
> The mod is republished as one flat SDK package with `api = "0.2"` and `[native]` pinned to Horizon
> `0b14f8acfd97fa2c750e42795249dcc981178ca4` / horizon-mixins
> `e59da9094f0c2fdda6c627fe9eff22dec6a906f0` (game 26.3, launcher 0.2.78). Changes: the retired
> injections are now typed `impl mod_api::Hooks` methods — `level_tick_pre` for the tick,
> `right_click_block`/`left_click_block` for attaching and removing a label, and
> `register_commands`/`run_mod_command` for `/transfer_labels`; labels are a level `AttachmentType`
> instead of a hand-rolled attachment lookup; the transfer is one `host.transaction` over the
> `horizon:items` capability (`move_resources`), so a refused item still stays in its slot; the item,
> its model, texture, lang, recipe, creative tab and tag now ship inside the same `.hmod`, so the
> embedded-content injection and its `.horizon-cache` write are gone; a removal hands back every
> item the upgrades stood for, in stack-sized pieces, and attaching swings the hand as vanilla does.
> `runtime.rs` and its 28 rules are unchanged, with 9 glue tests beside them. **Version 0.2.0.**
> MIT; the original copyright and permission notices travel inside the package as `LICENSE` and
> `LICENSE-NOTICE.md`.
