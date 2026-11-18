# Transfer Labels for Horizon

A Horizon native port of [Transfer Labels](https://github.com/Buuz135/Transfer-Labels)
by Buuz135 (MIT), Minecraft Java 1.21.1 NeoForge → Horizon (Minecraft 26.3).

A label sticks to a block face and, on its own tick, moves items between the
block it is attached to and the block on the other side of that face.

This is a **flat SDK package** (mod API `0.2`): one `.hmod` carries the code
(`src/`) and the item's content (`assets/`, `data/`) together.

## What this port does (slice 1)

- **`transfer_labels:itemstack_extract_transfer_label`** — craft it (hopper,
  wooden chest, redstone), right-click a block face with it to attach it, and it
  pulls from that block into the container across the face.
- One item every 20 game ticks; `speed` upgrades shorten the interval to a tick
  at 38, `amount` upgrades raise the haul to 64 items an operation.
- A 20-slot filter: whitelist or blacklist (blacklist by default), matching the
  same item (`normal`), the same namespace (`mod`) or keeping a count on the
  destination (`regulating`, at most four items an operation).
- Labels are level state, saved with the world.
- Left-click a label with the label item in hand to take it back.

## Commands

```
/transfer_labels list
/transfer_labels info   <x> <y> <z> <face>
/transfer_labels filter <x> <y> <z> <face> <whitelist|blacklist>
/transfer_labels type   <x> <y> <z> <face> <normal|regulating|mod>
/transfer_labels entry  <x> <y> <z> <face> <slot 0..19> <item> [count]
/transfer_labels clear  <x> <y> <z> <face> <slot>
/transfer_labels amount <x> <y> <z> <face> <0..63>
/transfer_labels speed  <x> <y> <z> <face> <0..38>
/transfer_labels remove <x> <y> <z> <face>
```

## Not in this slice

The insert label, the four fluid labels, the filter GUI and the amount/speed
upgrade slots, and the `exact_count`/`tag` filter kinds — see
[`notes.md`](notes.md) for each gap and why it is one.

## Installing

One file: `transfer_labels.hmod`. The launcher builds it against the game build
its manifest pins (`[native]` in `horizon-mod.toml`).

## Layout

| Path | What |
|---|---|
| `horizon-mod.toml` | the only manifest and the only identity: id, version, `api = "0.2"`, side, capabilities, the `[native]` pins |
| `Cargo.toml` | one crate; its only dependency is `horizon-mod-api` from the SDK the tool fetches into `.horizon/` |
| `src/runtime.rs` | the pure rules (rate, amount, filter, saved text, command grammar) and their 28 tests |
| `src/lib.rs` | the one `impl mod_api::Hooks`: the level tick, the two clicks, the command |
| `assets/`, `data/` | the item, its model/texture/lang, recipe, creative tab and tag |
| `tools/e2e.sh`, `tools/parse_info.py` | the in-game scenario and its reply parser |
| `FINDINGS.md` | what the jar and the upstream source contain |
| `notes.md` | scope, gaps, hooks, the migration recipe, verification |

## Building

```sh
ports/tools/build-slot.sh run --job sdk-tl --port ports/transfer_labels --step test \
    -- bash -c 'cd ports/transfer_labels && cargo test --offline'
ports/tools/build-slot.sh run --job sdk-tl --port ports/transfer_labels --step build \
    -- bash -c 'cd ports/transfer_labels && cargo horizon-mod build'
ports/tools/build-slot.sh run --job sdk-tl --port ports/transfer_labels --step check \
    -- bash -c 'cd ports/transfer_labels && cargo horizon-mod check transfer_labels.hmod'
```

`cargo horizon-mod` fetches the pinned SDK into `.horizon/sdk/` on first use; the
delivered file is `transfer_labels.hmod` in this folder, built and checked.

## Licence

MIT (see `LICENSE`, `LICENSE-NOTICE.md`, both inside the package). Local-only
until the maintainer approves publishing.
