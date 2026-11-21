# Validation — Transfer Labels

> **Current: 0.2.2 (2026-11-21) — the item-stack insert label, and every carried recipe.** Measured
> at the campaign pin **Horizon `0a875da91dbb0fdcb7ba8c42c11341aa229af51a`** / horizon-mixins
> `e59da9094f0c2fdda6c627fe9eff22dec6a906f0`, game 26.3, launcher 0.2.92. Delivered
> `transfer_labels.hmod` **45,567 B**, SHA-256
> `0d1504ad0a8324dd71da0ff97d88684ade73a1001547d79bb7a680eafe110e71`, **24 entries**, version
> **0.2.2**, no packed `Cargo.lock`. Full detail in `notes.md`, **`## Slice 2`**.

| Check | Command | Result |
|---|---|---|
| Red | `cargo test --offline` (tests first, rules as `todo!()`) | 44 passed, **5 failed** — `not yet implemented` (`sdk-test-red.log`) |
| Runtime tests | `cargo test --offline` | **50 passed**, 0 failed (`sdk-test.log`) |
| Formatting / lints | `cargo fmt --check`; `cargo clippy --all-targets -- -D warnings` | `FMT_EXIT:0`, `CLIPPY_EXIT:0`, 0 warnings (`sdk-clippy.log`) |
| Build ×2 | `rm -f Cargo.lock && cargo horizon-mod build`, twice, `cmp` | identical, **45,567 B** `0d1504ad…` (`sdk-build.log`) |
| Registry check | `cargo horizon-mod check transfer_labels.hmod` | `0.2.2 (native-mod, format native-1/0a875da…/e59da90…, for 26.3)`, **EXIT 0** (`sdk-check.log`) |
| Fits, the pin | `cargo horizon-mod fits … --build 0a875da` | `0a875da  made for`, EXIT 0 (`sdk-fits.log`) |
| Fits, another build | `cargo horizon-mod fits … --build 0b14f8a` | `0b14f8a  fits`, EXIT 0 (`sdk-fits-other.log`) |
| Carried data | `python3 ports/tools/check_261_dialect.py transfer_labels.hmod` | 4 recipes in the 26.3 dialect, EXIT 0 (`dialect-check.log`) |
| Package gate | `python3 ports/tools/verify-sdk-package.py transfer_labels` | **1/1** |
| In game, default | `launcher-game.sh build` → `Compiling transfer_labels (1 of 1)`, then the e2e in a slot | **PASS + EXIT:0** (`transfer_labels-e2e/e2e-launcher.log`) |
| In game, control | the same Nightly without the package | `without the package: no label item, the block stayed minecraft:air`, **FAIL + EXIT:1** |
| In game, `SELFTEST=1` | flips phase B's rate | `FAIL: selftest: 11 items moved, expected the wrong count`, **EXIT:1** |
| In game, `INSERT_SELFTEST=1` | flips phase I's direction | `FAIL: insert selftest: 11 items dropped, expected the wrong direction`, **EXIT:1** |
| Log audit | `python3 ports/tools/check-e2e-logs.py transfer_labels-e2e --strict` | `logs=4 ok=4` |

What the new phases prove, in numbers from the log: the insert label on the lower chest's top face
dropped **11 items over 215 ticks** (`floor(T1/20) - floor(T0/20)`) into the labelled chest and the
two chests still held **exactly 32** between them (19 + 13); the same geometry with no label moved
nothing; the same geometry with an extract label moved nothing either. All four carried recipes
crafted with their exact output id and count and their ingredients consumed, and a grid with an iron
ingot in the hopper slot made nothing.

> **History — 0.2.0/0.2.1 (`.hnmod` bundle and the SDK migration).** Everything below was measured at
> Horizon `1d63e0a5` / horizon-mixins `5a64af93` for the retired `.hnmod` container (Horizon retired
> it in #16941), or at `0b14f8a` for the first SDK package. It remains the record of slice 1 (the
> extract label) and its review fixes; `notes.md` carries those runs under **`## Verification`**,
> **`## Migration to the SDK format`** and **`## Review fixes (2026-11-18)`**.

