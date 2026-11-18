#!/usr/bin/env bash
# In-game check for Transfer Labels slice 1: an item-stack extract label on a
# block face moves items from that block into the container across the face, on
# its own tick, at the jar's rate and under the jar's filter rules.
#
# Every mod command is typed with `ctl chat "/cmd"` because `ctl command` runs
# the console path and bypasses the native `perform_command` hook.
#
# The End, in the void at x=200 (away from the central island, its dragon egg and
# the ambient item entities of a generated level). Each case owns a column: a
# chest at y=101 (the labelled block) and one at y=102 (the target above it).
# The label is attached *before* the target is placed, because a label sits on
# the face between the two blocks and that face is covered afterwards — the
# upstream label is clicked through its own thin voxel shape, which this slice
# does not draw.
#
# Default run:
#  A  a full chest under an empty one, with no label, moves nothing over 40 ticks
#     (the control: the harness tells "no label" from "label").
#  B  a label on the top face moves exactly one item per 20 game ticks: the items
#     gained equal floor(T1/20) - floor(T0/20) over a 200-tick window.
#  C  a whitelist naming dirt moves no cobblestone over 60 ticks; naming
#     cobblestone moves it (the filter is not vacuous).
#  D  speed 38 makes the interval one tick: the items gained equal the ticks
#     between the two reads.
#  E  amount 63 moves 64 items in one operation (at most three ticks pass).
#  F  a regulating entry of 10 never moves more than four at once even with
#     amount 63, and stops at exactly 10 on the destination.
#  G  left-clicking a label with the label item in hand removes it, gives the
#     label back, and leaves the block standing (in creative an uncancelled click
#     would break it).
#  H  the label survives a save and a relaunch of the same world, and keeps
#     moving items afterwards (the level attachment's codec).
#
# Default run (EMBEDDED=0): the game loads `content/` as a dev mod.
# EMBEDDED=1: the game is launched with **only the native bundle** (no
# `--dev-mod`) and the content comes from the bundle's embedded copy
# (`mixins/<id>/data/…`, the AE2 pattern). The run also checks the file the
# helper wrote; CONTROL=1 (default with EMBEDDED) first launches the same binary
# with `.horizon-cache` unusable, where the item id must be unknown.
#
# SELFTEST=1 flips B's rate assertion (it must FAIL), proving the scenario can
# fail.
set -Eeuo pipefail
export LC_ALL=C
[ -z "${TL_DEBUG:-}" ] || set -x

EMBEDDED=${EMBEDDED:-0}
CONTROL=${CONTROL:-$EMBEDDED}
SELFTEST=${SELFTEST:-0}

CLONE=${CLONE:-$(cd "$(dirname "$0")/../../.." && pwd)}
PORT=$CLONE/ports/transfer_labels
PIN_BIN=$CLONE/ports/_shared/bin/horizon-1d63e0a5
CTL=${CTL:-$PIN_BIN/horizon-ctl}
GAME=${GAME:-$PORT/target/horizon-cargo/fast/horizon-client}
RES=${RES:-${HORIZON_RES:-/home/adlf/Desktop/horizon/.local/horizon-client/26.3}}
SOCK=${SOCK:-/tmp/opencode/transfer_labels-e2e.sock}
KEEP=${KEEP:-0}
WORLD=TransferLabelsE2E
CONTROL_WORLD=TransferLabelsControl
LABEL=transfer_labels:itemstack_extract_transfer_label
# Where the arena stands: the End's void, far from the central island.
X0=200
SX=$((X0 + 2))

command -v jq >/dev/null || { echo "missing jq (every assertion parses a ctl reply)" >&2; exit 1; }
[ -x "$CTL" ] || { echo "missing horizon-ctl: $CTL" >&2; exit 1; }
[ -x "$GAME" ] || { echo "missing horizon-client: $GAME" >&2; exit 1; }
[ -d "$RES/assets" ] || { echo "missing prepared resources: $RES" >&2; exit 1; }

