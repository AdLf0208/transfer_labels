#!/usr/bin/env bash
# In-game check for Transfer Labels: the two item-stack labels on a block face move
# items between that block and the container across the face, on their own tick, at
# the jar's rate and under the jar's filter rules; every carried recipe crafts.
#
# Every mod command is typed with `ctl chat "/cmd"` because `ctl command` runs
# the console path and bypasses the native `perform_command` hook.
#
# The End, in the void at x=200 (away from the central island, its dragon egg and
# the ambient item entities of a generated level). Each case owns a column: a
# chest at y=101 and one at y=102. A label is attached *before* the second block is
# placed, because a label sits on the face between the two blocks and that face is
# covered afterwards — the upstream label is clicked through its own thin voxel
# shape, which this slice does not draw.
#
# Default run:
#  A  a full chest under an empty one, with no label, moves nothing over 40 ticks
#     (the control: the harness tells "no label" from "label").
#  B  an extract label on the top face moves exactly one item per 20 game ticks: the
#     items gained equal floor(T1/20) - floor(T0/20) over a 200-tick window.
#  C  a whitelist naming dirt moves no cobblestone over 60 ticks; naming
#     cobblestone moves it (the filter is not vacuous).
#  D  speed 38 makes the interval one tick: the items gained equal the ticks
#     between the two reads.
#  E  amount 63 moves 64 items in one operation (at most three ticks pass).
#  F  a regulating entry of 10 never moves more than four at once even with
#     amount 63, and stops at exactly 10 on the destination.
#  G  left-clicking a label with a label item in hand removes it, gives the label
#     back, and leaves the block standing (in creative an uncancelled click
#     would break it).
#  G2 an edit from out of reach is refused and changes nothing;
#  G3 `list` prints one page with a header, never the whole level.
#  I  the insert label is the opposite rule on the same geometry: stuck to the
#     upper chest's bottom face it fills that chest from the lower one, at the same
#     rate and without losing or duplicating an item; two chests with no label move
#     nothing, and an extract label in the same geometry moves nothing either (it
#     pulls from the empty labelled chest), so the flow is pinned both ways.
#  H  the label survives a save and a relaunch of the same world, and keeps
#     moving items afterwards (the level attachment's codec) — including the insert
#     label's saved flow.
#  J  every carried recipe (extract, insert, and the two shapeless conversions)
#     crafts in a real crafting table, with the output id and count and the
#     ingredient consumption asserted; a control grid makes nothing.
#
# FORMAT (2026-11-18): this port is one flat SDK package, so the code and the content
# travel together. `GAME` must be the game the **launcher** built from this package
# (`ports/tools/launcher-game.sh run transfer_labels` supplies it, with `CTL`,
# `CONTROL_GAME` and `RES`); there is no `--dev-mod` any more.
#
# CONTROL=1 (the launcher run sets `CONTROL_GAME`) first launches the same Nightly
# **without** the package, where the item ids must be unknown.
#
# SELFTEST=1 flips B's rate assertion and INSERT_SELFTEST=1 flips I's direction
# assertion (each must FAIL), proving the scenario can fail.
set -Eeuo pipefail
export LC_ALL=C
[ -z "${TL_DEBUG:-}" ] || set -x

CONTROL=${CONTROL:-0}
SELFTEST=${SELFTEST:-0}
INSERT_SELFTEST=${INSERT_SELFTEST:-0}

CLONE=${CLONE:-$(cd "$(dirname "$0")/../../.." && pwd)}
PORT=$CLONE/ports/transfer_labels
PIN_BIN=$CLONE/ports/_shared/bin/horizon-1d63e0a5
CTL=${CTL:-}
GAME=${GAME:-}
CONTROL_GAME=${CONTROL_GAME:-}
RES=${RES:-${HORIZON_RES:-/home/adlf/Desktop/horizon/.local/horizon-client/26.3}}
SOCK=${SOCK:-/tmp/opencode/transfer_labels-e2e.sock}
KEEP=${KEEP:-0}
WORLD=TransferLabelsE2E
CONTROL_WORLD=TransferLabelsControl
LABEL=transfer_labels:itemstack_extract_transfer_label
INSERT=transfer_labels:itemstack_insert_transfer_label
# Where the arena stands: the End's void, far from the central island.
X0=200
SX=$((X0 + 2))

