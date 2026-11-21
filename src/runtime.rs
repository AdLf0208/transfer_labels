// Transfer Labels' rules, with no game types: which two blocks a label joins,
// when it acts, how much it may move, and what its filter lets through.
//
// Every number here is the jar's: `Config.java` (`baseItemTransferAmount = 1`,
// `amountUpgrades = 63`, `speedUpgrades = 38`, `filterSlots = 20`),
// `LabelInstance.getSpeed`/`getAmount` (the 20-tick base interval) and
// `ItemFilter.calculateMaxTransferAmount` (the regulating cap of 4). See
// `FINDINGS.md` §2.

use std::collections::BTreeMap;

/// The item that pulls items out of the block it is stuck to
/// (`ItemStackExtractTransferLabelItem`).
pub const EXTRACT_LABEL_ITEM: &str = "transfer_labels:itemstack_extract_transfer_label";

/// The item that pushes items into the block it is stuck to
/// (`ItemStackInsertTransferLabelItem`).
pub const INSERT_LABEL_ITEM: &str = "transfer_labels:itemstack_insert_transfer_label";

/// `Config.baseItemTransferAmount`: items moved per activation with no upgrade.
pub const BASE_ITEM_TRANSFER_AMOUNT: u32 = 1;
/// `Config.amountUpgrades`: the most amount-upgrade items a label takes.
pub const AMOUNT_UPGRADES: u32 = 63;
/// `Config.speedUpgrades`: the most speed-upgrade items a label takes.
pub const SPEED_UPGRADES: u32 = 38;
/// `Config.filterSlots`: filter entries a label has.
pub const FILTER_SLOTS: usize = 20;
/// `LabelInstance.work`'s base: the ticks between activations with no upgrade.
pub const BASE_INTERVAL_TICKS: u32 = 20;
/// `ItemFilter.calculateMaxTransferAmount`: the regulating filter's cap per
/// activation, whatever the amount upgrades say.
pub const REGULATING_CAP: u32 = 4;

/// A block face, named as Java's `Direction` names it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Face {
    Down,
    Up,
    North,
    South,
    West,
    East,
}

impl Face {
    pub const ALL: [Face; 6] = [
        Face::Down,
        Face::Up,
        Face::North,
        Face::South,
        Face::West,
        Face::East,
    ];
    /// The face's lower-case name, as `/transfer_labels` spells it.
    pub fn name(self) -> &'static str {
        match self {
            Self::Down => "down",
            Self::Up => "up",
            Self::North => "north",
            Self::South => "south",
            Self::West => "west",
            Self::East => "east",
        }
    }
    /// The face `name` names, if it names one.
    pub fn parse(name: &str) -> Option<Face> {
        Face::ALL.into_iter().find(|face| face.name() == name)
    }
    /// Where the block across this face stands, relative to the labelled one.
    pub fn offset(self) -> [i32; 3] {
        match self {
            Self::Down => [0, -1, 0],
            Self::Up => [0, 1, 0],
            Self::North => [0, 0, -1],
            Self::South => [0, 0, 1],
            Self::West => [-1, 0, 0],
            Self::East => [1, 0, 0],
        }
    }
    /// The face looking back.
    pub fn opposite(self) -> Face {
        match self {
            Self::Down => Self::Up,
            Self::Up => Self::Down,
            Self::North => Self::South,
            Self::South => Self::North,
            Self::West => Self::East,
            Self::East => Self::West,
        }
    }
}

/// `ILabelFilter.Type`: whether the filter's matches are what may move
/// (whitelist) or what may not (blacklist, upstream's default).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FilterMode {
    Whitelist,
    Blacklist,
}

impl FilterMode {
    pub fn name(self) -> &'static str {
        match self {
            Self::Whitelist => "whitelist",
            Self::Blacklist => "blacklist",
        }
    }
    pub fn parse(name: &str) -> Option<FilterMode> {
        match name {
            "whitelist" => Some(Self::Whitelist),
            "blacklist" => Some(Self::Blacklist),
            _ => None,
        }
    }
}

/// `FilterType`: how an entry decides whether it matches an item.
///
/// Upstream's `exact_count` and `tag` are not carried in this slice (there is no
/// item-tag lookup a native mod can ask for, and the exact-count transfer gathers
/// several source slots, which the pin's move helper does not); naming one is
/// refused rather than silently moving nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FilterKind {
    Normal,
    Regulating,
    Mod,
}

impl FilterKind {
    pub fn name(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Regulating => "regulating",
            Self::Mod => "mod",
        }
    }
    pub fn parse(name: &str) -> Option<FilterKind> {
        match name {
            "normal" => Some(Self::Normal),
            "regulating" => Some(Self::Regulating),
            "mod" => Some(Self::Mod),
            _ => None,
        }
    }
}

/// One filter slot: the item it names, and the `regulating` count for it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FilterEntry {
    pub item: String,
    /// `Filter.extras.NumberFilterExtra`: how many of `item` the destination
    /// should hold. Only `regulating` uses it.
    pub count: u32,
}

/// The two label items a player can attach: which way the label moves items.
///
/// Upstream has two item classes for the two directions
/// (`ItemStackExtractTransferLabelItem` / `ItemStackInsertTransferLabelItem`); the
/// port keeps the direction on the label and takes it from the item that was
/// clicked, so one store and one tick serve both.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LabelFlow {
    /// `itemstack_extract_transfer_label`: labelled block → the block across the face.
    Extract,
    /// `itemstack_insert_transfer_label`: the block across the face → labelled block.
    Insert,
}

impl LabelFlow {
    /// The flow's lower-case name, as the saved text and `/transfer_labels` spell it.
    pub fn name(self) -> &'static str {
        match self {
            Self::Extract => "extract",
            Self::Insert => "insert",
        }
    }
    /// The flow `name` names, if it names one.
    pub fn parse(name: &str) -> Option<LabelFlow> {
        match name {
            "extract" => Some(Self::Extract),
            "insert" => Some(Self::Insert),
            _ => None,
        }
    }
    /// The item that attaches this flow.
    pub fn item_id(self) -> &'static str {
        match self {
            Self::Extract => EXTRACT_LABEL_ITEM,
            Self::Insert => INSERT_LABEL_ITEM,
        }
    }
    /// The flow `item` attaches, or `None` for an item that is not a label.
    pub fn of_item(item: &str) -> Option<LabelFlow> {
        [Self::Extract, Self::Insert]
            .into_iter()
            .find(|flow| flow.item_id() == item)
    }
}

