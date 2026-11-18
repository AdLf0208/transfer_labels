# Transfer Labels 0.1.9 — what the jar and the upstream source contain

Port of `transfer_labels` (`com.buuz135.transfer_labels`), read from

- the pack jar `ports/_shared/licence-cache/transfer_labels-0.1.9.jar`
  (127 entries, `unzip -l`; `META-INF/neoforge.mods.toml` says
  `license = "MIT"`, `modId = "transfer_labels"`, `version = "0.1.9"`,
  author `Buuz135`), and
- the public repository <https://github.com/Buuz135/Transfer-Labels>
  (`git clone --depth 1`, commit `e7b04d013cf8becedc9c644e67e9348662d6b7b3`,
  2026-08-03, matching the jar's build date).

No decompilation: this file records behaviour read from the public Java source
and the jar's data/assets, which the rewrite reimplements in Rust.

## 1. Jar inventory (the artefact, not the name)

| Kind | Entries |
|---|---|
| code | 44 `.class` files under `com/buuz135/transfer_labels/**` |
| items | **5 item models**, no block states, no block models: `itemstack_insert_transfer_label`, `itemstack_extract_transfer_label`, `fluidstack_insert_transfer_label`, `fluidstack_extract_transfer_label`, `label_accessor` |
| blocks | **none** — there is no `assets/transfer_labels/blockstates/`, no `loot_table/blocks/`, and `LabelBlock.java` is a *data* record (a position with a map of face → label), not a `Block` |
| textures | 5 item textures + 7 GUI textures (`textures/gui/textures.png`, `whitelist.png`, `blacklist.png`, `filter_normal/regulating/exact_count/mod/tag.png`) |
| recipes | 18: 4 labels (`crafting_shaped`, `result.count = 2`), 4 label→label conversions (`crafting_shapeless`, 1↔1), 1 `label_accessor`, 1 fluid variant conversion… all gated by `neoforge:conditions` |
| advancements | 9 recipe-unlock advancements under `advancement/recipes/misc/` |
| lang | `en_us`, `fr_fr`, `zh_cn` |
| mixins | `transfer_labels.mixins.json` (client-side ray tracing only) |

**Families found:** item-stack labels (insert + extract), fluid-stack labels
(insert + extract), a filter GUI (5 filter kinds, 20 slots, amount and speed
upgrade slots), a label `SavedData` with a client sync packet, and a client
ray-tracer that lets a player click a label's own thin shape from a distance.

The brief's inventory line (5 items, 18 recipes, no blocks) matches the
artefact exactly.

## 2. How a label works (behaviour extracted)

### 2.1 Attaching and removing — `item/TransferLabelItem.java`, `LabelInteractEvents.java`, `storage/LabelStorage.java`

- `useOn` (right click a block with a label item): `LabelStorage.addLabel(level,
  clickedPos, clickedFace, stack.copyWithCount(1))` then `stack.shrink(1)`.
  **The label is attached to a block face, and stored in level-scoped SavedData
  (`TRANSFER_LABELS_LABEL_STORAGE`), not in a block entity.** One `LabelBlock`
  per position, a map `Direction → LabelInstance` inside it.
- Left click a label (or right click it, holding a label or the accessor): the
  mod's own client ray trace finds the label's thin voxel shape
  (`client/LabelShapes.java`, `client/RayTraceUtils.java`) and either opens its
  container menu (right) or removes it (left). `LabelBlock.remove` returns the
  label stack **plus the copies held in the amount and speed upgrade slots**.
- `LabelStorage.getNearbyLabels(level, pos, distance)`: a label can be clicked
  from up to `BLOCK_INTERACTION_RANGE²` blocks away.

### 2.2 The tick — `LabelInteractEvents.onTick(LevelTickEvent.Pre)`

```
for each LabelBlock whose pos is loaded and ticks:
    for each (direction, label):
        label.work(level)
```

`LabelInstance.work`:

```
if filter != null and level.getGameTime() % (20 - getSpeed()) == 0:
    filter.work(level, pos, facing, Config.baseItemTransferAmount + getAmount())
```

- `getSpeed() = min(speedSlot.count, Config.speedUpgrades) / 2` → the interval is
  `20 - speed` ticks: **20 ticks with no upgrade**, 19 at 2 upgrade items,
  **1 tick at 38** (integer division, so 1 upgrade item changes nothing).
- `getAmount() = min(amountSlot.count, Config.amountUpgrades)` → **1 item per
  operation with no upgrade, up to 64 at 63 items**.
- The upgrade "slots" accept only **copies of the same label item**
  (`ItemStack.isSameItem(stack, label)`).
- Base config (`Config.java`): `baseItemTransferAmount = 1`,
  `amountUpgrades = 63`, `speedUpgrades = 38`, `fluidTransferMultiplier = 100`,
  `filterSlots = 20`.

### 2.3 Which two blocks — `filter/ItemFilter.work`

```
oppositeDirection = direction.getOpposite();
oppositePos       = pos.relative(direction);
if EXTRACT: source = handler(pos, direction);          target = handler(oppositePos, oppositeDirection)
else:       source = handler(oppositePos, oppositeDirection); target = handler(pos, direction)
if either is null: return
```

So a label sits **between two blocks**: the block it is attached to, on the
attached face, and the block on the other side of that face.
`itemstack_extract_transfer_label` pulls from the labelled block and pushes into
the neighbour; `itemstack_insert_transfer_label` pushes into the labelled block.

### 2.4 The transfer — `ItemFilter.transferItems` / `calculateMaxTransferAmount`

- Walk the source's slots **through the labelled face**, in slot order.
- `extractItem(slot, maxAmount, true)` (simulated); empty → next slot.
- `passesFilter(stack) == false` → next slot.
- `maxAmount` (`calculateMaxTransferAmount`):
  - `normal`, `mod`, `tag`: the default `1 + amountUpgrades`.
  - `regulating`: find the filter entry for this item; count how much of it the
    **target** already holds; `maxAmount = min(max(0, desired - held), 4)` — a
    hard cap of **4 per operation** whatever the upgrades.
  - `exact_count`: `maxAmount = exactAmount` when the source can supply it
    (across several slots), else 0; when the target cannot take all of it, the
    transfer is skipped entirely.
- Simulate insertion into the target; extract and insert for real; then `break`:
  **at most one item type moves per activation** (except the `exact_count`
  multi-slot gather).

### 2.5 Filter semantics — `passesFilter`, `filter/FilterType.java`, `ILabelFilter`

- `type` is the whitelist/blacklist *mode*: `Type.BLACKLIST` is the default, and
  `passesFilter` ends with `type.getFilter().test(matches)` — a blacklist lets
  everything through that does *not* match, so **a fresh label with no filter
  entries transfers everything**.
- `FilterType` (the *kind*): `normal` (same item, components included —
  `ItemStack.isSameItem`), `regulating` (same item, keep N on the destination),
  `exact_count` (same item, move exactly N), `mod` (same namespace),
  `tag` (same item tag).
- 20 filter slots (`Config.filterSlots`); an entry stores the filter stack and,
  for `regulating`/`exact_count`, a per-slot number that the GUI scrolls
  (`filter/extras/NumberFilterExtra.java`).

### 2.6 Everything else

- `transfer_labels.mixins.json` + `client/*` are **client** ray tracing and HUD
  only; they give no gameplay behaviour.
- `container/TransferLabelBasicAddonContainer.java` + `screen/` + `gui/` are the
  Titanium addon GUI (filter grid, whitelist button, filter-kind scroll, amount
  and speed slots) and the two sync packets.
- The `label_accessor` item only opens that GUI from a distance (it is a plain
  `Item` with `stacksTo(1)`).

## 3. Recipes (as shipped, 1.21.1 shapes)

| Result | Shape |
|---|---|
| `itemstack_extract_transfer_label` ×2 | `" R " / " C " / " H "` — `R` = `minecraft:redstone`, `C` = tag `c:chests/wooden`, `H` = `minecraft:hopper` |
| `itemstack_insert_transfer_label` ×2 | `" H " / " C " / " R "` — the same three keys, hopper and redstone swapped |
| `fluidstack_extract_transfer_label` / `..._insert_...` ×2 | the same two patterns with `C` = `minecraft:bucket` |
| 4 label→label conversions ×1 | `crafting_shapeless`, one label in, the other out |
| `label_accessor` ×1 | `"RCR" / " R " / " R "`, `R` = tag `c:ingots/copper`, `C` = any of the four labels |

The carried recipe in `content/` is the extract label's, in the 26.3 shape.

## 4. Where the rewrite differs (see `notes.md` for the slice boundary)

- **No block entity.** Upstream keeps labels in level SavedData; at this pin the
  faithful equivalent is a **level attachment**
  (`AttachmentType<LabelStore>` under `Holder::Level`, saved in
  `data/minecraft/horizon_attachments.dat`), with the transfer run from
  `level_tick_pre`. The brief guessed "an attached block entity"; the artefact
  says the labels have no block of their own, and a native mod cannot add a
  block entity to a vanilla chest.
- **No client ray tracing and no GUI.** The label's own thin voxel shape cannot
  be clicked in Horizon (no per-position client shape for a non-block), so a
  label is attached by right-clicking the *block face* it belongs to, before the
  neighbouring container is placed over it, and configured through the mod's
  command instead of the Titanium GUI.
- **Items move through the pin's hopper-rules helpers** (`native_move_items`),
  one by one, so a refused item stays in its slot and nothing is ever lost —
  upstream's simulated insert does the same job through capabilities.
