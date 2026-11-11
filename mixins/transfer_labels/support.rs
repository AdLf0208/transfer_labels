// Game-side glue for Transfer Labels slice 1.
//
// Injected into `crate::game` as `transfer_labels_state`; it needs the game, so
// it is not part of the standalone library (only `runtime.rs` is).
//
// A label lives in the level, not in a block entity: upstream keeps it in
// `LabelStorage`, a `SavedData` keyed by block position with one label per
// face (`LabelBlock.labels`). At this pin the faithful equivalent is one level
// attachment (API "Server attachments"), `transfer_labels:labels` under
// `Holder::Level`, saved in the dimension's `data/horizon/attachments.dat`.
//
// The tick is upstream's `LevelTickEvent.Pre`: for every label in the level,
// if the level reaches its position, `LabelInstance.work` runs the filter's
// `work` on `gameTime % (20 - speed) == 0`. The transfer itself is
// `ItemFilter.work` with an EXTRACT label: the source is the labelled block
// through the labelled face, the target the block across it, and the pin's
// hopper-rules helpers (`native_container_slots`, `native_move_items`) do the
// moving so a refused item stays where it was.
//
// Read from Transfer Labels 0.1.9 (`LabelInteractEvents`, `LabelStorage`,
// `LabelBlock`, `LabelInstance`, `ItemFilter`). See `../../FINDINGS.md`.

use horizon_client::mod_api::{AttachmentCodec, AttachmentError, AttachmentType, Holder};
use horizon_client::text::Text;
use horizon_core::geometry::Direction;
use horizon_world::nbt::{Compound, JavaString, Tag};

use crate::game::Game;
use crate::game::native_server::{NativeCommand, NativeServerPlayer};
use crate::game::transfer_labels::{
    AMOUNT_UPGRADES, Command, Face, FilterEntry, LABEL_ITEM, LabelConfig, LabelKey, LabelStore,
    SPEED_UPGRADES, Slot,
};

/// The attachment that holds every label of one level.
static LABELS: AttachmentType<LabelStore> = AttachmentType {
    namespace: "transfer_labels",
    name: "labels",
    default: LabelStore::default,
    codec: Some(AttachmentCodec {
        encode: encode_store,
        decode: decode_store,
    }),
    copy_on_death: false,
    sync: false,
};

/// The command root, as the chat tree and `perform_command` see it.
pub const COMMAND: &str = "transfer_labels";

// ---------------------------------------------------------------------------
// The saved form (`LabelStorage.save`/`load`)
// ---------------------------------------------------------------------------

fn tag_key(name: &str) -> JavaString {
    JavaString::from(name)
}

fn encode_store(store: &LabelStore) -> Result<Compound, AttachmentError> {
    let mut labels = Compound::new();
    for (key, text) in store.encode() {
        labels.insert(
            tag_key(&key),
            Tag::String(JavaString::from(text.as_str())),
        );
    }
    let mut root = Compound::new();
    root.insert(tag_key("labels"), Tag::Compound(labels));
    Ok(root)
}

fn decode_store(root: &Compound) -> Result<LabelStore, AttachmentError> {
    match root.get(&tag_key("labels")) {
        None => Ok(LabelStore::default()),
        Some(Tag::Compound(labels)) => {
            let records: Vec<(String, String)> = labels
                .iter()
                .filter_map(|(key, tag)| match tag {
                    Tag::String(text) => {
                        Some((key.to_string_lossy(), text.to_string_lossy()))
                    }
                    _ => None,
                })
                .collect();
            Ok(LabelStore::decode(&records))
        }
        Some(_) => Err(AttachmentError::Decode),
    }
}

/// The level's labels, or an empty store where the level has none yet.
fn read_labels(game: &mut Game<'_>) -> LabelStore {
    game.native_attachments(Holder::Level)
        .and_then(|attachments| attachments.get_existing(&LABELS))
        .ok()
        .flatten()
        .unwrap_or_default()
}

/// Write the level's labels back; false where the level cannot hold them.
fn write_labels(game: &mut Game<'_>, store: &LabelStore) -> bool {
    game.native_attachments(Holder::Level)
        .map(|attachments| attachments.set(&LABELS, store.clone()).is_ok())
        .unwrap_or(false)
}

