# transfer_labels (Horizon port)

A Horizon port of [transfer_labels](Buuz135/Transfer-Labels) — MIT.

**Status: first slices.** What is below is what the port actually does today.

## Implemented so far

- Implemented:** the **item-stack extract label**
- (`transfer_labels:itemstack_extract_transfer_label`).
- Attached by right-clicking a block face with the label in hand (one item used
- up); removed by left-clicking the labelled face with the label in hand (the
- item, and the copies its upgrades stand for, come back). Cancelling the click
- is what keeps a chest from opening and the block from breaking.
- Runs on `level_tick_pre`, once per label whose position the level reaches,
- `gameTime % (20 - min(speed,38)/2) == 0` — **one item every 20 ticks** with no

## Not yet ported

- nothing recorded yet

## Install

Build the bundle with a native-mod capable launcher, or add the file from the
registry once it is listed. Pinned to Horizon `1d63e0a` and horizon-mixins
`5a64af93` for game 26.3.

## Build from source

```sh
cargo test
python3 tools/horizon.py check --horizon <horizon checkout>
python3 tools/horizon.py build --horizon <horizon checkout>
python3 tools/package.py
```

## Credits and licence

Original mod: Buuz135/Transfer-Labels. Port: AdLf0208. Licence: MIT — see [LICENSE](LICENSE);
the original notices are preserved.