> **2026-11-12 (review repairs, PR #670).** The delivered bytes changed when the
> MIT notices were added and `tools/package.sh` began stamping them itself: the
> content `.hmod` is **6,784 B** `ab87a153…b712` (11 entries) and the bundle
> **25,173 B** `27ed6ef4…96b00` (14 entries). The tables below record the
> pre-repair run; `notes.md`, *Verification after the repairs*, has the fresh
> ladder and e2e evidence.

Linux, Rust 1.95.0, the exact revisions in `compatibility.json`
(Horizon `1d63e0a5a37a7a708eb0b32b49bf0dfeb28ed8a9`, horizon-mixins
`5a64af938ec86b00d53d16a408aabc0038fb3e57`), every step inside a
`ports/tools/build-slot.sh` lease.

| Check | Command | Result |
|---|---|---|
| Runtime tests, standalone | `cargo test --locked --offline` | **28 passed**, 0 failed |
| Formatting | `cargo fmt` | clean |
| Lints | `cargo clippy --all-targets --locked --offline -- -D warnings` | clean |
| Stage | `python3 tools/horizon.py stage` | `cargo_exit 0`, one bundle planned |
| Compile in the game | `python3 tools/horizon.py check` | `cargo_exit 0`, **0 warnings** |
| Embedded tests | `python3 tools/horizon.py test` | `cargo_exit 0`, `game::transfer_labels::tests`: **28 passed**, 1240 filtered out |
| Fast build | `python3 tools/horizon.py build` | `cargo_exit 0`, **0 warnings**, `target/…/fast/horizon-client` |
| Content package | `cargo horizon-mod build` / `check` (in `tools/package.sh`) | `transfer_labels.hmod`, 5,223 B, 9 entries, SHA-256 `3043e024d3d04396f4d66a4dff167fdfe51f58cc02a79b72efb34f483bf322f9`, “no code: content only” |
| Native package | `tools/package.py` + `cargo horizon-mod check --kind native-mod` | `transfer_labels.hnmod`, 22,075 B, 12 entries, SHA-256 `65a1c37b2df77fea6142acb959704c15d819b12fc3b23e4e724a72e4f681344b` |
| e2e, content as a dev mod | `build-slot.sh e2e -- tools/e2e.sh` | PASS, `EXIT:0` — `evidence/transfer_labels/e2e-default.log` |
| e2e, self-test | `bash -c 'SELFTEST=1 exec tools/e2e.sh'` | FAIL, `EXIT:1` — `evidence/transfer_labels/e2e-selftest.log` |
| e2e, embedded bundle (no `--dev-mod`) + control | `bash -c 'EMBEDDED=1 exec tools/e2e.sh'` | PASS, `EXIT:0` — `evidence/transfer_labels/e2e-embedded.log` |
| e2e log audit | `python3 ports/tools/check-e2e-logs.py transfer_labels --strict` | exit 0 (see the report) |

## What the e2e measured (numbers from the logs)

| Claim | Evidence |
|---|---|
| One item per 20 game ticks | `ticks 287..506: 11 activations, 11 items moved` — `floor(506/20) - floor(287/20) = 11`, and the destination grew by exactly 11 (the tick and the count are read in one command, `ticks=` and `dest=`) |
| A filter that does not match moves nothing | `whitelist dirt: 0 cobblestone moved in 60 ticks` |
| ... and a matching one does | `whitelist cobblestone: 2 moved`; then `blacklist cobblestone: still 2, nothing more` |
| `speed 38` is one item a tick | `ticks 943..979: 36 items, one a tick` (`T1 - T0` items over `T1 - T0` ticks) |
| `amount 63` is 64 items in one operation | `the first activation carried all 64` (5-tick polls cannot skip a 20-tick boundary) |
| `regulating 10` caps at four an activation and stops at ten | `after ~25 ticks: 8 on the target (never 10, never 64)`; `settled at exactly 10 and stayed` |
| Removal keeps the block and returns the item | `the chest stands, the label is gone, 3 -> 4 label items` (in creative an uncancelled click breaks the chest) |
| Labels survive a save and a relaunch | `5 transfer labels in this level.` after the relaunch, then `after the relaunch: 103 -> 128 moved by the same label` |
| The content really comes from the bundle | `embedded content: 5223 B … identical to content/transfer_labels.hmod`, and in the control `without the embedded file: no label item, the block stayed minecraft:air` |
| The scenario can fail | `e2e-selftest.log`: `FAIL: selftest: 11 items moved, expected the wrong count`, `EXIT:1` |

## The delivered bundle

`transfer_labels.hnmod` — 22,075 B, 12 entries, SHA-256
`65a1c37b2df77fea6142acb959704c15d819b12fc3b23e4e724a72e4f681344b`:

```
horizon-native.json
mixins/transfer_labels/{horizon-mixin.toml, runtime.rs, support.rs, commands.rs,
                        embedded_content.rs, left_click_block.rs, level_tick_pre.rs,
                        perform_command.rs, register_commands.rs, right_click_block.rs,
                        data/transfer_labels-content.hmod}
```

`tools/package.py` run a second time produced the same bytes (same SHA-256), the
sources inside are byte-identical to `mixins/transfer_labels/`, and the embedded
`data/transfer_labels-content.hmod` is byte-identical to `content/transfer_labels.hmod`
(checked with zipfile + sha256, 2026-11-11).