// ---------------------------------------------------------------------------
// The tick (`LevelTickEvent.Pre` → `LabelInstance.work` → `ItemFilter.work`)
// ---------------------------------------------------------------------------

/// One server level's tick: run every due label in it (API 4).
#[allow(
    private_interfaces,
    reason = "the injected bundle is this crate's own module"
)]
pub fn on_level_tick_pre(game: &mut Game<'_>, dimension: &str) {
    let store = read_labels(game);
    if store.is_empty() {
        return;
    }
    let Some(level) = game.native_level(dimension) else {
        return;
    };
    let Some(now) = game.native_level_game_time(level) else {
        return;
    };
    let due: Vec<LabelKey> = store
        .keys()
        .filter(|key| store.get(key).is_some_and(|label| label.is_due(now)))
        .copied()
        .collect();
    for key in due {
        let Some(label) = store.get(&key) else {
            continue;
        };
        let target = key.target();
        if !game.native_level_is_loaded(level, key.pos)
            || !game.native_level_is_loaded(level, target)
        {
            continue;
        }
        let from_face = direction(key.from_face());
        let to_face = direction(key.to_face());
        let Some(source_slots) = game.native_container_slots(dimension, key.pos, from_face) else {
            continue;
        };
        let Some(target_slots) = game.native_container_slots(dimension, target, to_face) else {
            continue;
        };
        let source: Vec<Slot> = source_slots
            .iter()
            .filter_map(|held| {
                let stack = held.stack.as_ref()?;
                Some(Slot {
                    item: stack.item.to_string(),
                    count: stack.count,
                    can_take: held.can_take,
                })
            })
            .collect();
        let counts = counted(&target_slots);
        let Some(plan) = label.plan(&source, &counts) else {
            continue;
        };
        game.native_move_items(
            dimension,
            key.pos,
            from_face,
            target,
            to_face,
            Some(&plan.item),
            plan.amount,
        );
    }
}

/// How many of each item a side of the destination already holds
/// (`ItemFilter.calculateMaxTransferAmount`'s count for `regulating`).
fn counted(slots: &[crate::game::machines::native_items::NativeContainerSlot]) -> Vec<(String, u32)> {
    let mut counts: Vec<(String, u32)> = Vec::new();
    for stack in slots.iter().filter_map(|held| held.stack.as_ref()) {
        let item = stack.item.to_string();
        match counts.iter_mut().find(|(other, _)| *other == item) {
            Some((_, count)) => *count += stack.count,
            None => counts.push((item, stack.count)),
        }
    }
    counts
}

/// A face's direction, as the container helpers take it.
fn direction(face: Face) -> Direction {
    match face {
        Face::Down => Direction::Down,
        Face::Up => Direction::Up,
        Face::North => Direction::North,
        Face::South => Direction::South,
        Face::West => Direction::West,
        Face::East => Direction::East,
    }
}

/// The face a hook's `[i32; 3]` normal names.
fn face_of(normal: [i32; 3]) -> Option<Face> {
    Face::ALL
        .into_iter()
        .find(|face| face.offset() == normal)
}

// ---------------------------------------------------------------------------
// Attaching and removing (`TransferLabelItem.useOn`, `LabelBlock.remove`)
// ---------------------------------------------------------------------------

/// A label was placed with the label item in hand (API 21): it sticks to that
/// block face, and one item is used up. The block's own use is stopped, as
/// upstream cancels the click so a chest does not open behind the label.
#[allow(
    private_interfaces,
    reason = "the injected bundle is this crate's own module"
)]
pub fn on_right_click_block(game: &mut Game<'_>, pos: [i32; 3], face: [i32; 3], canceled: &mut bool) {
    let Some(held) = game.native_held() else {
        return;
    };
    if held.0 != LABEL_ITEM {
        return;
    }
    let Some(face) = face_of(face) else {
        return;
    };
    let key = LabelKey { pos, face };
    let mut store = read_labels(game);
    if store.get(&key).is_some() {
        // Upstream opens the label's own menu here; this slice has no menu, and
        // replacing the label would throw away its filter, so it stays.
        *canceled = true;
        game.native_feedback(Text::literal(format!(
            "A label already sits on the {} face of {}, {}, {}: \
             /transfer_labels info {} {} {} {} shows it.",
            face.name(),
            pos[0],
            pos[1],
            pos[2],
            pos[0],
            pos[1],
            pos[2],
            face.name()
        )));
        return;
    }
    store.insert(key, LabelConfig::default());
    if !write_labels(game, &store) {
        return;
    }
    if game.native_take_items(LABEL_ITEM, 1) == 0 {
        store.remove(&key);
        write_labels(game, &store);
        return;
    }
    *canceled = true;
    game.native_feedback(Text::literal(format!(
        "Transfer label attached to the {} face of {}, {}, {}.",
        face.name(),
        pos[0],
        pos[1],
        pos[2]
    )));
}

