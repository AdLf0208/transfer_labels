//! Transfer Labels for Horizon: a label on a block face moves items between two containers.
//!
//! Upstream (see `FINDINGS.md`) keeps its labels in a level `SavedData` and works one on
//! `LevelTickEvent.Pre`; attaches one with `TransferLabelItem.useOn`; removes one from
//! `LabelInteractEvents`; and configures one in a Titanium GUI. Here a label is a level
//! [`AttachmentType`], the tick is [`Hooks::level_tick_pre`], attaching and removing are
//! [`Hooks::right_click_block`] / [`Hooks::left_click_block`], and `/transfer_labels` stands in
//! for the GUI (`notes.md` carries that gap).
//!
//! The label's rules — its rate, its amount, its filter, the regulating budget, the block pair
//! it joins, its saved text and the command grammar — live in [`runtime`], byte-identical to
//! the retired `.hnmod` bundle's `runtime.rs`. This file only converts between the SDK's values
//! and the runtime's: an item handler's slots, a clicked face, a parsed command.

pub mod runtime;

use mod_api::{
    self, AttachmentCodec, AttachmentError, AttachmentType, BlockClick, Capability,
    CommandArgumentValue, CommandInvocation, CommandSource, Compound, Direction, Flow, Hand,
    Holder, Hooks, HostSide, HostTransaction, ITEMS, Identifier, InteractionResult, ItemKey,
    ItemResource, ItemStack, LeftClickBlockEvent, LevelRef, LevelTickPreEvent, PlayerRef,
    RegisterCommandsEvent, ResourceHandler, RightClickBlockEvent, RunModCommandEvent, ServerHost,
    Tag, Text, move_resources,
};

use crate::runtime::{
    AMOUNT_UPGRADES, Command, Face, FilterEntry, LIST_PAGE_SIZE, LabelConfig, LabelFlow, LabelKey,
    LabelStore, SPEED_UPGRADES, Slot, list_page, within_reach,
};

/// The command root, as the command tree and `/transfer_labels …` spell it.
const COMMAND: &str = "transfer_labels";

/// How many label items one removal may hand back at most (`1 + 63 + 38`).
const MAX_GIVEN: u32 = 1 + AMOUNT_UPGRADES + SPEED_UPGRADES;

/// The reach a player with no `minecraft:block_interaction_range` attribute is treated as
/// having: vanilla's 4.5 blocks.
const REACH_FALLBACK: f64 = 4.5;

/// What one stack gives back when the game has no answer for the item's own limit.
const FALLBACK_MAX_STACK: u32 = 64;

/// The command's usages, as the retired bundle declared them (API 8). The argument kinds are
/// the ones its grammar reads: a block position, a face word, and words the runtime validates
/// itself, so a caller that mistypes still gets the runtime's refusal rather than a parser's.
const USAGES: [&str; 9] = [
    "transfer_labels list",
    "transfer_labels info <pos:block_pos> <face:word>",
    "transfer_labels filter <pos:block_pos> <face:word> <mode:word>",
    "transfer_labels type <pos:block_pos> <face:word> <kind:word>",
    "transfer_labels entry <pos:block_pos> <face:word> <slot:integer> <item:id> [count:integer]",
    "transfer_labels clear <pos:block_pos> <face:word> <slot:integer>",
    "transfer_labels amount <pos:block_pos> <face:word> <amount:integer>",
    "transfer_labels speed <pos:block_pos> <face:word> <speed:integer>",
    "transfer_labels remove <pos:block_pos> <face:word>",
];

/// Every label of one level: upstream's `LabelStorage` `SavedData`, as a level attachment.
///
/// The holder is the level the game has in place — during `level_tick_pre` that is the ticking
/// level, and inside a player's hook or command it is that player's level — so every dimension
/// keeps its own labels, as upstream's per-level `SavedData` does.
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

/// The mod keeps no state of its own: every label lives in the level's attachment, so one
/// `Default` instance serves the session.
#[derive(Default)]
struct Mod;

#[mod_api::horizon_mod]
impl Hooks for Mod {
    /// A level's tick pass (upstream's `LevelTickEvent.Pre`): every due label in that level
    /// moves its items. Only the server owns the labels and the containers.
    fn level_tick_pre(&self, host: HostSide, event: &mut LevelTickPreEvent) {
        let HostSide::Server(host) = host else {
            return;
        };
        tick_level(host, event.level);
    }