command -v jq >/dev/null || { echo "missing jq (every assertion parses a ctl reply)" >&2; exit 1; }
[ -x "$CTL" ] || { echo "missing horizon-ctl: $CTL" >&2; exit 1; }
[ -x "$GAME" ] || { echo "missing horizon-client: $GAME" >&2; exit 1; }
[ -d "$RES/assets" ] || { echo "missing prepared resources: $RES" >&2; exit 1; }

# The package this run is about: print its bytes and digest, so the log says which
# bytes were in the game and not just that something ran.
if [ -f "$PORT/transfer_labels.hmod" ]; then
    echo "package: $(stat -c%s "$PORT/transfer_labels.hmod") B sha256 $(sha256sum "$PORT/transfer_labels.hmod" | cut -d' ' -f1)"
fi
echo "manifest: $(grep -m1 '^version' "$PORT/horizon-mod.toml")"

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

launch() { # launch [WORLD] [GAME]
    local world=${1:-$WORLD}
    local game=${2:-$GAME}
    rm -f "$SOCK"
    cd "$SCRATCH"
    # `--offscreen`, not `--headless`: the recipe phase opens a crafting-table menu
    # and clicks its slots, and a headless client never opens a screen (the same
    # reason pocketstorage and chisel launch offscreen for their craft phases).
    "$CTL" --socket "$SOCK" launch --game "$game" --world "$world" --seed 20261105 \
        --creative --offscreen --wait --resources "$RES" >/dev/null
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

# Put a label item in the first hotbar slot and select it: `give` may fill a
# main-inventory slot that `select ITEM` cannot reach.
hold_label() { # hold_label [ITEM]
    local item=${1:-$LABEL}
    ctl command "item replace entity @s hotbar.0 with $item 3" >/dev/null
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

# Attach a label to the top face of the chest at (SX, 101, Z), then build the chest
# above it. `ITEM`/`FLOW` say which label; the line's own flow must match, so a label
# attached with the wrong item is caught here rather than in the behaviour below.
attach_top() { # attach_top Z [ITEM] [FLOW]
    local z=$1 item=${2:-$LABEL} flow=${3:-extract}
    lower_chest "$z"
    stand_at "$z"
    hold_label "$item"
    ctl look-at-block "$SX" 101 "$z" up >/dev/null
    ctl wait 2 >/dev/null
    ctl interact "$SX" 101 "$z" up >/dev/null
    ctl wait 6 >/dev/null
    info "$SX" 101 "$z" up
    [ "$FLOW" = "$flow" ] \
        || fail "the label on the top face reads flow=$FLOW, expected $flow"
    ctl command "execute in minecraft:the_end run setblock $SX 102 $z minecraft:chest" >/dev/null
    ctl wait 6 >/dev/null
    [ "$(block_name "$SX" 102 "$z")" = "minecraft:chest" ] || fail "no chest at $SX,102,$z"
}

# Fill a container at a given height: the plain pair fills y=101, the insert cases
# fill the upper chest at y=102 (which is their source).
fill_at() { # fill_at Y Z ITEM COUNT
    ctl container "$SX" "$1" "$2" "$3" "$4" >/dev/null
    ctl wait 4 >/dev/null
}

fill_source() { # fill_source Z ITEM COUNT
    fill_at 101 "$1" "$2" "$3"
}

carried_labels() {
    ctl inventory | jq '[.hotbar[], .main[] | select(.item == "'"$LABEL"'") | .count] | add // 0'
}

count_in() { # count_in Y Z ITEM
    ctl container "$SX" "$1" "$2" \
        | jq --arg item "$3" '[.slots[] | select(.item == $item) | .count] | add // 0'
}

# Two container reads are not atomic: with one item moving every 20 ticks an
# activation can land between them, so the pair shows the item in neither container
# (it is in the destination by the time the second read runs). Read the sink, the
# source, then the sink again and only trust the pair when the two sink reads agree
# — then no activation happened inside the window and the total is exact.
# Echoes "sink source".
stable_pair() { # stable_pair SINK_Y SOURCE_Y Z ITEM
    local sink source again tries=0
    while [ "$tries" -lt 30 ]; do
        tries=$((tries + 1))
        sink=$(count_in "$1" "$3" "$4")
        source=$(count_in "$2" "$3" "$4")
        again=$(count_in "$1" "$3" "$4")
        if [ "$sink" = "$again" ]; then
            echo "$sink $source"
            return 0
        fi
        ctl wait 2 >/dev/null
    done
    return 1
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

# A rate window must not end on the label's own tick boundary: the command runs
# before the level tick, so an `info` read at a game time that is a multiple of 20
# can be taken *before* that tick's activation and the count misses it, while
# floor(T1/20) - floor(T0/20) counts it (measured: dest 12 at ticks=1880 with 13
# activations due). Read until both ends sit off the boundary, then the formula is
# exact and a real skipped activation still fails.
read_off_boundary() { # read_off_boundary X Y Z FACE
    local tries=0
    info "$1" "$2" "$3" "$4"
    while [ "$((TICKS % 20))" = 0 ] && [ "$tries" -lt 5 ]; do
        tries=$((tries + 1))
        ctl wait 3 >/dev/null
        info "$1" "$2" "$3" "$4"
    done
}

label_cmd() { # label_cmd ARGS...
    say "transfer_labels $*"
    ctl wait 8 >/dev/null
    ctl messages 4 | jq -r '.chat[]' | head -2 | sed 's/^/  | /' >&2
}

expect_no_label() { # expect_no_label X Y Z FACE
    say "transfer_labels info $1 $2 $3 $4"
    ctl wait 12 >/dev/null
    ctl messages 12 | jq -r '.chat[]' | grep -q 'No transfer label on the' \
        || fail "a label is still on the $4 face of $1,$2,$3"
}

# The control: the same Nightly, unmodified (the launcher exports it as
# `CONTROL_GAME`), so the only difference is the package. The item id must be unknown.
control_leg() {
    echo "== control: the same Nightly without this package =="
    launch "$CONTROL_WORLD" "$CONTROL_GAME"
    arena
    ctl command "item replace entity @s hotbar.0 with $LABEL 1" >/dev/null 2>&1 || true
    ctl wait 4 >/dev/null
    ctl inventory | jq -e '[.hotbar[], .main[] | select(.item == "'"$LABEL"'")] | length == 0' >/dev/null \
        || fail "the control game has the label item, so the package was in it"
    ctl command "execute in minecraft:the_end run setblock $SX 101 0 $LABEL" >/dev/null 2>&1 || true
    ctl wait 4 >/dev/null
    local name
    name=$(block_name "$SX" 101 0)
    [ "$name" = "minecraft:air" ] || fail "the control game holds $name, so the item id was known"
    echo "  without the package: no label item, the block stayed $name"
    ctl quit >/dev/null 2>&1 || true
}

echo "== launch (The End arena, content + native bundle) =="
if [ "$CONTROL" = 1 ]; then
    [ -n "$CONTROL_GAME" ] || { echo "CONTROL=1 needs CONTROL_GAME" >&2; exit 2; }
    control_leg
fi
launch
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
read_off_boundary "$SX" 101 -12 up
T0=$TICKS
C0=$DEST
ctl wait 200 >/dev/null
read_off_boundary "$SX" 101 -12 up
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

echo "== G2: a label is reconfigured only from within reach =="
# The commands edit a label a player names by coordinates; an edit from across the
# level would let anyone reconfigure (and later take) another player's label.
info "$SX" 101 0 up
AMOUNT_BEFORE=$AMOUNT
ctl command "execute in minecraft:the_end run fill $((SX + 6)) 100 -1 $((SX + 14)) 100 1 minecraft:stone" >/dev/null
ctl wait 6 >/dev/null
ctl command "execute in minecraft:the_end run tp @s $((SX + 10)).5 101 0.5" >/dev/null
ctl wait 6 >/dev/null
label_cmd amount "$SX" 101 0 up 5
ctl wait 8 >/dev/null
OUT=$(ctl messages 16 | jq -r '.chat[]' | grep -m1 'not within reach' || true)
[ -n "$OUT" ] || fail "an out-of-reach amount edit was not refused: $(ctl messages 16 | jq -r '.chat[]' | tail -3)"
info "$SX" 101 0 up
[ "$AMOUNT" = "$AMOUNT_BEFORE" ] || fail "an out-of-reach edit changed the label ($AMOUNT_BEFORE -> $AMOUNT)"
echo "  from ten blocks away: $OUT; the label still reads amount=$AMOUNT"
stand_at 0

echo "== G3: list prints one page, not the whole level =="
label_cmd list
ctl wait 8 >/dev/null
PAGE=$(ctl messages 16 | jq -r '.chat[]' | grep -m1 'transfer labels in this level (page' || true)
[ -n "$PAGE" ] || fail "the list reply carries no page header: $(ctl messages 16 | jq -r '.chat[]' | tail -3)"
echo "  $PAGE"

echo "== I: an insert label fills the block it is stuck to from the block across the face =="
# Upstream's `ItemFilter.work` swaps source and destination for the insert item and
# nothing else: the same geometry, the same rate, the same filter. The label goes on
# the *top* face of the lower chest (the face this harness can always click), so the
# block across that face — the chest above, placed afterwards — is its source.
#
# I1 — the differential in this geometry: two chests, no label, nothing moves even
# with the source full.
pair 24
fill_at 102 24 minecraft:cobblestone 8
ctl wait 40 >/dev/null
[ "$(count_in 101 24 minecraft:cobblestone)" = 0 ] \
    || fail "the lower chest filled itself with no label in this geometry"
[ "$(count_in 102 24 minecraft:cobblestone)" = 8 ] \
    || fail "the upper chest lost items with no label in this geometry"
echo "  no label in this geometry: the lower holds 0, the upper 8"
# Clear the column (the contents spill as item entities; kill them) so the label can
# go on a bare face.
ctl command "execute in minecraft:the_end run fill $SX 101 24 $SX 102 24 minecraft:air" >/dev/null
ctl wait 6 >/dev/null
ctl command "execute in minecraft:the_end run kill @e[type=minecraft:item]" >/dev/null
ctl wait 6 >/dev/null

# I2 — the insert label: the upper chest's items drop into the labelled lower one.
attach_top 24 "$INSERT" insert
[ "$INTERVAL" = 20 ] || fail "an insert label's interval is $INTERVAL ticks, expected 20"
[ "$PER_OP" = 1 ] || fail "an insert label moves $PER_OP items an operation, expected 1"
fill_at 102 24 minecraft:cobblestone 32
ctl wait 20 >/dev/null
read_off_boundary "$SX" 101 24 up
T0=$TICKS
C0=$DEST
ctl wait 200 >/dev/null
read_off_boundary "$SX" 101 24 up
T1=$TICKS
C1=$DEST
WANT=$((T1 / 20 - T0 / 20))
[ "$((C1 - C0))" = "$WANT" ] \
    || fail "insert: $((C1 - C0)) items dropped over $((T1 - T0)) ticks, expected $WANT (one per 20 ticks)"
[ "$WANT" -ge 5 ] || fail "only $WANT activations in the window: the insert phase measured nothing"
# Conservation, and it must be exact: the source started with 32 and the sink with
# 0, so every item that left the source is on the sink (C1 counts everything that
# moved, including the activations before T0). Loss and duplication both fail here.
PAIR=$(stable_pair 101 102 24 minecraft:cobblestone) \
    || fail "the two chests never held still long enough to be read together"
SINK=$(echo "$PAIR" | cut -d' ' -f1)
SOURCE=$(echo "$PAIR" | cut -d' ' -f2)
[ "$((SINK + SOURCE))" = 32 ] \
    || fail "the two chests hold $((SINK + SOURCE)) cobblestone between them (source $SOURCE, sink $SINK), expected 32: an item was lost or duplicated"
[ "$SINK" -ge "$C1" ] || fail "the sink holds $SINK after the window, less than the $C1 the window measured"
echo "  insert: ticks $T0..$T1, $((C1 - C0)) items dropped into the labelled chest; the two chests still hold exactly 32 between them ($SOURCE + $SINK)"
if [ "$INSERT_SELFTEST" = 1 ]; then
    echo "(insert selftest: asserting the direction is reversed)"
    [ "$((C1 - C0))" = 0 ] \
        || fail "insert selftest: $((C1 - C0)) items dropped, expected the wrong direction"
fi

# I3 — the direction is pinned both ways: the same geometry and the same fill with an
# *extract* label. It pulls from the labelled (empty) lower chest into the upper one,
# so nothing moves. A port that ignored the flow would drain the upper chest here.
attach_top 22 "$LABEL" extract
fill_at 102 22 minecraft:cobblestone 8
ctl wait 60 >/dev/null
[ "$(count_in 101 22 minecraft:cobblestone)" = 0 ] \
    || fail "an extract label in this geometry filled the labelled chest"
[ "$(count_in 102 22 minecraft:cobblestone)" = 8 ] \
    || fail "an extract label in this geometry moved the upper chest's items"
echo "  extract label, same geometry, same fill: the upper chest still holds 8 and the lower 0"

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
# The insert label's flow is part of the saved record: it must come back as insert,
# not as the default extract label.
info "$SX" 101 24 up
[ "$FLOW" = insert ] \
    || fail "the insert label came back as flow=$FLOW after the relaunch"
echo "  the insert label came back as flow=insert"

echo "== J: every carried recipe crafts in a real crafting table =="
# The two shaped grids and the two shapeless conversions, crafted the way they
# declare, with the output id and count and the ingredient consumption asserted; a
# grid with one wrong key makes nothing. Survival, or a creative player would take
# the result without consuming the grid. `NEGATIVE=1` flips the first count.
ctl command "gamemode survival @a" >/dev/null
ctl wait 6 >/dev/null
ctl command "execute in minecraft:the_end run setblock $X0 101 24 minecraft:crafting_table" >/dev/null
ctl wait 6 >/dev/null
[ "$(block_name "$X0" 101 24)" = "minecraft:crafting_table" ] || fail "no crafting table at $X0,101,24"
python3 "$PORT/tools/craft_labels.py" "$SOCK" "$CTL" "$X0" 101 24 \
    || fail "a carried recipe did not craft as it declares"
echo "  recipes PASS"

echo "PASS: an extract label on a block face moves exactly one item per 20 game ticks into the container across the face, an insert label on the same geometry fills the block it is stuck to at the same rate and loses nothing, a whitelist that does not match moves nothing while a matching one moves, speed 38 gives one item a tick, amount 63 moves 64 in one operation, a regulating entry of 10 never moves more than four at once and stops at 10, a removed label returns its own item without breaking the block, both labels survive a save and a relaunch with their flow, and every carried recipe (extract, insert and the two conversions) crafts in a table with its ingredients consumed while a wrong grid makes nothing"