SCRATCH=$(mktemp -d /tmp/opencode/transfer_labels-home.XXXXXX)
export HORIZON_HOME=$SCRATCH/game
export HORIZON_LAUNCHER_HOME=$SCRATCH/launcher
export XDG_DATA_HOME=$SCRATCH/data
export XDG_CONFIG_HOME=$SCRATCH/config
export HORIZON_NO_AUDIO=1
mkdir -p "$(dirname "$SOCK")" "$XDG_CONFIG_HOME"
rm -f "$SOCK"

ctl() { "$CTL" --socket "$SOCK" "$@"; }
say() { ctl chat "/$1" >/dev/null; }
fail() { echo "FAIL: $*" >&2; ctl logs --lines 60 >&2 || true; exit 1; }
on_error() { echo "FAIL: line $1: '$2' exited $?" >&2; ctl logs --lines 40 >&2 || true; }
trap 'on_error "$LINENO" "$BASH_COMMAND"' ERR

cleanup() {
    local status=$?
    ctl quit >/dev/null 2>&1 || true
    sleep 1
    if [ "$KEEP" = 1 ]; then
        echo "kept scratch: $SCRATCH" >&2
    else
        rm -rf "$SCRATCH" 2>/dev/null || true
    fi
    # Last line of every run: `check-e2e-logs.py` reads it as the verdict.
    echo "EXIT:$status"
}
trap cleanup EXIT

launch() { # launch [WORLD]
    local world=${1:-$WORLD}
    rm -f "$SOCK"
    cd "$SCRATCH"
    local args=(--game "$GAME" --world "$world" --seed 20261105 --creative --headless
                --wait --resources "$RES")
    if [ "$EMBEDDED" = 1 ]; then
        echo "  launch: no --dev-mod; the bundle carries its own content" >&2
    else
        args+=(--dev-mod "$PORT/content")
    fi
    "$CTL" --socket "$SOCK" launch "${args[@]}" >/dev/null
    ctl wait-ready 120
    ctl wait 40
}

block_name() { ctl block "$1" "$2" "$3" | jq -r '.name // .block // empty'; }

# The End's void at X0: a stone floor and clear air above it. The player is
# teleported first, so the chunks the fills touch are loaded.
arena() {
    ctl command "execute in minecraft:the_end run tp @s $X0.5 104 0.5" >/dev/null
    local loaded=false
    for _ in $(seq 1 120); do
        ctl wait 5 >/dev/null
        if [ "$(ctl block "$X0" 100 0 | jq -r '.loaded // false')" = true ]; then
            loaded=true
            break
        fi
    done
    [ "$loaded" = true ] || fail "the End arena chunk never loaded"
    ctl command "execute in minecraft:the_end run fill $((X0 - 2)) 100 -26 $((X0 + 4)) 100 26 minecraft:stone" >/dev/null
    ctl wait 6 >/dev/null
    ctl command "execute in minecraft:the_end run fill $((X0 - 2)) 101 -26 $((X0 + 4)) 108 26 minecraft:air" >/dev/null
    ctl wait 6 >/dev/null
    ctl command "execute in minecraft:the_end run kill @e[type=!player]" >/dev/null
    ctl command "execute in minecraft:the_end run tp @s $X0.5 101 0.5" >/dev/null
    ctl wait 20
    [ "$(block_name "$X0" 100 0)" = "minecraft:stone" ] || fail "the arena floor is not stone"
    [ "$(block_name "$X0" 101 0)" = "minecraft:air" ] || fail "the arena air above the floor is not air"
}

# Put the label item in the first hotbar slot and select it: `give` may fill a
# main-inventory slot that `select ITEM` cannot reach.
hold_label() {
    ctl command "item replace entity @s hotbar.0 with $LABEL 3" >/dev/null
    ctl wait 4 >/dev/null
    ctl select 1 >/dev/null
    ctl wait 2 >/dev/null
}

# Stand where both faces of the column are reachable.
stand_at() { # stand_at Z
    ctl command "execute in minecraft:the_end run tp @s $(($SX - 2)).5 101 $1.5" >/dev/null
    ctl wait 6 >/dev/null
}