    /// A right click with the label item in hand (`TransferLabelItem.useOn`): the label sticks
    /// to the clicked face, one item is used up, and the click stops there so a chest behind it
    /// does not open. `SuccessServer` is vanilla's server-side `SUCCESS`: the server swings the
    /// hand, which the retired bundle did not do.
    fn right_click_block(&self, host: HostSide, event: &mut RightClickBlockEvent<'_>) -> Flow {
        let HostSide::Server(host) = host else {
            return Flow::Continue;
        };
        if !attach(host, event.click) {
            return Flow::Continue;
        }
        *event.cancellation_result = InteractionResult::SuccessServer;
        Flow::Cancel
    }

    /// A left click on a labelled face with the label item in hand (`LabelBlock.remove`): the
    /// label comes off with the copies its upgrades stood for, and the block is not broken.
    fn left_click_block(&self, host: &mut dyn ServerHost, event: &mut LeftClickBlockEvent) -> Flow {
        if remove_clicked(host, event.pos, event.face) {
            Flow::Cancel
        } else {
            Flow::Continue
        }
    }

    /// Declares `/transfer_labels …`, the configuration path this slice has instead of the GUI.
    fn register_commands(&self, _host: &mut dyn ServerHost, event: &mut RegisterCommandsEvent<'_>) {
        for usage in USAGES {
            event.commands.push(mod_api::Command::new(usage));
        }
    }

    /// Runs `/transfer_labels …` for the player who typed it. The dispatcher has already
    /// resolved the arguments; they are turned back into the words the runtime's grammar reads.
    fn run_mod_command(
        &self,
        host: &mut dyn ServerHost,
        event: &mut RunModCommandEvent<'_>,
    ) -> Flow {
        let CommandSource::Player { player, .. } = *event.source else {
            return Flow::Continue;
        };
        let words = words_of(event.command);
        if words.first().map(String::as_str) != Some(COMMAND) {
            return Flow::Continue;
        }
        let borrowed: Vec<&str> = words.iter().map(String::as_str).collect();
        match Command::parse(&borrowed) {
            Err(usage) => tell(host, player, usage),
            Ok(command) => run_command(host, player, command),
        }
        Flow::Cancel
    }
}

// ---------------------------------------------------------------------------
// The level's labels (`LabelStorage.save`/`load`)
// ---------------------------------------------------------------------------

/// A compound's member by name. `Compound` is keyed by the engine's `JavaString`, which a mod
/// cannot build, so the lookup decodes the keys instead.
fn member<'a>(compound: &'a Compound, name: &str) -> Option<&'a Tag> {
    compound
        .iter()
        .find(|(key, _)| key.to_string_lossy() == name)
        .map(|(_, value)| value)
}

/// `LabelStorage.save`: one string per label, keyed by its position and face.
fn encode_store(store: &LabelStore) -> Result<Compound, AttachmentError> {
    let records = store.encode();
    let labels = Tag::compound(
        records
            .iter()
            .map(|(key, text)| (key.as_str(), Tag::String(text.as_str().into()))),
    );
    match Tag::compound([("labels", labels)]) {
        Tag::Compound(root) => Ok(root),
        _ => Err(AttachmentError::Encode),
    }
}

/// `LabelStorage.load`: a record this version cannot read is dropped, as upstream's loader
/// keeps going; a `labels` member of the wrong shape is a decode error, not an empty store.
fn decode_store(root: &Compound) -> Result<LabelStore, AttachmentError> {
    match member(root, "labels") {
        None => Ok(LabelStore::default()),
        Some(Tag::Compound(labels)) => {
            let records: Vec<(String, String)> = labels
                .iter()
                .filter_map(|(key, tag)| match tag {
                    Tag::String(text) => Some((key.to_string_lossy(), text.to_string_lossy())),
                    _ => None,
                })
                .collect();
            Ok(LabelStore::decode(&records))
        }
        Some(_) => Err(AttachmentError::Decode),
    }
}

/// The level's labels, or an empty store where it has none yet.
fn read_labels(host: &mut dyn ServerHost) -> LabelStore {
    LABELS
        .get_existing(host.attachments(), Holder::Level)
        .ok()
        .flatten()
        .unwrap_or_default()
}

/// Writes the level's labels back; false where the level cannot hold them.
fn write_labels(host: &mut dyn ServerHost, store: &LabelStore) -> bool {
    LABELS
        .set(host.attachments(), Holder::Level, store.clone())
        .is_ok()
}

