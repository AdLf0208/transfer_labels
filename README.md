# Transfer Labels — a Horizon port

**Transfer Labels** is a Minecraft mod. This repository is a **port of it to [Horizon](https://github.com/horizon-rust/horizon)**, the Rust engine for Minecraft Java 26.3. It carries the complete source of the port plus the content files it ships, and installs as a single `.hmod` package.

Upstream: [Buuz135/Transfer-Labels](https://github.com/Buuz135/Transfer-Labels) — licence **MIT**.

| | |
|---|---|
| Upstream mod | Transfer Labels ([source](https://github.com/Buuz135/Transfer-Labels)) |
| Ported to | Horizon 0b14f8a, Minecraft Java 26.3, mod API `0.2` |
| Package | `transfer_labels.hmod` (install it with the Horizon launcher) |
| Licence | **MIT** — the upstream mod's licence; see [Licence and credit](#licence-and-credit) |

## What this port implements

### What this port does (slice 1)

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

## Not yet ported

- **No in-game run at `0b14f8a`.** There is no `horizon-launcher` on this box, so the package cannot
- **No log service in the SDK**, so the retired bundle's `[transfer_labels] …` lines are gone; the
- The slice's own gaps are unchanged: the insert label and the four fluid labels, the menu/screen

## Installing

1. Install the [Horizon launcher](https://github.com/horizon-rust/horizon/releases) (0.2.70 or newer).
2. Add `transfer_labels.hmod` to the instance's mods.
3. The package is pinned to a specific Horizon build; the launcher checks that pin.

## Licence and credit

Transfer Labels was created by its upstream authors; the code, models and assets in the original mod
remain theirs and are used here under **MIT**. The Rust in this repository is a clean-room
rewrite for Horizon, not a translation of the original source.

- `LICENSE` — the upstream licence text.
- `LICENSE-NOTICE.md` — what this port carries and where it came from.
- The upstream project: https://github.com/Buuz135/Transfer-Labels

## Technical details

```
horizon-mod.toml   package manifest (id, version, api, [native] pin)
Cargo.toml         the crate; depends on `horizon-mod-api` from the fetched SDK
src/               lib.rs (the one `impl Hooks`), runtime.rs (pure logic + tests), glue
assets/ data/      the content the package ships
tools/             content generators and the e2e scenario
```

- Build: `cargo horizon-mod build` (packages; it does not compile against the game).
- Test: `cargo test`, `cargo clippy --all-targets -- -D warnings`.
- In-game: `ports/tools/launcher-game.sh run transfer_labels` in the campaign tree, or install the
  `.hmod` and play; the harness scenario is `tools/e2e.sh`.

This repository is generated from the campaign's port at `ports/transfer_labels/`; the port's own
`notes.md` is the full record (decisions, traps, evidence).