# A chest at y=101 (the labelled block) and one above it at y=102 (the target),
# with no label anywhere near them.
pair() { # pair Z
    local z=$1
    ctl command "execute in minecraft:the_end run fill $SX 101 $z $SX 102 $z minecraft:chest" >/dev/null
    ctl wait 6 >/dev/null
    [ "$(block_name "$SX" 101 "$z")" = "minecraft:chest" ] || fail "no chest at $SX,101,$z"
    [ "$(block_name "$SX" 102 "$z")" = "minecraft:chest" ] || fail "no chest at $SX,102,$z"
}

# The labelled block only; the target is placed after the label is on its face.
lower_chest() { # lower_chest Z
    ctl command "execute in minecraft:the_end run setblock $SX 101 $1 minecraft:chest" >/dev/null
    ctl wait 6 >/dev/null
    [ "$(block_name "$SX" 101 "$1")" = "minecraft:chest" ] || fail "no chest at $SX,101,$1"
}

# Attach a label to the top face of the chest at (SX, 101, Z), then build the
# target above it.
attach_top() { # attach_top Z
    local z=$1
    lower_chest "$z"
    stand_at "$z"
    hold_label
    ctl look-at-block "$SX" 101 "$z" up >/dev/null
    ctl wait 2 >/dev/null
    ctl interact "$SX" 101 "$z" up >/dev/null
    ctl wait 6 >/dev/null
    info "$SX" 101 "$z" up
    ctl command "execute in minecraft:the_end run setblock $SX 102 $z minecraft:chest" >/dev/null
    ctl wait 6 >/dev/null
}

fill_source() { # fill_source Z ITEM COUNT
    ctl container "$SX" 101 "$1" "$2" "$3" >/dev/null
    ctl wait 4 >/dev/null
}

carried_labels() {
    ctl inventory | jq '[.hotbar[], .main[] | select(.item == "'"$LABEL"'") | .count] | add // 0'
}

count_in() { # count_in Y Z ITEM
    ctl container "$SX" "$1" "$2" \
        | jq --arg item "$3" '[.slots[] | select(.item == $item) | .count] | add // 0'
}

# `/transfer_labels info X Y Z FACE`: the line, then its fields in FACE, MODE,
# TYPE, AMOUNT, SPEED, INTERVAL, PER_OP, ENTRIES, TICKS and ok.
info() { # info X Y Z FACE
    say "transfer_labels info $1 $2 $3 $4"
    ctl wait 12 >/dev/null
    # `ctl messages` is newest first, so the first match is the reply just sent.
    INFO_LINE=$(ctl messages 12 | jq -r '.chat[]' | grep -m1 'label .* at ' || true)
    [ -n "$INFO_LINE" ] || fail "no /transfer_labels info reply"
    eval "$(python3 "$PORT/tools/parse_info.py" "$INFO_LINE")"
    [ "${ok:-0}" = 1 ] || fail "the info line did not parse: $INFO_LINE"
    echo "  $INFO_LINE" >&2
}

label_cmd() { # label_cmd ARGS...
    say "transfer_labels $*"
    ctl wait 8 >/dev/null
}

expect_no_label() { # expect_no_label X Y Z FACE
    say "transfer_labels info $1 $2 $3 $4"
    ctl wait 12 >/dev/null
    ctl messages 12 | jq -r '.chat[]' | grep -q 'No transfer label on the' \
        || fail "a label is still on the $4 face of $1,$2,$3"
}