// ---------------------------------------------------------------------------
// The tick (`LabelInstance.work` → `ItemFilter.work`)
// ---------------------------------------------------------------------------

/// One server level's tick: run every label whose interval the level's game time reaches.
fn tick_level(host: &mut dyn ServerHost, level: LevelRef) {
    let store = read_labels(host);
    if store.is_empty() {
        return;
    }
    let Some(now) = level.game_time(&*host) else {
        return;
    };
    let due: Vec<LabelKey> = store
        .keys()
        .copied()
        .filter(|key| store.get(key).is_some_and(|label| label.is_due(now)))
        .collect();
    for key in due {
        let Some(label) = store.get(&key) else {
            continue;
        };
        move_once(host, level, key, label);
    }
}

/// One label's activation: the label's flow names which of its two blocks is the source
/// and which the destination, and the move is one transaction, so an item the
/// destination refuses stays in its slot and nothing is lost.
fn move_once(host: &mut dyn ServerHost, level: LevelRef, key: LabelKey, label: &LabelConfig) {
    let (source_at, sink_at) = key.endpoints(label.flow);
    if !level.is_loaded(&*host, source_at.pos) || !level.is_loaded(&*host, sink_at.pos) {
        return;
    }
    let Some(source) =
        level.capability(host, &ITEMS, source_at.pos, Some(direction(source_at.face)))
    else {
        return;
    };
    let Some(destination) =
        level.capability(host, &ITEMS, sink_at.pos, Some(direction(sink_at.face)))
    else {
        return;
    };
    let slots = slots_of(host, &source);
    let counts = counts_of(host, &destination);
    let Some(plan) = label.plan(&slots, &counts) else {
        return;
    };
    let Some(item) = item_key(&*host, &plan.item) else {
        return;
    };
    let _ = host.transaction(|host, tx| {
        move_resources(
            host,
            &source,
            &destination,
            |held| held.item() == item,
            plan.amount,
            tx,
        )
    });
}

/// A source handler's filled slots as the runtime reads them (`native_container_slots`).
///
/// `can_take` is always true: the game decides whether a slot may be emptied when it is asked
/// to, and `move_resources` skips a slot that answers zero, which is the retired helper's rule.
fn slots_of(
    host: &mut dyn ServerHost,
    handler: &Capability<dyn ResourceHandler<ItemResource>>,
) -> Vec<Slot> {
    let mut slots = Vec::new();
    for index in 0..handler.size(host) {
        let Some(resource) = handler.resource(host, index) else {
            continue;
        };
        let count = handler.amount(host, index);
        if count == 0 {
            continue;
        }
        slots.push(Slot {
            item: resource.item().id(&*host),
            count,
            can_take: true,
        });
    }
    slots
}

/// How many of each item a handler holds (`ItemFilter.calculateMaxTransferAmount`'s count).
fn counts_of(
    host: &mut dyn ServerHost,
    handler: &Capability<dyn ResourceHandler<ItemResource>>,
) -> Vec<(String, u32)> {
    let mut counts: Vec<(String, u32)> = Vec::new();
    for index in 0..handler.size(host) {
        let Some(resource) = handler.resource(host, index) else {
            continue;
        };
        let item = resource.item().id(&*host);
        let amount = handler.amount(host, index);
        match counts.iter_mut().find(|(known, _)| *known == item) {
            Some((_, held)) => *held = held.saturating_add(amount),
            None => counts.push((item, amount)),
        }
    }
    counts
}

/// A face as the container helpers take it.
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

/// The face a click's normal names.
fn face_of(normal: [i32; 3]) -> Option<Face> {
    Face::ALL.into_iter().find(|face| face.offset() == normal)
}

// ---------------------------------------------------------------------------
// Attaching and removing (`TransferLabelItem.useOn`, `LabelBlock.remove`)
// ---------------------------------------------------------------------------