/// One end of the pair a label joins: the block and the side it is reached through.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Endpoint {
    pub pos: [i32; 3],
    pub face: Face,
}

/// Everything a label carries besides where it sits: its filter and its two
/// upgrades (`LabelInstance`'s `filter`, `amountFilter`, `speedFilter`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LabelConfig {
    /// Which way this label moves items, from the item it was attached with.
    pub flow: LabelFlow,
    pub mode: FilterMode,
    pub kind: FilterKind,
    /// `amountFilter`'s stack count: copies of the label item put into it.
    pub amount_upgrades: u32,
    /// `speedFilter`'s stack count.
    pub speed_upgrades: u32,
    /// How many of `amount_upgrades` a player actually paid for. Only a paid upgrade is
    /// handed back when the label comes off; an upgrade the harness command set for
    /// free was never an item and must not mint one.
    pub paid_amount: u32,
    /// How many of `speed_upgrades` a player paid for.
    pub paid_speed: u32,
    /// `Config.filterSlots` slots, each naming one item.
    pub entries: Vec<Option<FilterEntry>>,
}

/// The most label lines one `/transfer_labels list` prints: a level can hold any number of
/// labels, and one reply per label would flood the chat of a large world.
pub const LIST_PAGE_SIZE: usize = 16;

/// The `[start, end)` window of a level's labels that page `page` (0-based) prints, clamped to
/// `total`. A page past the end is empty, never out of range.
pub fn list_page(total: usize, page: usize) -> (usize, usize) {
    let start = page.saturating_mul(LIST_PAGE_SIZE).min(total);
    let end = start.saturating_add(LIST_PAGE_SIZE).min(total);
    (start, end)
}

/// `Player.canInteractWithBlock(pos, 0.0)`: whether the eye is within `reach` of the block's
/// centre. A command that edits a label for a player asks this first, so a label across the
/// level is not theirs to change.
pub fn within_reach(eye: [f64; 3], block: [i32; 3], reach: f64) -> bool {
    let centre = [
        f64::from(block[0]) + 0.5,
        f64::from(block[1]) + 0.5,
        f64::from(block[2]) + 0.5,
    ];
    let distance =
        (eye[0] - centre[0]).powi(2) + (eye[1] - centre[1]).powi(2) + (eye[2] - centre[2]).powi(2);
    distance <= reach * reach
}

impl Default for LabelConfig {
    /// `new ItemFilter(...)`: a blacklist with `normal` matching and no entries,
    /// so a label just placed moves everything.
    fn default() -> Self {
        Self {
            flow: LabelFlow::Extract,
            mode: FilterMode::Blacklist,
            kind: FilterKind::Normal,
            amount_upgrades: 0,
            speed_upgrades: 0,
            paid_amount: 0,
            paid_speed: 0,
            entries: vec![None; FILTER_SLOTS],
        }
    }
}

impl LabelConfig {
    /// A label of `flow` just attached: upstream's `new ItemFilter(...)` for that
    /// label item — a blacklist with `normal` matching and no entries.
    pub fn of(flow: LabelFlow) -> Self {
        Self {
            flow,
            ..Self::default()
        }
    }

    /// The label item this label was attached with, and the one a removal hands back.
    pub fn refund_item(&self) -> &'static str {
        self.flow.item_id()
    }

    /// Sets the amount upgrades a label runs with (the command's control). The paid count
    /// follows down when the count drops below it, so a label never refunds more than it
    /// still holds; setting more than was paid leaves the paid count alone, because the
    /// extra copies were never items.
    pub fn set_amount(&mut self, amount: u32) {
        self.amount_upgrades = amount.min(AMOUNT_UPGRADES);
        self.paid_amount = self.paid_amount.min(self.amount_upgrades);
    }

    /// Sets the speed upgrades a label runs with, under the same rule as [`set_amount`].
    ///
    /// [`set_amount`]: Self::set_amount
    pub fn set_speed(&mut self, speed: u32) {
        self.speed_upgrades = speed.min(SPEED_UPGRADES);
        self.paid_speed = self.paid_speed.min(self.speed_upgrades);
    }

    /// An amount upgrade the player put in: the count and the paid count both grow.
    pub fn pay_amount(&mut self, count: u32) {
        self.amount_upgrades = self
            .amount_upgrades
            .saturating_add(count)
            .min(AMOUNT_UPGRADES);
        self.paid_amount = self
            .paid_amount
            .saturating_add(count)
            .min(self.amount_upgrades);
    }

    /// A speed upgrade the player put in.
    pub fn pay_speed(&mut self, count: u32) {
        self.speed_upgrades = self
            .speed_upgrades
            .saturating_add(count)
            .min(SPEED_UPGRADES);
        self.paid_speed = self
            .paid_speed
            .saturating_add(count)
            .min(self.speed_upgrades);
    }

    /// The label items taking this label off hands back: the label itself plus only the
    /// upgrades it still runs with and that a player paid for.
    pub fn refund_items(&self) -> u32 {
        1 + self.amount_upgrades.min(self.paid_amount) + self.speed_upgrades.min(self.paid_speed)
    }
}

