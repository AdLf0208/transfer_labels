#!/usr/bin/env python3
"""Craft every carried transfer-label recipe in a real crafting table.

Called by `tools/e2e.sh` with the open game's control socket. The harness opens a
crafting table as a screen whose 3x3 grid is slots 100..108 (row-major) and whose
result is 200; the player's own slots keep their inventory numbers (hotbar 0..8).
The player must already be in survival: a creative player takes a result without
consuming the grid, which would make the consumption assertion vacuous.

What it proves, in this order:

1. the carried **extract** recipe (`" R " / " C " / " H "`, redstone over a wooden
   chest over a hopper) makes **2** `transfer_labels:itemstack_extract_transfer_label`
   and its three ingredients are consumed;
2. the carried **insert** recipe (`" H " / " C " / " R "`, the same three keys with
   hopper and redstone swapped) makes **2**
   `transfer_labels:itemstack_insert_transfer_label`, ingredients consumed — so the
   two grids are two recipes, not one shadowing the other;
3. the shapeless extract → insert conversion makes one insert label from one extract
   label, consumption asserted;
4. the shapeless insert → extract conversion makes one extract label back;
5. the *control*: the extract grid with an iron ingot where the hopper belongs makes
   nothing, so the recipe really reads its keys instead of matching any column.

Exit 1 with the observed result on the first mismatch. `NEGATIVE=1` flips step 1's
count assertion (it expects the wrong number), so the run MUST exit 1.
"""
from __future__ import annotations

import json
import os
import subprocess
import sys

NEGATIVE = os.environ.get("NEGATIVE") == "1"

# The harness's crafting screen: the table's grid and its result.
GRID = list(range(100, 109))
RESULT = 200
TOP, MIDDLE, BOTTOM = 101, 104, 107
CENTRE = 104
# The crafted stack goes into the first hotbar slot, where `select 1` holds it.
HOTBAR = 0

EXTRACT = "transfer_labels:itemstack_extract_transfer_label"
INSERT = "transfer_labels:itemstack_insert_transfer_label"
CHEST = "minecraft:chest"
HOPPER = "minecraft:hopper"
REDSTONE = "minecraft:redstone"