/// A label was placed with a label item in hand: it sticks to that block face, moves items
/// the way that item's flow says, and one item is used up. Returns whether the click was
/// the mod's, so the caller cancels it.
fn attach(host: &mut dyn ServerHost, click: &BlockClick<'_>) -> bool {
    let Some(player) = host.player(click.player) else {
        return false;
    };
    let Some(stack) = click.stack else {
        return false;
    };
    let Some(flow) = LabelFlow::of_item(&stack.item().id(&*host)) else {
        return false;
    };
    let Some(face) = face_of(click.hit.face) else {
        return false;
    };
    let key = LabelKey {
        pos: click.pos,
        face,
    };
    let mut store = read_labels(host);
    if store.get(&key).is_some() {
        // Upstream opens the label's own menu here; this slice has no menu, and replacing the
        // label would throw its filter away, so it stays.
        tell(
            host,
            player,
            format!(
                "A label already sits on the {} face of {}, {}, {}: \
                 /transfer_labels info {} {} {} {} shows it.",
                face.name(),
                key.pos[0],
                key.pos[1],
                key.pos[2],
                key.pos[0],
                key.pos[1],
                key.pos[2],
                face.name()
            ),
        );
        return true;
    }
    store.insert(key, LabelConfig::of(flow));
    if !write_labels(host, &store) {
        return false;
    }
    if !consume_one(host, player, click.hand) {
        // Nothing was taken out of the hand, so the label must not stay: upstream's item is
        // only used up when the player had one.
        store.remove(&key);
        write_labels(host, &store);
        return false;
    }
    tell(
        host,
        player,
        format!(
            "Transfer label attached to the {} face of {}, {}, {} (it {} items).",
            face.name(),
            key.pos[0],
            key.pos[1],
            key.pos[2],
            match flow {
                LabelFlow::Extract => "takes items out of that block",
                LabelFlow::Insert => "puts items into that block",
            }
        ),
    );
    true
}

/// One label item out of the hand that clicked (`ItemStack.consume(1, player)`); false when the
/// hand holds nothing to take, so the caller puts the label back.
fn consume_one(host: &mut dyn ServerHost, player: PlayerRef, hand: Hand) -> bool {
    let Some(living) = host.living(player.entity()) else {
        return false;
    };
    let Some(stack) = living.hand_item(&*host, hand) else {
        return false;
    };
    let left = if stack.count() > 1 {
        let mut stack = stack;
        if stack.set_count(stack.count() - 1).is_err() {
            return false;
        }
        Some(stack)
    } else {
        None
    };
    matches!(living.set_hand_item(host, hand, left), Ok(true))
}

/// A label was hit with a label item in hand: it comes off, with the copies its amount and
/// speed upgrades stood for, as the item it was attached with. Returns whether the click was
/// the mod's.
fn remove_clicked(host: &mut dyn ServerHost, pos: [i32; 3], face: [i32; 3]) -> bool {
    let Some(player) = host.players().acting_player() else {
        return false;
    };
    let Some(living) = host.living(player.entity()) else {
        return false;
    };
    let Some(held) = living.hand_item(&*host, Hand::Main) else {
        return false;
    };
    if LabelFlow::of_item(&held.item().id(&*host)).is_none() {
        return false;
    }
    let Some(face) = face_of(face) else {
        return false;
    };
    let key = LabelKey { pos, face };
    let mut store = read_labels(host);
    let Some(label) = store.remove(&key) else {
        return false;
    };
    if !write_labels(host, &store) {
        return false;
    }
    let back = label.refund_items();
    let given = give_labels(host, player, label.refund_item(), back);
    tell(
        host,
        player,
        format!(
            "Transfer label removed from the {} face of {}, {}, {} ({given} of {back} returned).",
            face.name(),
            key.pos[0],
            key.pos[1],
            key.pos[2]
        ),
    );
    true
}

/// Gives up to `count` items of `item` to `player`, one stack at a time, and answers how many
/// went out. The retired helper gave only what fitted and left the rest nowhere; `/give`'s own
/// path throws what does not fit, which is what a removal now does.
fn give_labels(host: &mut dyn ServerHost, player: PlayerRef, item: &str, count: u32) -> u32 {
    let Some(key) = item_key(&*host, item) else {
        return 0;
    };
    let per_stack = host
        .max_stack_size(key)
        .unwrap_or(FALLBACK_MAX_STACK)
        .max(1);
    let mut given = 0;
    for size in stack_chunks(count.min(MAX_GIVEN), per_stack) {
        let Some(stack) = ItemStack::new(key, size) else {
            break;
        };
        if player.give_stack(host, stack).is_err() {
            break;
        }
        given += size;
    }
    given
}

