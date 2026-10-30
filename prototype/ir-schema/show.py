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
    lines = header_lines(doc, file_, fn, file_index, fn_index)
    append_body(lines, derived)
    append_coverage(lines, derived)
    append_footer(lines, fn, file_)
    return "\n".join(lines)


def header_lines(doc, file_, fn, file_index, fn_index):
    files = doc["files"]
    functions = file_["functions"]
    return [
        f"{BOLD}IR{RESET} {DIM}version {doc['version']}{RESET}",
        (
            f"{BOLD}file{RESET} {file_index + 1}/{len(files)}  "
            f"{file_['path']}  {DIM}{file_['language']}  "
            f"unparsed_ratio {file_['unparsed_ratio']}{RESET}"
        ),
        (
            f"{BOLD}function{RESET} {fn_index + 1}/{len(functions)}  "
            f"{fn['name']}"
        ),
        "",
        f"{BOLD}derived{RESET}",
    ]


def append_body(lines, derived):
    if derived["encoded"]:
        append_encoded(lines, derived)
        return
    lines.append(f"  {DIM}identity and span only; body not encoded{RESET}")


def append_encoded(lines, derived):
    lines.append(
        f"  cc {derived['cc']}   nesting {derived['nesting']}   "
        f"lloc {derived['lloc']}"
    )
    append_forward(lines, derived["forward"])
    interesting = [ch for ch in derived["chains"] if ch["depth"] > 0]
    append_chains(lines, interesting)


def append_forward(lines, forward):
    if forward is None:
        lines.append(f"  forward {DIM}none{RESET}")
        return
    ratio = forward["ratio"]
    shown = "n/a" if ratio is None else f"{ratio:.2f}"
    lines.append(
        f"  forward line {forward['line']}  "
        f"{', '.join(forward['forwarded'])}  ratio {shown}"
    )


def append_chains(lines, interesting):
    if not interesting:
        lines.append(f"  chains {DIM}none over depth 0{RESET}")
        return
    for ch in interesting:
        append_chain(lines, ch)


def append_chain(lines, ch):
    root = "receiver" if ch["rooted"] else "other"
    lines.append(
        f"  chain line {ch['line']}  depth {ch['depth']}  "
        f"{root}  {ch['text']}"
    )


def append_coverage(lines, derived):
    lines.append(
        f"  coverage {DIM}{derived['coverage']}{RESET}  "
        f"join {derived['join_span']['start']}-{derived['join_span']['end']}"
    )


def append_footer(lines, fn, file_):
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

    stream = sys.stdin
    fd = stream.fileno()
    old = termios.tcgetattr(fd)
    try:
        tty.setraw(fd)
        char = stream.read(1)
    finally:
        termios.tcsetattr(fd, termios.TCSADRAIN, old)
    return char


def write_out(text):
    stream = sys.stdout
    stream.write(text)


def flush_out():
    stream = sys.stdout
    stream.flush()


def stdin_is_tty():
    stream = sys.stdin
    return stream.isatty()


def dump_requested():
    args = sys.argv
    if "--dump" in args:
        return True
    return not stdin_is_tty()


def main():
    doc = document()
    if dump_requested():
        dump(doc)
        return
    browse(doc)


def browse(doc):
    file_index = 0
    first = doc["files"][0]
    fn_index = start_index(first["path"], first["functions"])
    while True:
        paint(doc, file_index, fn_index)
        step = apply_key(doc, file_index, fn_index, read_key())
        if step is None:
            return
        file_index, fn_index = step


def paint(doc, file_index, fn_index):
    write_out("\033[2J\033[H")
    write_out(frame(doc, file_index, fn_index))
    write_out("\n")
    flush_out()


def apply_key(doc, file_index, fn_index, char):
    file_ = doc["files"][file_index]
    count = len(file_["functions"])
    if char in ("q", "\x03"):
        write_out("\033[2J\033[H")
        return None
    if char == "n":
        return next_file(doc, file_index, 1)
    if char == "p":
        return next_file(doc, file_index, -1)
    if char == "j":
        return file_index, (fn_index + 1) % count
    if char == "k":
        return file_index, (fn_index - 1) % count
    return file_index, fn_index


def next_file(doc, file_index, delta):
    files = doc["files"]
    file_index = (file_index + delta) % len(files)
    chosen = files[file_index]
    return file_index, start_index(chosen["path"], chosen["functions"])


if __name__ == "__main__":
    main()