impl LabelConfig {
    /// `LabelInstance.work`'s divisor: `20 - min(speed, 38) / 2` ticks between
    /// activations, never below one.
    pub fn interval_ticks(&self) -> u32 {
        let speed = self.speed_upgrades.min(SPEED_UPGRADES) / 2;
        BASE_INTERVAL_TICKS.saturating_sub(speed)
    }
    /// `Config.baseItemTransferAmount + getAmount()`: one item plus one per
    /// amount upgrade, capped at 64.
    pub fn transfer_amount(&self) -> u32 {
        BASE_ITEM_TRANSFER_AMOUNT + self.amount_upgrades.min(AMOUNT_UPGRADES)
    }
    /// `ItemFilter.passesFilter` without the tag and exact-count kinds: does
    /// this item pass the filter at all?
    pub fn passes_filter(&self, item: &str) -> bool {
        let matches = self.entries.iter().flatten().any(|entry| match self.kind {
            FilterKind::Mod => namespace(&entry.item) == namespace(item),
            FilterKind::Normal | FilterKind::Regulating => entry.item == item,
        });
        // `type.getFilter().test(matches)`: a whitelist wants a match, a
        // blacklist wants none.
        match self.mode {
            FilterMode::Whitelist => matches,
            FilterMode::Blacklist => !matches,
        }
    }
    /// The first entry that names `item` (`normal` and `regulating` compare the
    /// item id; `mod` compares the namespace).
    pub fn entry_for(&self, item: &str) -> Option<&FilterEntry> {
        self.entries.iter().flatten().find(|entry| match self.kind {
            FilterKind::Mod => namespace(&entry.item) == namespace(item),
            FilterKind::Normal | FilterKind::Regulating => entry.item == item,
        })
    }
    /// `ItemFilter.calculateMaxTransferAmount`: how many may move this time.
    ///
    /// A `regulating` entry caps it at what the destination still lacks, and at
    /// four whatever the upgrades say; every other case is the amount rule.
    pub fn budget(&self, item: &str, target: &[(String, u32)]) -> u32 {
        if self.kind != FilterKind::Regulating {
            return self.transfer_amount();
        }
        let Some(entry) = self.entry_for(item) else {
            return self.transfer_amount();
        };
        let held = target
            .iter()
            .find(|(other, _)| other == item)
            .map_or(0, |(_, count)| *count);
        entry.count.saturating_sub(held).min(REGULATING_CAP)
    }
    /// `ItemFilter.transferItems`: the first source slot that may be taken from
    /// and passes the filter, and how much of it moves now.
    pub fn plan(&self, source: &[Slot], target: &[(String, u32)]) -> Option<Plan> {
        let slot = source
            .iter()
            .find(|slot| slot.count > 0 && slot.can_take && self.passes_filter(&slot.item))?;
        let amount = self.budget(&slot.item, target).min(slot.count);
        (amount > 0).then(|| Plan {
            item: slot.item.clone(),
            amount,
        })
    }
    /// `LabelInstance.work`: `gameTime % (20 - speed) == 0`.
    pub fn is_due(&self, game_time: i64) -> bool {
        let interval = i64::from(self.interval_ticks().max(1));
        game_time.rem_euclid(interval) == 0
    }
    /// The label's saved text: one `key=value` line each, the entries by slot.
    pub fn encode(&self) -> String {
        let mut text = format!(
            "mode={}\nkind={}\nflow={}\namount={}\nspeed={}\npaid_amount={}\npaid_speed={}",
            self.mode.name(),
            self.kind.name(),
            self.flow.name(),
            self.amount_upgrades,
            self.speed_upgrades,
            self.paid_amount,
            self.paid_speed
        );
        for (slot, entry) in self.entries.iter().enumerate() {
            if let Some(entry) = entry {
                text.push_str(&format!("\nslot{slot}={},{}", entry.item, entry.count));
            }
        }
        text
    }
    /// The label `text` was written by [`LabelConfig::encode`]; `None` for a
    /// text this version cannot read whole.
    pub fn decode(text: &str) -> Option<LabelConfig> {
        let mut label = LabelConfig::default();
        let mut paid_seen = false;
        if text.is_empty() {
            return None;
        }
        for line in text.lines() {
            let (key, value) = line.split_once('=')?;
            match key {
                "mode" => label.mode = FilterMode::parse(value)?,
                "kind" => label.kind = FilterKind::parse(value)?,
                // A text written before the insert label existed has no `flow`: the
                // item that was attached then was the extract label, which stays the
                // default, so an old save keeps working unchanged.
                "flow" => label.flow = LabelFlow::parse(value)?,
                "amount" => label.amount_upgrades = value.parse().ok()?,
                "speed" => label.speed_upgrades = value.parse().ok()?,
                "paid_amount" => {
                    label.paid_amount = value.parse().ok()?;
                    paid_seen = true;
                }
                "paid_speed" => {
                    label.paid_speed = value.parse().ok()?;
                    paid_seen = true;
                }
                _ => {
                    let slot: usize = key.strip_prefix("slot")?.parse().ok()?;
                    let (item, count) = value.rsplit_once(',')?;
                    let count: u32 = count.parse().ok()?;
                    if item.is_empty() || slot >= FILTER_SLOTS {
                        return None;
                    }
                    label.entries[slot] = Some(FilterEntry {
                        item: item.to_owned(),
                        count,
                    });
                }
            }
        }
        if !paid_seen {
            // A 0.2.0 text named no paid counts: its upgrades were items the player put in
            // before the command could set them, so they stay refundable.
            label.paid_amount = label.amount_upgrades;
            label.paid_speed = label.speed_upgrades;
        }
        Some(label)
    }
}

/// The namespace of an item id (`transfer_labels:label` → `transfer_labels`).
fn namespace(item: &str) -> &str {
    item.split_once(':').map_or(item, |(id, _)| id)
}

/// One source slot as the pin's container helper reports it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Slot {
    pub item: String,
    pub count: u32,
    pub can_take: bool,
}

/// What one activation should move.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Plan {
    pub item: String,
    pub amount: u32,
}

/// A label's place: the block it is stuck to and the face it covers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LabelKey {
    pub pos: [i32; 3],
    pub face: Face,
}

impl LabelKey {
    /// The key's saved form, `x,y,z,face`.
    pub fn text(&self) -> String {
        format!(
            "{},{},{},{}",
            self.pos[0],
            self.pos[1],
            self.pos[2],
            self.face.name()
        )
    }
    /// The key `text` names, as [`LabelKey::text`] wrote it.
    pub fn parse(text: &str) -> Option<LabelKey> {
        let mut words = text.split(',');
        let x = words.next()?.parse().ok()?;
        let y = words.next()?.parse().ok()?;
        let z = words.next()?.parse().ok()?;
        let face = Face::parse(words.next()?)?;
        words.next().is_none().then_some(LabelKey {
            pos: [x, y, z],
            face,
        })
    }
    /// The block across the labelled face: the one a label transfers to or from.
    pub fn target(&self) -> [i32; 3] {
        let [x, y, z] = self.pos;
        let [dx, dy, dz] = self.face.offset();
        [x + dx, y + dy, z + dz]
    }
    /// The side the labelled block is reached through.
    pub fn from_face(&self) -> Face {
        self.face
    }
    /// The side the neighbouring block is reached through.
    pub fn to_face(&self) -> Face {
        self.face.opposite()
    }
    /// `ItemFilter.work`: the two blocks a label joins, as (source, destination).
    ///
    /// An `Extract` label takes from the block it is stuck to and gives to the block
    /// across the face; an `Insert` label is the same pair with the roles swapped, so
    /// its destination is the labelled block itself.
    pub fn endpoints(self, flow: LabelFlow) -> (Endpoint, Endpoint) {
        let labelled = Endpoint {
            pos: self.pos,
            face: self.from_face(),
        };
        let neighbour = Endpoint {
            pos: self.target(),
            face: self.to_face(),
        };
        match flow {
            LabelFlow::Extract => (labelled, neighbour),
            LabelFlow::Insert => (neighbour, labelled),
        }
    }
}