/// The stacks `count` items are handed back in, none larger than `per_stack`.
fn stack_chunks(count: u32, per_stack: u32) -> Vec<u32> {
    let per_stack = per_stack.max(1);
    let mut sizes = Vec::new();
    let mut left = count;
    while left > 0 {
        let size = left.min(per_stack);
        sizes.push(size);
        left -= size;
    }
    sizes
}

// ---------------------------------------------------------------------------
// `/transfer_labels …`
// ---------------------------------------------------------------------------

/// The words a parsed invocation stands for: its literal path, then every resolved argument in
/// parse order, with a block position spread back over the three words the runtime's grammar
/// reads it as.
fn words_of(invocation: &CommandInvocation) -> Vec<String> {
    let mut words = invocation.literals.clone();
    words.extend(argument_words(&invocation.arguments));
    words
}

/// The words the arguments were typed as, in parse order; empty for a kind this port's usages
/// never declare, so an unexpected invocation is refused rather than misread.
fn argument_words(arguments: &[mod_api::CommandArgument]) -> Vec<String> {
    let mut words = Vec::new();
    for argument in arguments {
        match &argument.value {
            CommandArgumentValue::String(text) => words.push(text.clone()),
            CommandArgumentValue::Integer(number) => words.push(number.to_string()),
            CommandArgumentValue::Number(number) => words.push(number.to_string()),
            CommandArgumentValue::BlockPos([x, y, z]) => {
                words.push(x.to_string());
                words.push(y.to_string());
                words.push(z.to_string());
            }
            // `#[non_exhaustive]`: a kind this port does not declare is no word at all.
            _ => return Vec::new(),
        }
    }
    words
}

/// Runs one parsed `/transfer_labels …`.
fn run_command(host: &mut dyn ServerHost, player: PlayerRef, command: Command) {
    match command {
        Command::List { page } => list(host, player, page),
        Command::Info { key } => info(host, player, key),
        Command::Remove { key } => remove_by_command(host, player, key),
        Command::Filter { key, mode } => edit(host, player, key, |label| label.mode = mode),
        Command::Kind { key, kind } => edit(host, player, key, |label| label.kind = kind),
        Command::Entry {
            key,
            slot,
            item,
            count,
        } => edit(host, player, key, |label| {
            label.entries[slot] = Some(FilterEntry { item, count });
        }),
        Command::Clear { key, slot } => edit(host, player, key, |label| label.entries[slot] = None),
        Command::Amount { key, amount } => {
            edit(host, player, key, |label| label.set_amount(amount))
        }
        Command::Speed { key, speed } => edit(host, player, key, |label| label.set_speed(speed)),
    }
}

/// Changes one label and prints what it now is; a label that is not there is said so.
fn edit(
    host: &mut dyn ServerHost,
    player: PlayerRef,
    key: LabelKey,
    change: impl FnOnce(&mut LabelConfig),
) {
    if !may_change_label(host, player, key) {
        tell(host, player, out_of_reach(key));
        return;
    }
    let mut store = read_labels(host);
    let Some(label) = store.get_mut(&key) else {
        tell(host, player, missing(key));
        return;
    };
    change(label);
    let line = describe(&key, label);
    if write_labels(host, &store) {
        tell(host, player, line);
    }
}

/// Takes a label off and hands its items back, without touching the block.
fn remove_by_command(host: &mut dyn ServerHost, player: PlayerRef, key: LabelKey) {
    if !may_change_label(host, player, key) {
        tell(host, player, out_of_reach(key));
        return;
    }
    let mut store = read_labels(host);
    let Some(label) = store.remove(&key) else {
        tell(host, player, missing(key));
        return;
    };
    let back = label.refund_items();
    let given = give_labels(host, player, label.refund_item(), back);
    if write_labels(host, &store) {
        tell(
            host,
            player,
            format!(
                "Transfer label at {} removed ({given} of {back} label items returned).",
                key.text()
            ),
        );
    }
}

/// One page of the level's labels, at most `LIST_PAGE_SIZE` lines, so a level with any
/// number of labels never floods the chat. A page past the end says so instead of printing.
fn list(host: &mut dyn ServerHost, player: PlayerRef, page: usize) {
    let store = read_labels(host);
    if store.is_empty() {
        tell(host, player, "No transfer labels in this level.");
        return;
    }
    let total = store.len();
    let pages = total.div_ceil(LIST_PAGE_SIZE).max(1);
    let (start, end) = list_page(total, page);
    tell(
        host,
        player,
        format!(
            "{total} transfer labels in this level (page {} of {pages}).",
            page.saturating_add(1).min(pages)
        ),
    );
    if start == end {
        return;
    }
    let keys: Vec<LabelKey> = store.keys().copied().collect();
    for key in &keys[start..end] {
        if let Some(label) = store.get(key) {
            tell(host, player, describe(key, label));
        }
    }
}