/// A label was hit with the label item in hand (API 30): it comes off, with the
/// copies the amount and speed upgrades held.
#[allow(
    private_interfaces,
    reason = "the injected bundle is this crate's own module"
)]
pub fn on_left_click_block(game: &mut Game<'_>, pos: [i32; 3], face: [i32; 3], canceled: &mut bool) {
    let Some(held) = game.native_held() else {
        return;
    };
    if held.0 != LABEL_ITEM {
        return;
    }
    let Some(face) = face_of(face) else {
        return;
    };
    let key = LabelKey { pos, face };
    let mut store = read_labels(game);
    let Some(label) = store.remove(&key) else {
        return;
    };
    if !write_labels(game, &store) {
        return;
    }
    let back = 1 + label.amount_upgrades + label.speed_upgrades;
    let given = game.native_give_items(LABEL_ITEM, back);
    *canceled = true;
    game.native_feedback(Text::literal(format!(
        "Transfer label removed from the {} face of {}, {}, {} ({given} of {back} returned).",
        face.name(),
        pos[0],
        pos[1],
        pos[2]
    )));
}

// ---------------------------------------------------------------------------
// `/transfer_labels …`
// ---------------------------------------------------------------------------

/// The command's name, for the chat tree's completion (API 1).
pub fn on_commands(commands: &mut Vec<&'static str>) {
    commands.push(COMMAND);
}

/// The command's usage lines (API 8).
pub fn on_register_commands(commands: &mut Vec<NativeCommand>) {
    commands.push(NativeCommand::new(
        "transfer_labels list",
    ));
    commands.push(NativeCommand::new(
        "transfer_labels info <pos:block_pos> <face:word>",
    ));
    commands.push(NativeCommand::new(
        "transfer_labels filter <pos:block_pos> <face:word> <mode:word>",
    ));
    commands.push(NativeCommand::new(
        "transfer_labels type <pos:block_pos> <face:word> <kind:word>",
    ));
    commands.push(NativeCommand::new(
        "transfer_labels entry <pos:block_pos> <face:word> <slot:integer> <item:word> [count:integer]",
    ));
    commands.push(NativeCommand::new(
        "transfer_labels clear <pos:block_pos> <face:word> <slot:integer>",
    ));
    commands.push(NativeCommand::new(
        "transfer_labels amount <pos:block_pos> <face:word> <amount:integer>",
    ));
    commands.push(NativeCommand::new(
        "transfer_labels speed <pos:block_pos> <face:word> <speed:integer>",
    ));
    commands.push(NativeCommand::new(
        "transfer_labels remove <pos:block_pos> <face:word>",
    ));
}

/// Run `/transfer_labels …` for the acting player (API 8).
#[allow(
    private_interfaces,
    reason = "the injected bundle is this crate's own module"
)]
pub fn on_perform_command(
    game: &mut Game<'_>,
    _player: &NativeServerPlayer,
    command: &str,
    ran: &mut bool,
) {
    let words: Vec<&str> = command.split_whitespace().collect();
    if words.first() != Some(&COMMAND) {
        return;
    }
    *ran = true;
    match Command::parse(&words) {
        Err(usage) => game.native_feedback(Text::literal(usage)),
        Ok(parsed) => run_command(game, parsed),
    }
}

