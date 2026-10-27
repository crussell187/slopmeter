#!/usr/bin/env python3
"""PROTOTYPE — wipe me. Thin view over model.document. Not the crate.

Run: python3 prototype/ir-schema/show.py
"""

import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from model import NOTES, derive, document

START = {
    "src/requests/api.py": "get",
    "src/buf/take.rs": "<Take<T> as Buf>::chunks_vectored",
    "include/CLI/StringTools.hpp": "CLI::detail::join",
}

BOLD = "\x1b[1m"
DIM = "\x1b[2m"
RESET = "\x1b[0m"


def start_index(path, functions):
    name = START[path]
    for index, fn in enumerate(functions):
        if fn["name"] == name:
            return index
    return 0


def frame(doc, file_index, fn_index):
    file_ = doc["files"][file_index]
    fn = file_["functions"][fn_index]
    derived = derive(fn, file_["language"])
    lines = [
        f"{BOLD}IR{RESET} {DIM}version {doc['version']}{RESET}",
        (
            f"{BOLD}file{RESET} {file_index + 1}/{len(doc['files'])}  "
            f"{file_['path']}  {DIM}{file_['language']}  "
            f"unparsed_ratio {file_['unparsed_ratio']}{RESET}"
        ),
        (
            f"{BOLD}function{RESET} {fn_index + 1}/{len(file_['functions'])}  "
            f"{fn['name']}"
        ),
        "",
        f"{BOLD}derived{RESET}",
    ]
    if not derived["encoded"]:
        lines.append(f"  {DIM}identity and span only; body not encoded{RESET}")
    else:
        lines.append(
            f"  cc {derived['cc']}   nesting {derived['nesting']}   "
            f"lloc {derived['lloc']}"
        )
        forward = derived["forward"]
        if forward is None:
            lines.append(f"  forward {DIM}none{RESET}")
        else:
            ratio = forward["ratio"]
            shown = "n/a" if ratio is None else f"{ratio:.2f}"
            lines.append(
                f"  forward line {forward['line']}  "
                f"{', '.join(forward['forwarded'])}  ratio {shown}"
            )
        interesting = [ch for ch in derived["chains"] if ch["depth"] > 0]
        if not interesting:
            lines.append(f"  chains {DIM}none over depth 0{RESET}")
        for ch in interesting:
            root = "receiver" if ch["rooted"] else "other"
            lines.append(
                f"  chain line {ch['line']}  depth {ch['depth']}  "
                f"{root}  {ch['text']}"
            )
    lines.append(
        f"  coverage {DIM}{derived['coverage']}{RESET}  "
        f"join {derived['join_span']['start']}-{derived['join_span']['end']}"
    )
    lines += [
        "",
        f"{BOLD}record{RESET}",
        json.dumps(fn, indent=2),
        "",
        f"{DIM}{NOTES[file_['path']]}{RESET}",
        f"{DIM}not stored: exports, imports, classes, call edges, churn, "
        f"coverage, cognitive complexity{RESET}",
        "",
        f"{BOLD}n{RESET} {DIM}next file{RESET}  "
        f"{BOLD}p{RESET} {DIM}prev file{RESET}  "
        f"{BOLD}j{RESET} {DIM}next function{RESET}  "
        f"{BOLD}k{RESET} {DIM}prev function{RESET}  "
        f"{BOLD}q{RESET} {DIM}quit{RESET}",
    ]
    return "\n".join(lines)


def dump(doc):
    for file_ in doc["files"]:
        print(f"== {file_['path']}")
        for fn in file_["functions"]:
            derived = derive(fn, file_["language"])
            if not derived["encoded"]:
                print(f"  {fn['name']}  identity")
                continue
            depths = [ch["depth"] for ch in derived["chains"]]
            worst = max(depths) if depths else 0
            forward = derived["forward"]
            ratio = None if forward is None else forward["ratio"]
            print(
                f"  {fn['name']}  cc {derived['cc']}  "
                f"nesting {derived['nesting']}  lloc {derived['lloc']}  "
                f"forward {ratio}  worst chain {worst}"
            )


def read_key():
    import termios
    import tty

    fd = sys.stdin.fileno()
    old = termios.tcgetattr(fd)
    try:
        tty.setraw(fd)
        char = sys.stdin.read(1)
    finally:
        termios.tcsetattr(fd, termios.TCSADRAIN, old)
    return char


def main():
    doc = document()
    if "--dump" in sys.argv or not sys.stdin.isatty():
        dump(doc)
        return
    file_index = 0
    fn_index = start_index(
        doc["files"][0]["path"], doc["files"][0]["functions"]
    )
    while True:
        sys.stdout.write("\033[2J\033[H")
        sys.stdout.write(frame(doc, file_index, fn_index))
        sys.stdout.write("\n")
        sys.stdout.flush()
        char = read_key()
        file_ = doc["files"][file_index]
        count = len(file_["functions"])
        if char in ("q", "\x03"):
            sys.stdout.write("\033[2J\033[H")
            return
        if char == "n":
            file_index = (file_index + 1) % len(doc["files"])
            fn_index = start_index(
                doc["files"][file_index]["path"],
                doc["files"][file_index]["functions"],
            )
        elif char == "p":
            file_index = (file_index - 1) % len(doc["files"])
            fn_index = start_index(
                doc["files"][file_index]["path"],
                doc["files"][file_index]["functions"],
            )
        elif char == "j":
            fn_index = (fn_index + 1) % count
        elif char == "k":
            fn_index = (fn_index - 1) % count


if __name__ == "__main__":
    main()