# The control for EMBEDDED=1: the same binary (every hook is in it) with the
# embedded content made unavailable — `.horizon-cache` is a plain file, so the
# helper can neither compare nor write its content file and falls back to
# `Continue`. The item id must then be unknown.
control_leg() {
    echo "== control: the same game with no embedded content file =="
    rm -rf "$SCRATCH/.horizon-cache"
    : > "$SCRATCH/.horizon-cache"
    launch "$CONTROL_WORLD"
    arena
    ctl command "item replace entity @s hotbar.0 with $LABEL 1" >/dev/null || true
    ctl wait 4 >/dev/null
    ctl inventory | jq -e '[.hotbar[], .main[] | select(.item == "'"$LABEL"'")] | length == 0' >/dev/null \
        || fail "the control game has the label item, so the content did not come from the embedded file"
    ctl command "execute in minecraft:the_end run setblock $SX 101 0 $LABEL" >/dev/null || true
    ctl wait 4 >/dev/null
    local name
    name=$(block_name "$SX" 101 0)
    [ "$name" = "minecraft:air" ] || fail "the control game holds $name, so the item id was known"
    [ -f "$SCRATCH/.horizon-cache" ] || fail "the control's .horizon-cache was replaced: the helper wrote its content"
    echo "  without the embedded file: no label item, the block stayed $name"
    ctl quit >/dev/null 2>&1 || true
    rm -f "$SCRATCH/.horizon-cache"
}

# EMBEDDED=1: the bundle must have written its content out at startup, byte for
# byte the delivered content mod.
embedded_content_check() {
    local cache=$SCRATCH/.horizon-cache/transfer_labels-embedded-content.hmod
    [ -f "$cache" ] || fail "the bundle did not write its embedded content to $cache"
    cmp -s "$cache" "$PORT/content/transfer_labels.hmod" \
        || fail "the embedded content differs from content/transfer_labels.hmod"
    echo "  embedded content: $(wc -c < "$cache" | tr -d ' ') B at $cache, identical to content/transfer_labels.hmod"
}

echo "== launch (The End arena, content + native bundle) =="
if [ "$CONTROL" = 1 ]; then control_leg; fi
launch
if [ "$EMBEDDED" = 1 ]; then embedded_content_check; fi
arena

ctl command "item replace entity @s hotbar.0 with $LABEL 1" >/dev/null
ctl wait 4 >/dev/null
ctl inventory | jq -e '[.hotbar[], .main[] | select(.item == "'"$LABEL"'")] | length >= 1' >/dev/null \
    || fail "the label item did not register: $(ctl inventory | jq -c '{hotbar,main}')"

echo "== A: a full chest under an empty one, with no label, moves nothing =="
pair -18
fill_source -18 minecraft:cobblestone 64
[ "$(count_in 101 -18 minecraft:cobblestone)" = 64 ] || fail "the source chest did not take 64 cobblestone"
ctl wait 40 >/dev/null
[ "$(count_in 102 -18 minecraft:cobblestone)" = 0 ] \
    || fail "a chest with no label transferred items: $(ctl container "$SX" 102 -18 | jq -c .)"
echo "  no label: the target holds 0, the source 64"

echo "== B: one item per 20 game ticks =="
attach_top -12
info "$SX" 101 -12 up
[ "$INTERVAL" = 20 ] || fail "a fresh label's interval is $INTERVAL ticks, expected 20"
[ "$PER_OP" = 1 ] || fail "a fresh label moves $PER_OP items an operation, expected 1"
fill_source -12 minecraft:cobblestone 64
ctl wait 20 >/dev/null
info "$SX" 101 -12 up
T0=$TICKS
C0=$DEST
ctl wait 200 >/dev/null
info "$SX" 101 -12 up
T1=$TICKS
C1=$DEST
WANT=$((T1 / 20 - T0 / 20))
[ "$((C1 - C0))" = "$WANT" ] \
    || fail "$((C1 - C0)) items moved over $((T1 - T0)) ticks, expected $WANT (one per 20 ticks)"
[ "$WANT" -ge 5 ] || fail "only $WANT activations in the window: the scenario measured nothing"
if [ "$SELFTEST" = 1 ]; then
    echo "(selftest: asserting one item more than the ticks allow)"
    [ "$((C1 - C0))" = "$((WANT + 1))" ] || fail "selftest: $((C1 - C0)) items moved, expected the wrong count"
fi
echo "  ticks $T0..$T1: $WANT activations, $((C1 - C0)) items moved"