fn run_command(game: &mut Game<'_>, command: Command) {
    match command {
        Command::List => list(game),
        Command::Info { key } => info(game, key),
        Command::Remove { key } => remove(game, key),
        Command::Filter { key, mode } => edit(game, key, |label| label.mode = mode),
        Command::Kind { key, kind } => edit(game, key, |label| label.kind = kind),
        Command::Entry {
            key,
            slot,
            item,
            count,
        } => edit(game, key, |label| {
            label.entries[slot] = Some(FilterEntry { item, count });
        }),
        Command::Clear { key, slot } => edit(game, key, |label| label.entries[slot] = None),
        Command::Amount { key, amount } => edit(game, key, |label| {
            label.amount_upgrades = amount.min(AMOUNT_UPGRADES);
        }),
        Command::Speed { key, speed } => edit(game, key, |label| {
            label.speed_upgrades = speed.min(SPEED_UPGRADES);
        }),
    }
}

fn edit(game: &mut Game<'_>, key: LabelKey, change: impl FnOnce(&mut LabelConfig)) {
    let mut store = read_labels(game);
    let Some(label) = store.get_mut(&key) else {
        game.native_feedback(Text::literal(format!(
            "No transfer label on the {} face of {}, {}, {}.",
            key.face.name(),
            key.pos[0],
            key.pos[1],
            key.pos[2]
        )));
        return;
    };
    change(label);
    let line = describe(&key, label);
    if write_labels(game, &store) {
        game.native_feedback(Text::literal(line));
    }
}

fn remove(game: &mut Game<'_>, key: LabelKey) {
    let mut store = read_labels(game);
    let Some(label) = store.remove(&key) else {
        game.native_feedback(Text::literal(format!(
            "No transfer label on the {} face of {}, {}, {}.",
            key.face.name(),
            key.pos[0],
            key.pos[1],
            key.pos[2]
        )));
        return;
    };
    let back = 1 + label.amount_upgrades + label.speed_upgrades;
    let given = game.native_give_items(LABEL_ITEM, back);
    if write_labels(game, &store) {
        game.native_feedback(Text::literal(format!(
            "Transfer label at {} removed ({given} of {back} label items returned).",
            key.text()
        )));
    }
}

fn list(game: &mut Game<'_>) {
    let store = read_labels(game);
    if store.is_empty() {
        game.native_feedback(Text::literal("No transfer labels in this level."));
        return;
    }
    game.native_feedback(Text::literal(format!(
        "{} transfer labels in this level.",
        store.len()
    )));
    for key in store.keys().copied().collect::<Vec<_>>() {
        if let Some(label) = store.get(&key) {
            game.native_feedback(Text::literal(describe(&key, label)));
        }
    }
}

fn info(game: &mut Game<'_>, key: LabelKey) {
    let store = read_labels(game);
    let Some(label) = store.get(&key) else {
        game.native_feedback(Text::literal(format!(
            "No transfer label on the {} face of {}, {}, {}.",
            key.face.name(),
            key.pos[0],
            key.pos[1],
            key.pos[2]
        )));
        return;
    };
    let dimension = game.native_dimension();
    let ticks = game
        .native_level(dimension)
        .and_then(|level| game.native_level_game_time(level))
        .unwrap_or(-1);
    // The destination's items, read in the same tick as `ticks`, so a harness
    // can compare what moved with the ticks that passed.
    let dest: u32 = game
        .native_container_slots(dimension, key.target(), direction(key.to_face()))
        .map(|slots| slots.iter().filter_map(|held| held.stack.as_ref().map(|stack| stack.count)).sum())
        .unwrap_or(0);
    game.native_feedback(Text::literal(format!(
        "{} ticks={ticks} dest={dest}",
        describe(&key, label)
    )));
}

/// One label, as `/transfer_labels list|info` prints it. The fields are what the
/// e2e asserts on, so they stay `key=value`.
fn describe(key: &LabelKey, label: &LabelConfig) -> String {
    let entries: Vec<String> = label
        .entries
        .iter()
        .enumerate()
        .filter_map(|(slot, entry)| {
            entry
                .as_ref()
                .map(|entry| format!("slot{slot}={},{}", entry.item, entry.count))
        })
        .collect();
    format!(
        "label {} at {},{},{} mode={} type={} amount={} speed={} interval={} per_op={} entries={}{}",
        key.face.name(),
        key.pos[0],
        key.pos[1],
        key.pos[2],
        label.mode.name(),
        label.kind.name(),
        label.amount_upgrades,
        label.speed_upgrades,
        label.interval_ticks(),
        label.transfer_amount(),
        entries.len(),
        if entries.is_empty() {
            String::new()
        } else {
            format!(" [{}]", entries.join(" "))
        }
    )
}