/// Every label in one level (`LabelStorage`, as a level attachment).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LabelStore {
    labels: BTreeMap<LabelKey, LabelConfig>,
}

impl LabelStore {
    pub fn len(&self) -> usize {
        self.labels.len()
    }
    pub fn is_empty(&self) -> bool {
        self.labels.is_empty()
    }
    pub fn insert(&mut self, key: LabelKey, label: LabelConfig) {
        self.labels.insert(key, label);
    }
    pub fn get(&self, key: &LabelKey) -> Option<&LabelConfig> {
        self.labels.get(key)
    }
    pub fn get_mut(&mut self, key: &LabelKey) -> Option<&mut LabelConfig> {
        self.labels.get_mut(key)
    }
    pub fn remove(&mut self, key: &LabelKey) -> Option<LabelConfig> {
        self.labels.remove(key)
    }
    pub fn keys(&self) -> impl Iterator<Item = &LabelKey> {
        self.labels.keys()
    }
    /// The store as the attachment's compound holds it: one record per label,
    /// in key order.
    pub fn encode(&self) -> Vec<(String, String)> {
        self.labels
            .iter()
            .map(|(key, label)| (key.text(), label.encode()))
            .collect()
    }
    /// The store `records` describe. A record this version cannot read is
    /// dropped, as upstream's loader would keep going.
    pub fn decode(records: &[(String, String)]) -> LabelStore {
        let mut store = LabelStore::default();
        for (key, text) in records {
            let Some(key) = LabelKey::parse(key) else {
                continue;
            };
            let Some(label) = LabelConfig::decode(text) else {
                continue;
            };
            store.insert(key, label);
        }
        store
    }
}

/// `/transfer_labels …`: what a player asked for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    /// `/transfer_labels list [page]`: one page of the level's labels (1-based; the first
    /// page when no number is given). A level can hold any number of labels, so a single
    /// invocation never prints them all.
    List {
        page: usize,
    },
    Info {
        key: LabelKey,
    },
    Filter {
        key: LabelKey,
        mode: FilterMode,
    },
    Kind {
        key: LabelKey,
        kind: FilterKind,
    },
    Entry {
        key: LabelKey,
        slot: usize,
        item: String,
        count: u32,
    },
    Clear {
        key: LabelKey,
        slot: usize,
    },
    Amount {
        key: LabelKey,
        amount: u32,
    },
    Speed {
        key: LabelKey,
        speed: u32,
    },
    Remove {
        key: LabelKey,
    },
}

impl Command {
    /// The command `words` spell, or the reason it spells none.
    pub fn parse(words: &[&str]) -> Result<Command, String> {
        let (name, rest) = words.split_first().ok_or_else(usage)?;
        if *name != "transfer_labels" {
            return Err(usage());
        }
        let (sub, rest) = rest.split_first().ok_or_else(usage)?;
        match *sub {
            "list" => {
                let page = match rest.first() {
                    Some(number) => {
                        number
                            .parse::<usize>()
                            .ok()
                            .filter(|page| *page > 0)
                            .ok_or_else(usage)?
                            - 1
                    }
                    None => 0,
                };
                if rest.len() > 1 {
                    return Err(usage());
                }
                Ok(Command::List { page })
            }
            "info" => Ok(Command::Info {
                key: key_from(rest)?,
            }),
            "remove" => Ok(Command::Remove {
                key: key_from(rest)?,
            }),
            "filter" => {
                let (key, rest) = key_and_rest(rest)?;
                let mode = rest.first().and_then(|name| FilterMode::parse(name));
                Ok(Command::Filter {
                    key,
                    mode: mode.ok_or_else(|| usage_or("whitelist|blacklist"))?,
                })
            }
            "type" => {
                let (key, rest) = key_and_rest(rest)?;
                let kind = rest.first().and_then(|name| FilterKind::parse(name));
                Ok(Command::Kind {
                    key,
                    kind: kind.ok_or_else(|| {
                        usage_or("normal|regulating|mod (exact_count and tag are not carried)")
                    })?,
                })
            }
            "entry" => {
                let (key, rest) = key_and_rest(rest)?;
                let (slot, rest) = rest.split_first().ok_or_else(usage)?;
                let slot: usize = slot.parse().map_err(|_| usage())?;
                if slot >= FILTER_SLOTS {
                    return Err(usage_or(&format!(
                        "a filter slot is 0..{}",
                        FILTER_SLOTS - 1
                    )));
                }
                let (item, rest) = rest.split_first().ok_or_else(usage)?;
                if !item.contains(':') {
                    return Err(usage_or("an item id is namespace:name"));
                }
                let count = match rest.first() {
                    Some(count) => count.parse().map_err(|_| usage())?,
                    None => 1,
                };
                Ok(Command::Entry {
                    key,
                    slot,
                    item: (*item).to_owned(),
                    count,
                })
            }
            "clear" => {
                let (key, rest) = key_and_rest(rest)?;
                let (slot, _) = rest.split_first().ok_or_else(usage)?;
                let slot: usize = slot.parse().map_err(|_| usage())?;
                if slot >= FILTER_SLOTS {
                    return Err(usage_or(&format!(
                        "a filter slot is 0..{}",
                        FILTER_SLOTS - 1
                    )));
                }
                Ok(Command::Clear { key, slot })
            }
            "amount" => {
                let (key, rest) = key_and_rest(rest)?;
                let (amount, _) = rest.split_first().ok_or_else(usage)?;
                Ok(Command::Amount {
                    key,
                    amount: amount.parse().map_err(|_| usage())?,
                })
            }
            "speed" => {
                let (key, rest) = key_and_rest(rest)?;
                let (speed, _) = rest.split_first().ok_or_else(usage)?;
                Ok(Command::Speed {
                    key,
                    speed: speed.parse().map_err(|_| usage())?,
                })
            }
            _ => Err(usage()),
        }
    }
}

