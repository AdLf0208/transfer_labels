#!/usr/bin/env python3
"""Turn `/transfer_labels info …`'s reply into shell assignments.

The mod prints one line, `label up at 202,101,0 mode=blacklist type=normal
amount=0 speed=0 interval=20 per_op=1 entries=0 ticks=1234`; this script writes
`FACE=…`, `X=…`, `Y=…`, `Z=…`, `MODE=…`, `TYPE=…`, `AMOUNT=…`, `SPEED=…`,
`INTERVAL=…`, `PER_OP=…`, `ENTRIES=…`, `TICKS=…`, `DEST=…`, and `ok=1` when
the line parsed. An unparsable line gives `ok=0`, so an e2e can refuse it instead of
reading empty variables.
"""
import re
import shlex
import sys

LINE = re.compile(
    r"^label (?P<face>\w+) at (?P<x>-?\d+),(?P<y>-?\d+),(?P<z>-?\d+) "
    r"mode=(?P<mode>\w+) type=(?P<kind>\w+) amount=(?P<amount>\d+) "
    r"speed=(?P<speed>\d+) interval=(?P<interval>\d+) per_op=(?P<per_op>\d+) "
    r"entries=(?P<entries>\d+)(?P<rest>.*)$"
)


def main() -> int:
    text = " ".join(sys.argv[1:]).strip()
    match = LINE.match(text)
    if match is None:
        print("ok=0")
        return 0
    fields = match.groupdict()
    print("ok=1")
    print("FACE=%s" % shlex.quote(fields["face"]))
    for name, key in (("X", "x"), ("Y", "y"), ("Z", "z")):
        print("%s=%s" % (name, fields[key]))
    for key in ("mode", "amount", "speed", "interval", "per_op", "entries"):
        print("%s=%s" % (key.upper(), fields[key]))
    print("TYPE=%s" % shlex.quote(fields["kind"]))
    for name in ("ticks", "dest"):
        found = re.search(r"%s=(-?\d+)" % name, fields["rest"] or "")
        print("%s=%s" % (name.upper(), found.group(1) if found else -1))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
