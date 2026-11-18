# Validation — Transfer Labels slice 1

> **History (retired `.hnmod` bundle).** Everything below was measured at Horizon `1d63e0a5` /
> horizon-mixins `5a64af93` for the `.hnmod` container, which Horizon retired (#16941). The port is
> now a flat SDK `.hmod` at `0b14f8a` / `e59da90`: see `notes.md`, **`## Migration to the SDK
> format`**, for the current evidence (`evidence/transfer_labels/sdk-0b14f8a/`). The in-game runs
> below remain the behaviour's proof until a launcher-built game exists here.

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