/// `/transfer_labels …`, the line every refusal repeats.
fn usage() -> String {
    "usage: /transfer_labels list|info|remove <x> <y> <z> <face>; \
     filter|type|amount|speed <x> <y> <z> <face> <value>; \
     entry <x> <y> <z> <face> <slot> <item> [count]; clear <x> <y> <z> <face> <slot>"
        .to_owned()
}

fn usage_or(what: &str) -> String {
    format!("{}; {what}", usage())
}

/// The key four words spell.
fn key_from(words: &[&str]) -> Result<LabelKey, String> {
    let mut rest = words.iter();
    let x = rest.next().and_then(|word| word.parse().ok());
    let y = rest.next().and_then(|word| word.parse().ok());
    let z = rest.next().and_then(|word| word.parse().ok());
    let face = rest.next().and_then(|word| Face::parse(word));
    match (x, y, z, face) {
        (Some(x), Some(y), Some(z), Some(face)) => Ok(LabelKey {
            pos: [x, y, z],
            face,
        }),
        _ => Err(usage()),
    }
}

/// The key four words spell, and what follows it.
fn key_and_rest<'a>(words: &'a [&'a str]) -> Result<(LabelKey, &'a [&'a str]), String> {
    if words.len() < 5 {
        return Err(usage());
    }
    Ok((key_from(words)?, &words[4..]))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> LabelConfig {
        LabelConfig::default()
    }

    fn entry(item: &str, count: u32) -> Option<FilterEntry> {
        Some(FilterEntry {
            item: item.to_owned(),
            count,
        })
    }

    fn slot(item: &str, count: u32) -> Slot {
        Slot {
            item: item.to_owned(),
            count,
            can_take: true,
        }
    }

    // --- rate and quantity (LabelInstance.getSpeed / getAmount) ---

    #[test]
    fn a_fresh_label_moves_one_item_every_twenty_ticks() {
        let label = config();
        assert_eq!(
            label.interval_ticks(),
            20,
            "no speed upgrade: the base 20 ticks"
        );
        assert_eq!(label.transfer_amount(), 1, "no amount upgrade: one item");
    }

    #[test]
    fn speed_upgrades_halve_the_interval_integer_divided() {
        let mut label = config();
        label.speed_upgrades = 1;
        assert_eq!(
            label.interval_ticks(),
            20,
            "min(1,38)/2 = 0, so still 20 ticks"
        );
        label.speed_upgrades = 2;
        assert_eq!(label.interval_ticks(), 19, "min(2,38)/2 = 1");
        label.speed_upgrades = 38;
        assert_eq!(
            label.interval_ticks(),
            1,
            "min(38,38)/2 = 19, so every tick"
        );
        label.speed_upgrades = 63;
        assert_eq!(label.interval_ticks(), 1, "clamped at 38 upgrades");
    }

    #[test]
    fn amount_upgrades_add_one_item_each_up_to_sixty_four() {
        let mut label = config();
        label.amount_upgrades = 63;
        assert_eq!(label.transfer_amount(), 64, "1 + 63");
        label.amount_upgrades = 100;
        assert_eq!(label.transfer_amount(), 64, "clamped at 63 upgrades");
    }

    #[test]
    fn a_label_is_due_on_the_multiples_of_its_interval() {
        let label = config();
        assert!(label.is_due(0));
        assert!(!label.is_due(1));
        assert!(label.is_due(20));
        assert!(label.is_due(2000));
        assert!(!label.is_due(2001));
        let mut fast = config();
        fast.speed_upgrades = 38;
        assert!(fast.is_due(1), "an interval of one tick is due every tick");
    }

    // --- the filter (ItemFilter.passesFilter) ---

    #[test]
    fn a_fresh_label_is_a_blacklist_that_passes_everything() {
        let label = config();
        assert_eq!(label.mode, FilterMode::Blacklist);
        assert_eq!(label.kind, FilterKind::Normal);
        assert!(label.passes_filter("minecraft:cobblestone"));
        assert!(label.passes_filter("minecraft:dirt"));
    }

    #[test]
    fn a_whitelist_passes_only_its_entries() {
        let mut label = config();
        label.mode = FilterMode::Whitelist;
        label.entries[0] = entry("minecraft:cobblestone", 1);
        assert!(label.passes_filter("minecraft:cobblestone"));
        assert!(!label.passes_filter("minecraft:dirt"));
    }

    #[test]
    fn an_empty_whitelist_passes_nothing() {
        let mut label = config();
        label.mode = FilterMode::Whitelist;
        assert!(
            !label.passes_filter("minecraft:cobblestone"),
            "no entry matches, and a whitelist wants a match"
        );
    }

    #[test]
    fn a_blacklist_rejects_its_entries() {
        let mut label = config();
        label.entries[2] = entry("minecraft:cobblestone", 1);
        assert!(!label.passes_filter("minecraft:cobblestone"));
        assert!(label.passes_filter("minecraft:dirt"));
    }

    #[test]
    fn a_mod_filter_matches_the_namespace() {
        let mut label = config();
        label.kind = FilterKind::Mod;
        label.mode = FilterMode::Whitelist;
        label.entries[0] = entry("minecraft:stone", 1);
        assert!(
            label.passes_filter("minecraft:cobblestone"),
            "same namespace"
        );
        assert!(!label.passes_filter("transfer_labels:itemstack_extract_transfer_label"));
    }

    #[test]
    fn a_regulating_filter_still_matches_by_item() {
        let mut label = config();
        label.kind = FilterKind::Regulating;
        label.mode = FilterMode::Whitelist;
        label.entries[0] = entry("minecraft:cobblestone", 8);
        assert!(
            label.passes_filter("minecraft:cobblestone"),
            "the entry names it"
        );
        assert!(
            !label.passes_filter("minecraft:dirt"),
            "a whitelist refuses the rest"
        );
    }

    // --- the regulating budget (ItemFilter.calculateMaxTransferAmount) ---

    #[test]
    fn regulating_fills_the_destination_to_the_entry_and_no_further() {
        let mut label = config();
        label.kind = FilterKind::Regulating;
        label.entries[0] = entry("minecraft:cobblestone", 10);
        let target = [("minecraft:cobblestone".to_owned(), 8)];
        assert_eq!(
            label.budget("minecraft:cobblestone", &target),
            2,
            "two short of ten"
        );
        let full = [("minecraft:cobblestone".to_owned(), 10)];
        assert_eq!(
            label.budget("minecraft:cobblestone", &full),
            0,
            "full: nothing to do"
        );
        let overflowing = [("minecraft:cobblestone".to_owned(), 40)];
        assert_eq!(label.budget("minecraft:cobblestone", &overflowing), 0);
    }

    #[test]
    fn regulating_never_moves_more_than_four_at_once() {
        let mut label = config();
        label.kind = FilterKind::Regulating;
        label.entries[0] = entry("minecraft:cobblestone", 64);
        assert_eq!(
            label.budget("minecraft:cobblestone", &[]),
            4,
            "the regulating cap, not the amount upgrades"
        );
        label.amount_upgrades = 63;
        assert_eq!(
            label.budget("minecraft:cobblestone", &[]),
            4,
            "the cap wins over 64"
        );
    }

    #[test]
    fn regulating_leaves_an_unfiltered_item_to_the_amount_rule() {
        let mut label = config();
        label.kind = FilterKind::Regulating;
        label.entries[0] = entry("minecraft:cobblestone", 10);
        label.amount_upgrades = 3;
        assert_eq!(
            label.budget("minecraft:dirt", &[]),
            4,
            "no entry for dirt: 1 + 3, as upstream leaves maxAmount at the default"
        );
    }

    // --- which item moves (ItemFilter.transferItems) ---

    #[test]
    fn the_first_source_slot_that_passes_the_filter_is_the_one_moved() {
        let label = config();
        let source = [slot("minecraft:dirt", 5), slot("minecraft:cobblestone", 3)];
        let plan = label.plan(&source, &[]).expect("something to move");
        assert_eq!(plan.item, "minecraft:dirt", "slot 0 is first");
        assert_eq!(plan.amount, 1);
    }

    #[test]
    fn a_filtered_out_slot_is_skipped_for_a_later_one() {
        let mut label = config();
        label.mode = FilterMode::Whitelist;
        label.entries[0] = entry("minecraft:cobblestone", 1);
        let source = [slot("minecraft:dirt", 5), slot("minecraft:cobblestone", 3)];
        let plan = label.plan(&source, &[]).expect("cobblestone passes");
        assert_eq!(plan.item, "minecraft:cobblestone");
    }

    #[test]
    fn nothing_plans_when_no_slot_passes_or_the_source_is_empty() {
        let mut label = config();
        label.mode = FilterMode::Whitelist;
        label.entries[0] = entry("minecraft:cobblestone", 1);
        assert!(label.plan(&[slot("minecraft:dirt", 5)], &[]).is_none());
        assert!(label.plan(&[], &[]).is_none());
    }

    #[test]
    fn a_slot_the_label_cannot_take_from_is_skipped() {
        let label = config();
        let source = [
            Slot {
                item: "minecraft:dirt".to_owned(),
                count: 5,
                can_take: false,
            },
            slot("minecraft:cobblestone", 2),
        ];
        let plan = label
            .plan(&source, &[])
            .expect("the second slot can be taken from");
        assert_eq!(plan.item, "minecraft:cobblestone");
    }

    #[test]
    fn a_regulating_plan_moves_nothing_once_the_destination_is_full() {
        let mut label = config();
        label.kind = FilterKind::Regulating;
        label.mode = FilterMode::Whitelist;
        label.entries[0] = entry("minecraft:cobblestone", 10);
        let source = [slot("minecraft:cobblestone", 64)];
        let target = [("minecraft:cobblestone".to_owned(), 10)];
        assert!(
            label.plan(&source, &target).is_none(),
            "nothing left to move"
        );
        let short = [("minecraft:cobblestone".to_owned(), 7)];
        let plan = label.plan(&source, &short).expect("three short of ten");
        assert_eq!(plan.amount, 3);
    }

    // --- the block a label joins (ItemFilter.work) ---

    #[test]
    fn a_face_names_its_neighbour_and_its_opposite() {
        assert_eq!(Face::Up.offset(), [0, 1, 0]);
        assert_eq!(Face::Up.opposite(), Face::Down);
        assert_eq!(Face::North.offset(), [0, 0, -1]);
        assert_eq!(Face::East.opposite(), Face::West);
        for face in Face::ALL {
            assert_eq!(face.opposite().opposite(), face);
            let [x, y, z] = face.offset();
            assert_eq!(face.opposite().offset(), [-x, -y, -z]);
        }
    }

    #[test]
    fn a_label_between_two_blocks_is_the_labelled_one_and_the_neighbour() {
        let key = LabelKey {
            pos: [10, 64, -3],
            face: Face::Up,
        };
        assert_eq!(key.target(), [10, 65, -3]);
        assert_eq!(key.from_face(), Face::Up);
        assert_eq!(key.to_face(), Face::Down);
    }

    // --- what is saved (the level attachment's codec) ---

    #[test]
    fn a_label_config_survives_its_text_form() {
        let mut label = config();
        label.mode = FilterMode::Whitelist;
        label.kind = FilterKind::Regulating;
        label.amount_upgrades = 7;
        label.speed_upgrades = 4;
        label.entries[0] = entry("minecraft:cobblestone", 12);
        label.entries[19] = entry("minecraft:dirt", 3);
        let text = label.encode();
        let back = LabelConfig::decode(&text).expect("its own text decodes");
        assert_eq!(back, label);
    }

    #[test]
    fn garbage_is_refused_rather_than_guessed() {
        assert!(LabelConfig::decode("mode=sideways").is_none());
        assert!(LabelConfig::decode("kind=nonsense").is_none());
        assert!(LabelConfig::decode("amount=seven").is_none());
        assert!(LabelConfig::decode("slot99=minecraft:dirt,1").is_none());
        assert!(
            LabelConfig::decode("").is_none(),
            "an empty text is no label"
        );
    }

    #[test]
    fn the_store_keeps_labels_by_position_and_face() {
        let mut store = LabelStore::default();
        let top = LabelKey {
            pos: [1, 2, 3],
            face: Face::Up,
        };
        let side = LabelKey {
            pos: [1, 2, 3],
            face: Face::North,
        };
        assert!(store.is_empty());
        store.insert(top, config());
        store.insert(side, config());
        assert_eq!(store.len(), 2, "two faces of one block are two labels");
        assert!(store.get(&top).is_some());
        assert_eq!(store.remove(&top).map(|_| ()), Some(()));
        assert!(store.get(&top).is_none());
        assert_eq!(store.len(), 1);
        assert!(
            store.remove(&top).is_none(),
            "removing twice is not an error"
        );
    }

    #[test]
    fn a_store_survives_the_saved_records() {
        let mut store = LabelStore::default();
        let mut label = config();
        label.mode = FilterMode::Whitelist;
        label.entries[3] = entry("minecraft:cobblestone", 5);
        store.insert(
            LabelKey {
                pos: [-4, 70, 12],
                face: Face::West,
            },
            label,
        );
        store.insert(
            LabelKey {
                pos: [0, 0, 0],
                face: Face::Down,
            },
            config(),
        );
        let records = store.encode();
        assert_eq!(records.len(), 2);
        let back = LabelStore::decode(&records);
        assert_eq!(back, store);
        assert_eq!(
            back.keys().collect::<Vec<_>>(),
            vec![
                &LabelKey {
                    pos: [-4, 70, 12],
                    face: Face::West
                },
                &LabelKey {
                    pos: [0, 0, 0],
                    face: Face::Down
                },
            ],
            "the saved order is the key order, so a rebuild writes the same bytes"
        );
    }

    #[test]
    fn a_broken_saved_record_is_dropped_not_fatal() {
        let records = vec![
            (
                "1,2,3,up".to_owned(),
                "mode=blacklist\nkind=normal".to_owned(),
            ),
            (
                "not-a-key".to_owned(),
                "mode=blacklist\nkind=normal".to_owned(),
            ),
            ("4,5,6,up".to_owned(), "mode=sideways".to_owned()),
        ];
        let store = LabelStore::decode(&records);
        assert_eq!(store.len(), 1, "only the sound record survives");
    }

    // --- the command grammar ---

    fn words(line: &str) -> Vec<&str> {
        line.split_whitespace().collect()
    }

    #[test]
    fn the_command_addresses_a_label_by_position_and_face() {
        let key = LabelKey {
            pos: [10, 64, -3],
            face: Face::Up,
        };
        assert_eq!(
            Command::parse(&words("transfer_labels amount 10 64 -3 up 5")),
            Ok(Command::Amount { key, amount: 5 })
        );
        assert_eq!(
            Command::parse(&words("transfer_labels filter 10 64 -3 up whitelist")),
            Ok(Command::Filter {
                key,
                mode: FilterMode::Whitelist
            })
        );
        assert_eq!(
            Command::parse(&words("transfer_labels type 10 64 -3 up regulating")),
            Ok(Command::Kind {
                key,
                kind: FilterKind::Regulating
            })
        );
        assert_eq!(
            Command::parse(&words(
                "transfer_labels entry 10 64 -3 up 0 minecraft:cobblestone 12"
            )),
            Ok(Command::Entry {
                key,
                slot: 0,
                item: "minecraft:cobblestone".to_owned(),
                count: 12
            })
        );
        assert_eq!(
            Command::parse(&words("transfer_labels clear 10 64 -3 up 0")),
            Ok(Command::Clear { key, slot: 0 })
        );
        assert_eq!(
            Command::parse(&words("transfer_labels speed 10 64 -3 up 38")),
            Ok(Command::Speed { key, speed: 38 })
        );
        assert_eq!(
            Command::parse(&words("transfer_labels remove 10 64 -3 up")),
            Ok(Command::Remove { key })
        );
        assert_eq!(
            Command::parse(&words("transfer_labels info 10 64 -3 up")),
            Ok(Command::Info { key })
        );
        assert_eq!(
            Command::parse(&words("transfer_labels list")),
            Ok(Command::List { page: 0 })
        );
        assert_eq!(
            Command::parse(&words("transfer_labels list 3")),
            Ok(Command::List { page: 2 }),
            "the command's pages are 1-based"
        );
        assert!(Command::parse(&words("transfer_labels list 0")).is_err());
        assert!(Command::parse(&words("transfer_labels list many")).is_err());
    }

    #[test]
    fn a_malformed_command_is_refused_with_a_reason() {
        assert!(Command::parse(&words("transfer_labels")).is_err());
        assert!(Command::parse(&words("transfer_labels info 10 64 up")).is_err());
        assert!(Command::parse(&words("transfer_labels info 10 64 -3 sideways")).is_err());
        assert!(Command::parse(&words("transfer_labels amount 10 64 -3 up")).is_err());
        assert!(
            Command::parse(&words(
                "transfer_labels entry 10 64 -3 up 20 minecraft:dirt 1"
            ))
            .is_err()
        );
        assert!(Command::parse(&words("transfer_labels entry 10 64 -3 up 0 not an item")).is_err());
        assert!(Command::parse(&words("transfer_labels type 10 64 -3 up tag")).is_err());
        assert!(Command::parse(&words("somethingelse list")).is_err());
    }

    // --- what coming off a label hands back (review 2026-11-18) ---

    #[test]
    fn only_upgrades_that_were_paid_for_come_back_with_the_label() {
        // The harness command sets amount/speed without consuming anything, so those
        // upgrades were never paid for: taking the label off must return the one label
        // item and nothing else.
        let mut label = config();
        label.set_amount(63);
        label.set_speed(38);
        assert_eq!(label.refund_items(), 1);
    }

    #[test]
    fn a_paid_upgrade_is_refunded_with_the_label() {
        let mut label = config();
        label.pay_amount(3);
        label.pay_speed(2);
        assert_eq!(label.refund_items(), 6, "the label plus five upgrades");
    }

    #[test]
    fn a_command_that_lowers_the_count_below_what_was_paid_refunds_only_what_stays() {
        let mut label = config();
        label.pay_amount(5);
        label.set_amount(2);
        assert_eq!(
            label.paid_amount, 2,
            "the paid count follows the label down"
        );
        assert_eq!(label.refund_items(), 3);
    }

    #[test]
    fn an_old_text_without_paid_counters_keeps_its_upgrades_refundable() {
        // 0.2.0 wrote only `amount=`/`speed=`; those upgrades were paid for by the
        // player before the command could set them, so an old save stays refundable.
        let label = LabelConfig::decode("mode=blacklist\nkind=normal\namount=4\nspeed=1\n")
            .expect("a 0.2.0 label text");
        assert_eq!(label.amount_upgrades, 4);
        assert_eq!(label.paid_amount, 4);
        assert_eq!(label.paid_speed, 1);
        assert_eq!(label.refund_items(), 6);
    }

    #[test]
    fn an_eye_reaches_the_labelled_block_it_touches_but_not_one_across_the_room() {
        let eye = [0.5, 65.62, 0.5];
        assert!(within_reach(eye, [0, 64, 0], 4.5));
        assert!(within_reach(eye, [4, 64, 0], 4.5));
        assert!(!within_reach(eye, [5, 64, 0], 4.5));
    }

    #[test]
    fn the_list_is_paged_so_a_large_level_never_floods_the_chat() {
        assert_eq!(LIST_PAGE_SIZE, 16);
        assert_eq!(list_page(0, 0), (0, 0));
        assert_eq!(list_page(10, 0), (0, 10));
        assert_eq!(list_page(100, 0), (0, 16));
        assert_eq!(list_page(100, 1), (16, 32));
        assert_eq!(list_page(100, 6), (96, 100));
        assert_eq!(list_page(100, 7), (100, 100));
        assert_eq!(list_page(100, 9_000), (100, 100), "past the end is empty");
    }

    #[test]
    fn the_command_lists_one_page_at_a_time() {
        let parse = |line: &str| Command::parse(&line.split_whitespace().collect::<Vec<_>>());
        assert_eq!(
            parse("transfer_labels list 2"),
            Ok(Command::List { page: 1 })
        );
        assert_eq!(
            parse("transfer_labels list"),
            Ok(Command::List { page: 0 }),
            "no number is the first page"
        );
        assert!(parse("transfer_labels list 0").is_err());
        assert!(parse("transfer_labels list 2 3").is_err());
    }

    #[test]
    fn the_entry_count_defaults_to_one() {
        assert_eq!(
            Command::parse(&words("transfer_labels entry 10 64 -3 up 3 minecraft:dirt")),
            Ok(Command::Entry {
                key: LabelKey {
                    pos: [10, 64, -3],
                    face: Face::Up
                },
                slot: 3,
                item: "minecraft:dirt".to_owned(),
                count: 1
            })
        );
    }

    // --- the insert label (slice 2, 2026-11-21) ---

    #[test]
    fn the_two_label_items_name_the_two_flows() {
        assert_eq!(
            LabelFlow::of_item(EXTRACT_LABEL_ITEM),
            Some(LabelFlow::Extract)
        );
        assert_eq!(
            LabelFlow::of_item(INSERT_LABEL_ITEM),
            Some(LabelFlow::Insert)
        );
        assert_eq!(
            LabelFlow::of_item("minecraft:chest"),
            None,
            "a chest is not a label item"
        );
        for flow in [LabelFlow::Extract, LabelFlow::Insert] {
            assert_eq!(
                LabelFlow::of_item(flow.item_id()),
                Some(flow),
                "{} names its own item",
                flow.name()
            );
            assert_eq!(LabelFlow::parse(flow.name()), Some(flow), "{}", flow.name());
        }
        assert_eq!(LabelFlow::parse("sideways"), None);
    }

    #[test]
    fn a_fresh_label_of_either_item_extracts_until_it_is_told_otherwise() {
        assert_eq!(LabelConfig::default().flow, LabelFlow::Extract);
        assert_eq!(LabelConfig::of(LabelFlow::Insert).flow, LabelFlow::Insert);
        assert_eq!(
            LabelConfig::of(LabelFlow::Insert).interval_ticks(),
            20,
            "the insert label moves at the same rate"
        );
        assert_eq!(
            LabelConfig::of(LabelFlow::Insert).transfer_amount(),
            1,
            "and the same amount"
        );
    }

    #[test]
    fn an_insert_label_takes_from_the_neighbour_and_gives_to_the_labelled_block() {
        let key = LabelKey {
            pos: [10, 64, -3],
            face: Face::Up,
        };
        let (source, sink) = key.endpoints(LabelFlow::Extract);
        assert_eq!(
            source,
            Endpoint {
                pos: [10, 64, -3],
                face: Face::Up
            },
            "an extract label pulls from the block it is stuck to"
        );
        assert_eq!(
            sink,
            Endpoint {
                pos: [10, 65, -3],
                face: Face::Down
            },
            "and pushes into the block across the face"
        );
        let (source, sink) = key.endpoints(LabelFlow::Insert);
        assert_eq!(
            source,
            Endpoint {
                pos: [10, 65, -3],
                face: Face::Down
            },
            "an insert label pulls from the block across the face"
        );
        assert_eq!(
            sink,
            Endpoint {
                pos: [10, 64, -3],
                face: Face::Up
            },
            "and pushes into the block it is stuck to"
        );
    }

    #[test]
    fn a_label_saves_its_flow_and_an_old_text_reads_as_extract() {
        let mut label = LabelConfig::of(LabelFlow::Insert);
        label.amount_upgrades = 2;
        let back = LabelConfig::decode(&label.encode()).expect("its own text decodes");
        assert_eq!(back.flow, LabelFlow::Insert);
        assert_eq!(back.amount_upgrades, 2, "the rest still round-trips");
        let old = LabelConfig::decode("mode=blacklist\nkind=normal\namount=4\nspeed=1\n")
            .expect("a 0.2.1 text");
        assert_eq!(
            old.flow,
            LabelFlow::Extract,
            "a label saved before the insert label existed is an extract label"
        );
    }

    #[test]
    fn a_removal_hands_back_the_item_that_was_attached() {
        assert_eq!(
            LabelConfig::of(LabelFlow::Extract).refund_item(),
            EXTRACT_LABEL_ITEM
        );
        assert_eq!(
            LabelConfig::of(LabelFlow::Insert).refund_item(),
            INSERT_LABEL_ITEM
        );
    }
}