echo "== C: a whitelist naming another item moves nothing, then the right one moves =="
attach_top -6
label_cmd filter "$SX" 101 -6 up whitelist
label_cmd entry "$SX" 101 -6 up 0 minecraft:dirt
info "$SX" 101 -6 up
[ "$MODE" = whitelist ] || fail "the label's mode is $MODE, expected whitelist"
fill_source -6 minecraft:cobblestone 64
ctl wait 60 >/dev/null
[ "$(count_in 102 -6 minecraft:cobblestone)" = 0 ] \
    || fail "a whitelist naming dirt let cobblestone through"
echo "  whitelist dirt: 0 cobblestone moved in 60 ticks"
label_cmd clear "$SX" 101 -6 up 0
label_cmd entry "$SX" 101 -6 up 0 minecraft:cobblestone
ctl wait 40 >/dev/null
MOVED=$(count_in 102 -6 minecraft:cobblestone)
[ "$MOVED" -gt 0 ] || fail "a whitelist naming cobblestone moved nothing"
echo "  whitelist cobblestone: $MOVED moved"
label_cmd filter "$SX" 101 -6 up blacklist
label_cmd entry "$SX" 101 -6 up 1 minecraft:cobblestone
ctl wait 20 >/dev/null
BEFORE=$(count_in 102 -6 minecraft:cobblestone)
ctl wait 40 >/dev/null
[ "$(count_in 102 -6 minecraft:cobblestone)" = "$BEFORE" ] \
    || fail "a blacklist naming cobblestone still let it through"
echo "  blacklist cobblestone: still $BEFORE, nothing more"

echo "== D: speed 38 is one item a tick =="
attach_top 0
label_cmd speed "$SX" 101 0 up 38
info "$SX" 101 0 up
[ "$INTERVAL" = 1 ] || fail "speed 38 gives an interval of $INTERVAL ticks, expected 1"
fill_source 0 minecraft:cobblestone 64
ctl wait 4 >/dev/null
info "$SX" 101 0 up
T0=$TICKS
C0=$DEST
# Twenty ticks of window (plus the info round trips): a 64-item source covers it,
# and one item a tick over a shorter window is the same claim.
ctl wait 20 >/dev/null
info "$SX" 101 0 up
T1=$TICKS
C1=$DEST
[ "$((C1 - C0))" = "$((T1 - T0))" ] \
    || fail "$((C1 - C0)) items moved over $((T1 - T0)) ticks, expected one a tick"
[ "$((T1 - T0))" -ge 20 ] || fail "only $((T1 - T0)) ticks passed: the window measured nothing"
echo "  ticks $T0..$T1: $((C1 - C0)) items, one a tick"

echo "== E: amount 63 moves 64 items in one operation =="
attach_top 6
label_cmd amount "$SX" 101 6 up 63
info "$SX" 101 6 up
[ "$PER_OP" = 64 ] || fail "amount 63 moves $PER_OP items an operation, expected 64"
[ "$INTERVAL" = 20 ] || fail "the interval here is $INTERVAL, so the first read cannot be one operation"
fill_source 6 minecraft:cobblestone 64
# The first non-zero read is the first activation's whole haul: 5-tick polls
# cannot skip one of the 20-tick boundaries.
FIRST=0
for _ in $(seq 1 12); do
    ctl wait 5 >/dev/null
    FIRST=$(count_in 102 6 minecraft:cobblestone)
    [ "$FIRST" = 0 ] || break
done
[ "$FIRST" = 64 ] || fail "the first operation after the fill moved $FIRST items, expected 64"
echo "  the first activation carried all 64"