def main() -> int:
    if len(sys.argv) < 6:
        raise SystemExit("usage: craft_labels.py <socket> <ctl> <x> <y> <z>")
    sock, ctl, x, y, z = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4], sys.argv[5]

    def run(*args: str) -> dict:
        out = subprocess.run([ctl, "--socket", sock, *args], text=True, capture_output=True)
        try:
            return json.loads(out.stdout)
        except json.JSONDecodeError:
            raise SystemExit(f"ctl {' '.join(args)}: {out.stdout}{out.stderr}")

    def wait(ticks: int = 4) -> None:
        run("wait", str(ticks))

    def open_table() -> None:
        """Open the crafting table, retrying the click if the menu did not come up.

        `ctl interact` aims at the block from the eye; after a previous menu was
        closed the click sometimes lands while the client is still settling, and a
        run that silently carried on without the grid would assert nothing.
        """
        for attempt in range(1, 6):
            try:
                run("look-at-block", x, y, z, "up")
            except SystemExit:
                pass
            wait(2)
            reply = run("interact", x, y, z)
            wait(4)
            if slots_raw():
                print(f"  table open (attempt {attempt}, {reply.get('screen')})")
                return
        raise SystemExit(f"FAIL: the crafting table at {x},{y},{z} never opened a menu")

    def slots_raw() -> list[dict]:
        screen = run("screen")
        return screen.get("slots") or []

    def slots() -> list[dict]:
        found = slots_raw()
        if not found:
            raise SystemExit(f"no menu is open: {run('screen')}")
        return found

    def item_of(slot: int) -> str | None:
        found = [s for s in slots() if s["slot"] == slot]
        return (found[0].get("item") if found else None) or None

    def count_of(item: str) -> int:
        return run("inventory")["counts"].get(item, 0)

    def find(item: str) -> int:
        for slot in slots():
            if slot.get("item") == item:
                return slot["slot"]
        raise SystemExit(f"missing ingredient {item} (inventory: {run('inventory')['counts']})")

    def result() -> dict | None:
        found = [s for s in slots() if s["slot"] == RESULT]
        if not found or not found[0].get("item"):
            return None
        return found[0]

    def place(item: str, targets: list[int]) -> None:
        """Pick the item up, put one in each target, put the rest back."""
        slot = find(item)
        run("click-slot", str(slot))
        wait(1)
        for target in targets:
            run("click-slot", str(target), "right")
            wait(1)
        run("click-slot", str(slot))
        wait(2)

    def setup(items: list[tuple[str, int]]) -> None:
        """A fresh table, a cleared inventory and exactly `items` in it."""
        try:
            run("close")
        except SystemExit:
            pass
        wait(5)
        run("setblock", x, y, z, "minecraft:air")
        run("setblock", x, y, z, "minecraft:crafting_table")
        wait(2)
        run("command", f"tp @s {float(x) - 0.5} {y} {float(z) + 0.5}")
        wait(3)
        print(f"  clear -> {json.dumps(run('command', 'clear @s'))}")
        for item, count in items:
            reply = run("give", item, str(count))
            print(f"  give {count} {item} -> {json.dumps(reply)}")
        wait(4)
        for item, count in items:
            got = count_of(item)
            if got < count:
                raise SystemExit(
                    f"FAIL: the setup gave {count} {item} but the player holds {got} "
                    f"(inventory {json.dumps(run('inventory')['counts'])})"
                )
        # Hold nothing: a label item in the hand would be attached to the crafting
        # table by the mod's own right-click hook when `interact` opens it, and the
        # ingredient would be consumed before the grid saw it (measured).
        run("select", "9")
        wait(2)
        open_table()

    def expect_result(want_item: str | None, count: int, label: str) -> None:
        got = result()
        print(f"  {label} -> {json.dumps(got)}")
        if want_item is None:
            if got is not None:
                raise SystemExit(f"FAIL: {label}: expected no result, got {got}")
            return
        if not got or got.get("item") != want_item or got.get("count") != count:
            grid = [s for s in slots() if s["slot"] in GRID]
            raise SystemExit(
                f"FAIL: {label}: expected {count} {want_item}, got {got}; grid {json.dumps(grid)}"
            )

    def take_into_hotbar() -> None:
        """Take the result into the first hotbar slot; the take consumes the grid."""
        run("click-slot", str(RESULT))
        wait(2)
        run("click-slot", str(HOTBAR))
        wait(2)

    def consumed(items: list[str], label: str) -> None:
        for item in items:
            left = count_of(item)
            if left != 0:
                raise SystemExit(f"FAIL: {label}: {left} of {item} left after the craft")
        grid = [s for s in slots() if s["slot"] in GRID and s.get("item")]
        if grid:
            raise SystemExit(f"FAIL: {label}: the grid still holds {grid}")

    def held(item: str, count: int, label: str) -> None:
        got = count_of(item)
        if got != count:
            raise SystemExit(f"FAIL: {label}: the player holds {got} {item}, expected {count}")

    # ---------------------------------------------------------------- shaped extract
    print("== the carried extract recipe crafts 2, ingredients consumed ==")
    setup([(HOPPER, 1), (CHEST, 1), (REDSTONE, 1)])
    place(REDSTONE, [TOP])
    place(CHEST, [MIDDLE])
    place(HOPPER, [BOTTOM])
    want = 1 if NEGATIVE else 2
    if NEGATIVE:
        print("  (negative: expecting the wrong count, the run must fail)")
    expect_result(EXTRACT, want, "extract: redstone / chest / hopper")
    take_into_hotbar()
    consumed([HOPPER, CHEST, REDSTONE], "extract")
    held(EXTRACT, 2, "extract")
    print("  extract: 2 crafted, hopper + chest + redstone consumed")

    # ---------------------------------------------------------------- shaped insert
    print("== the carried insert recipe crafts 2 from the mirrored grid ==")
    setup([(HOPPER, 1), (CHEST, 1), (REDSTONE, 1)])
    place(HOPPER, [TOP])
    place(CHEST, [MIDDLE])
    place(REDSTONE, [BOTTOM])
    expect_result(INSERT, 2, "insert: hopper / chest / redstone")
    take_into_hotbar()
    consumed([HOPPER, CHEST, REDSTONE], "insert")
    held(INSERT, 2, "insert")
    print("  insert: 2 crafted, hopper + chest + redstone consumed")

    # ---------------------------------------------------------------- conversions
    print("== the shapeless extract -> insert conversion crafts 1 ==")
    setup([(EXTRACT, 1)])
    place(EXTRACT, [CENTRE])
    expect_result(INSERT, 1, "extract -> insert")
    take_into_hotbar()
    consumed([EXTRACT], "extract -> insert")
    held(INSERT, 1, "extract -> insert")
    print("  extract -> insert: 1, the extract label consumed")

    print("== the shapeless insert -> extract conversion crafts 1 ==")
    setup([(INSERT, 1)])
    place(INSERT, [CENTRE])
    expect_result(EXTRACT, 1, "insert -> extract")
    take_into_hotbar()
    consumed([INSERT], "insert -> extract")
    held(EXTRACT, 1, "insert -> extract")
    print("  insert -> extract: 1, the insert label consumed")

    # ---------------------------------------------------------------- control
    print("== control: the extract grid with an iron ingot makes nothing ==")
    setup([("minecraft:iron_ingot", 1), (CHEST, 1), (REDSTONE, 1)])
    place(REDSTONE, [TOP])
    place(CHEST, [MIDDLE])
    place("minecraft:iron_ingot", [BOTTOM])
    expect_result(None, 0, "control: iron ingot where the hopper belongs")
    print("  the recipe reads its keys: an iron ingot in the hopper slot makes nothing")

    run("close")
    wait(3)
    print("RECIPES PASS: 4 carried recipes crafted the way they declare, the control made nothing")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