/// One label's fields, plus the level's game time and what its destination holds, so a harness
/// can compare what moved with the ticks that passed. The destination is the label's own block
/// for an insert label, so it is read from the flow's endpoints, never assumed.
fn info(host: &mut dyn ServerHost, player: PlayerRef, key: LabelKey) {
    let store = read_labels(host);
    let Some(label) = store.get(&key).cloned() else {
        tell(host, player, missing(key));
        return;
    };
    let level = player.level(&*host);
    let ticks = level
        .and_then(|level| level.game_time(&*host))
        .unwrap_or(-1);
    let (_, sink) = key.endpoints(label.flow);
    let dest = level
        .and_then(|level| level.capability(host, &ITEMS, sink.pos, Some(direction(sink.face))))
        .map(|handler| {
            (0..handler.size(host)).fold(0_u32, |total, index| {
                total.saturating_add(handler.amount(host, index))
            })
        })
        .unwrap_or(0);
    tell(
        host,
        player,
        format!("{} ticks={ticks} dest={dest}", describe(&key, &label)),
    );
}

/// The access rule every label mutation shares: the player stands within block reach of the
/// labelled block and is allowed to build there, so a label is not another player's to move,
/// reconfigure or take with its refund from across the level. The click paths already carry
/// these rules; the commands must ask them too.
fn may_change_label(host: &mut dyn ServerHost, player: PlayerRef, key: LabelKey) -> bool {
    if player.is_spectator(&*host) == Some(true) || player.may_build(&*host) == Some(false) {
        return false;
    }
    let Some(eye) = player.entity().eye_position(&*host) else {
        return false;
    };
    let reach = host
        .living(player.entity())
        .and_then(|living| living.attribute_value(&*host, "minecraft:block_interaction_range"))
        .unwrap_or(REACH_FALLBACK);
    within_reach(eye, key.pos, reach)
}

/// The line a refused mutation prints.
fn out_of_reach(key: LabelKey) -> String {
    format!(
        "You are not within reach of the transfer label at {}, {}, {}.",
        key.pos[0], key.pos[1], key.pos[2]
    )
}

/// The line a command repeats for a label that is not there.
fn missing(key: LabelKey) -> String {
    format!(
        "No transfer label on the {} face of {}, {}, {}.",
        key.face.name(),
        key.pos[0],
        key.pos[1],
        key.pos[2]
    )
}

/// One label, as `/transfer_labels list|info` prints it. The fields are what the e2e asserts on,
/// so they stay `key=value`.
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
        "label {} at {},{},{} flow={} mode={} type={} amount={} speed={} interval={} per_op={} entries={}{}",
        key.face.name(),
        key.pos[0],
        key.pos[1],
        key.pos[2],
        label.flow.name(),
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

// ---------------------------------------------------------------------------
// The host's own conversions
// ---------------------------------------------------------------------------

/// Sends one chat line to `player`, ignoring a write the game refuses: a message is feedback,
/// never the operation itself.
fn tell(host: &mut dyn ServerHost, player: PlayerRef, line: impl Into<String>) {
    let _ = player.tell(host, Text::literal(line.into()));
}

/// The process-local key of `id`, or `None` for an id this game does not have.
fn item_key(host: &dyn mod_api::Host, id: &str) -> Option<ItemKey> {
    let id = Identifier::parse(id).ok()?;
    host.item_key(&id)
}

#[cfg(test)]
mod tests {
    use super::{
        Face, decode_store, describe, direction, encode_store, face_of, member, stack_chunks,
    };
    use crate::runtime::{LabelConfig, LabelFlow, LabelKey, LabelStore};
    use mod_api::{CommandArgument, CommandArgumentValue, Compound, Direction, Tag};

    #[test]
    fn a_labels_line_says_which_way_it_moves_items() {
        let key = LabelKey {
            pos: [1, 2, 3],
            face: Face::Up,
        };
        let insert = describe(&key, &LabelConfig::of(LabelFlow::Insert));
        assert!(insert.contains("flow=insert"), "{insert}");
        let extract = describe(&key, &LabelConfig::default());
        assert!(extract.contains("flow=extract"), "{extract}");
    }