echo "== F: a regulating entry of 10 caps an operation at 4 and stops at 10 =="
attach_top 12
label_cmd filter "$SX" 101 12 up whitelist
label_cmd type "$SX" 101 12 up regulating
label_cmd entry "$SX" 101 12 up 0 minecraft:cobblestone 10
label_cmd amount "$SX" 101 12 up 63
info "$SX" 101 12 up
[ "$TYPE" = regulating ] || fail "the label's filter type is $TYPE, expected regulating"
[ "$AMOUNT" = 63 ] || fail "the label's amount is $AMOUNT, expected 63"
fill_source 12 minecraft:cobblestone 64
ctl wait 25 >/dev/null
EARLY=$(count_in 102 12 minecraft:cobblestone)
[ "$EARLY" != 10 ] && [ "$EARLY" -le 8 ] \
    || fail "after ~25 ticks the target holds $EARLY: the four-item cap did not hold"
[ "$EARLY" -gt 0 ] || fail "the regulating label moved nothing"
echo "  after ~25 ticks: $EARLY on the target (never 10, never 64)"
ctl wait 100 >/dev/null
SETTLED=$(count_in 102 12 minecraft:cobblestone)
[ "$SETTLED" = 10 ] || fail "the regulating filter settled at $SETTLED, expected 10"
ctl wait 40 >/dev/null
[ "$(count_in 102 12 minecraft:cobblestone)" = 10 ] || fail "the regulating filter went past 10"
echo "  settled at exactly 10 and stayed"

echo "== G: left-clicking the label removes it without breaking the block =="
attach_top 18
fill_source 18 minecraft:cobblestone 8
ctl wait 40 >/dev/null
[ "$(count_in 102 18 minecraft:cobblestone)" -gt 0 ] || fail "the label moved nothing before removal"
ctl command "execute in minecraft:the_end run setblock $SX 102 18 minecraft:air" >/dev/null
ctl wait 6 >/dev/null
stand_at 18
hold_label
CARRIED_BEFORE=$(carried_labels)
ctl look-at-block "$SX" 101 18 up >/dev/null
ctl wait 2 >/dev/null
ctl mouse left click >/dev/null
ctl wait 6 >/dev/null
[ "$(block_name "$SX" 101 18)" = "minecraft:chest" ] \
    || fail "the left click broke the labelled chest instead of taking the label"
expect_no_label "$SX" 101 18 up
CARRIED_AFTER=$(carried_labels)
[ "$CARRIED_AFTER" = "$((CARRIED_BEFORE + 1))" ] \
    || fail "the label carried $CARRIED_BEFORE items before the click and $CARRIED_AFTER after: expected one more"
echo "  the chest stands, the label is gone, $CARRIED_BEFORE -> $CARRIED_AFTER label items"

echo "== H: the label survives a save and a relaunch =="
# Case D's source is empty by now; a fresh stack in a second slot gives the same
# label something to move after the world comes back.
fill_source 0 minecraft:cobblestone 64
ctl save >/dev/null
ctl wait 4 >/dev/null
info "$SX" 101 0 up
BEFORE=$(count_in 102 0 minecraft:cobblestone)
ctl quit >/dev/null 2>&1 || true
sleep 2
launch "$WORLD"
ctl command "execute in minecraft:the_end run tp @s $X0.5 101 0.5" >/dev/null
ctl wait 20 >/dev/null
say "transfer_labels list"
ctl wait 12 >/dev/null
LIST=$(ctl messages 12 | jq -r '.chat[]' | grep -m1 'transfer labels in this level' || true)
echo "  $LIST" >&2
[ -n "$LIST" ] || fail "the relaunched world has no labels: the level attachment did not come back"
info "$SX" 101 0 up
ctl wait 60 >/dev/null
AFTER=$(count_in 102 0 minecraft:cobblestone)
[ "$AFTER" -gt "$BEFORE" ] \
    || fail "the label did not move anything after the relaunch ($BEFORE -> $AFTER)"
echo "  after the relaunch: $BEFORE -> $AFTER moved by the same label"

echo "PASS: a labelled face moves exactly one item per 20 game ticks, a whitelist that does not match moves nothing while a matching one moves, speed 38 gives one item a tick, amount 63 moves 64 in one operation, a regulating entry of 10 never moves more than four at once and stops at 10, a removed label returns its item without breaking the block, and a label survives a save and a relaunch"