    #[test]
    fn a_block_position_spreads_back_over_three_words() {
        let arguments = [
            CommandArgument::new(
                "pos".to_owned(),
                CommandArgumentValue::BlockPos([10, 64, -3]),
            ),
            CommandArgument::new(
                "face".to_owned(),
                CommandArgumentValue::String("up".to_owned()),
            ),
            CommandArgument::new("amount".to_owned(), CommandArgumentValue::Integer(5)),
        ];
        assert_eq!(
            super::argument_words(&arguments),
            vec!["10", "64", "-3", "up", "5"],
            "the runtime's grammar reads a position as three words"
        );
    }

    #[test]
    fn an_entry_without_its_count_loses_no_word() {
        let arguments = [
            CommandArgument::new("pos".to_owned(), CommandArgumentValue::BlockPos([0, 0, 0])),
            CommandArgument::new(
                "face".to_owned(),
                CommandArgumentValue::String("north".to_owned()),
            ),
            CommandArgument::new("slot".to_owned(), CommandArgumentValue::Integer(3)),
            CommandArgument::new(
                "item".to_owned(),
                CommandArgumentValue::String("minecraft:dirt".to_owned()),
            ),
        ];
        assert_eq!(
            super::argument_words(&arguments),
            vec!["0", "0", "0", "north", "3", "minecraft:dirt"],
            "an absent optional argument is absent, not a guess"
        );
    }

    #[test]
    fn an_undeclared_argument_kind_yields_no_words() {
        let arguments = [CommandArgument::new(
            "pos".to_owned(),
            CommandArgumentValue::Boolean(true),
        )];
        assert!(
            super::argument_words(&arguments).is_empty(),
            "a kind this port never declares is refused, not misread"
        );
    }

    #[test]
    fn every_face_names_its_vector_and_its_direction() {
        for face in Face::ALL {
            assert_eq!(face_of(face.offset()), Some(face), "{}", face.name());
        }
        assert_eq!(face_of([2, 0, 0]), None, "not a face normal");
        assert_eq!(direction(Face::Up), Direction::Up);
        assert_eq!(direction(Face::West), Direction::West);
    }

    #[test]
    fn a_removals_items_are_handed_back_in_stack_sized_pieces() {
        assert_eq!(stack_chunks(0, 64), Vec::<u32>::new());
        assert_eq!(stack_chunks(1, 64), vec![1]);
        assert_eq!(stack_chunks(64, 64), vec![64]);
        assert_eq!(
            stack_chunks(102, 64),
            vec![64, 38],
            "1 + 63 amount + 38 speed is more than one stack"
        );
        assert_eq!(
            stack_chunks(3, 1),
            vec![1, 1, 1],
            "a stack of one never overfills"
        );
    }

    /// `Tag::compound` always builds a compound, so the match cannot fail.
    fn compound(entries: impl IntoIterator<Item = (&'static str, Tag)>) -> Compound {
        match Tag::compound(entries) {
            Tag::Compound(compound) => compound,
            _ => unreachable!("a compound tag holds a compound"),
        }
    }

    #[test]
    fn a_compound_member_is_found_by_its_decoded_name() {
        let root = compound([(
            "labels",
            Tag::compound([("1,2,3,up", Tag::String("mode=blacklist".into()))]),
        )]);
        assert!(member(&root, "labels").is_some(), "the member is there");
        assert!(
            member(&root, "elsewhere").is_none(),
            "a name the compound does not hold is absent, not guessed"
        );
    }

    #[test]
    fn a_store_survives_the_attachment_codec() {
        let mut store = LabelStore::default();
        store.insert(
            LabelKey {
                pos: [1, 2, 3],
                face: Face::Up,
            },
            LabelConfig::default(),
        );
        let root = encode_store(&store).expect("the store encodes");
        assert_eq!(decode_store(&root).expect("and decodes"), store);
    }

    #[test]
    fn a_level_without_the_member_loads_an_empty_store() {
        let root = compound([]);
        assert_eq!(
            decode_store(&root).expect("no labels is no error"),
            LabelStore::default()
        );
    }

    #[test]
    fn a_labels_member_of_the_wrong_shape_is_a_decode_error() {
        let root = compound([("labels", Tag::Int(3))]);
        assert!(
            decode_store(&root).is_err(),
            "a number where the labels belong is not an empty store"
        );
    }
}
