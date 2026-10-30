"""PROTOTYPE — wipe me. Not the crate.

Question: can Phase 1 score one real file per language from a language-neutral
list of functions, without module edges, classes, churn, or coverage on the IR?

The adapter classifies. This document stores that classification. It does not
store a threshold judgment, a coverage fraction, or an AST.
"""

ALLOW = {
    "python": {
        "get", "keys", "values", "items", "append", "extend", "add", "update",
        "pop", "split", "rsplit", "strip", "lstrip", "rstrip", "join", "lower",
        "upper", "startswith", "endswith", "encode", "decode", "format", "copy",
    },
    "rust": {
        "unwrap", "expect", "unwrap_or", "unwrap_or_else", "unwrap_or_default",
        "as_ref", "as_mut", "as_deref", "as_deref_mut", "ok", "err", "cloned",
        "copied", "iter", "iter_mut", "into_iter", "map", "filter", "filter_map",
        "flat_map", "flatten", "enumerate", "zip", "take", "skip", "chain",
        "collect", "rev", "inspect", "clone", "to_owned", "to_string", "to_vec",
        "as_str", "as_bytes", "into", "get", "get_mut", "first", "last", "front",
        "back", "entry", "keys", "values", "lock", "read", "write", "borrow",
        "borrow_mut",
    },
    "cpp": {
        "begin", "end", "cbegin", "cend", "rbegin", "rend", "crbegin", "crend",
        "size", "empty", "at", "front", "back", "data", "find", "c_str", "value",
        "value_or", "has_value",
    },
}

NOTES = {
    "src/requests/api.py": (
        "A docstring sits inside the span and is not a statement. "
        "No decorator, so the touch span equals the body span. "
        "head has two statements, so it is not a forwarding call."
    ),
    "src/buf/take.rs": (
        "A /// doc comment is not part of the touch span. "
        "The #[cfg] on chunks_vectored is. "
        "Declarations are absent; every fn here has a body."
    ),
    "include/CLI/StringTools.hpp": (
        "split, ltrim, rtrim, and the other semicolon declarations are not functions. "
        "They are omitted, not listed one by one. "
        "join is one function for the template that is written here. "
        "trim returns ltrim(rtrim(str)), which is not a forwarding call, "
        "and the record stores that only as forward: null."
    ),
}


def param(name, receiver=False):
    return {"name": name, "receiver": receiver}


def span(start, end):
    return {"start": start, "end": end}


def hop(name, call):
    return {"name": name, "call": call}


def chain(line, rooted, hops):
    return {"line": line, "rooted": rooted, "members": hops}


def function(name, start, end, params, param_types=None, touch=None, body=None):
    record = {
        "name": name,
        "param_types": param_types or [],
        "span": span(start, end),
        "touch": span(*(touch or (start, end))),
        "params": params,
    }
    if body is not None:
        record["body"] = body
    return record


def body(lloc, decisions=None, chains=None, forward=None):
    return {
        "lloc": lloc,
        "decisions": decisions or [],
        "chains": chains or [],
        "forward": forward,
    }


def decision(line, depth):
    return {"line": line, "depth": depth}


def document():
    return {"version": 1, "files": FILES}


def ordinary_names(fn):
    return [p["name"] for p in fn["params"] if not p["receiver"]]


def chain_depth(members, language):
    hops = list(members)
    if hops and hops[-1]["call"] and hops[-1]["name"] == "build":
        while hops and hops[-1]["call"]:
            hops.pop()
    allow = ALLOW[language]
    return sum(1 for hop_ in hops if hop_["name"] not in allow)


def chain_text(ch):
    parts = []
    for hop_ in ch["members"]:
        parts.append(hop_["name"] + ("()" if hop_["call"] else ""))
    return ".".join(parts)


def derive(fn, language):
    encoded = "body" in fn
    result = {
        "encoded": encoded,
        "ordinary": ordinary_names(fn),
        "coverage": "absent",
        "join_span": fn["span"],
    }
    if not encoded:
        return result
    fill_metrics(result, fn, language)
    return result


def fill_metrics(result, fn, language):
    body = fn["body"]
    decisions = body["decisions"]
    result["cc"] = 1 + len(decisions)
    result["nesting"] = max((d["depth"] for d in decisions), default=0)
    result["lloc"] = body["lloc"]
    fill_forward(result, body["forward"])
    result["chains"] = ranked_chains(body["chains"], language)


def fill_forward(result, forward):
    if forward is None:
        result["forward"] = None
        return
    ordinary = result["ordinary"]
    names = forward["names"]
    result["forward"] = forward_view(forward, ordinary, names)
    unknown = unknown_forward(names, ordinary)
    if not unknown:
        return
    result["forward_names_not_ordinary"] = unknown


def unknown_forward(names, ordinary):
    return [n for n in names if n not in ordinary]


def forward_view(forward, ordinary, names):
    return {
        "line": forward["line"],
        "forwarded": names,
        "ratio": forward_ratio(names, ordinary),
    }


def forward_ratio(names, ordinary):
    if not ordinary:
        return None
    return len(names) / len(ordinary)


def ranked_chains(chains, language):
    ranked = []
    for ch in chains:
        ranked.append(chain_view(ch, language))
    ranked.sort(key=chain_sort_key)
    return ranked


def chain_view(ch, language):
    return {
        "line": ch["line"],
        "rooted": ch["rooted"],
        "depth": chain_depth(ch["members"], language),
        "text": chain_text(ch),
    }


def chain_sort_key(item):
    return (-item["depth"], item["line"])


FILES = [
    {
        "path": "src/requests/api.py",
        "language": "python",
        "unparsed_ratio": 0.0,
        "functions": [
            function(
                "request",
                24,
                71,
                [param("method"), param("url"), param("kwargs")],
                body=body(
                    2,
                    chains=[
                        chain(71, False, [hop("request", True)]),
                    ],
                ),
            ),
            function(
                "get",
                74,
                87,
                [param("url"), param("params"), param("kwargs")],
                body=body(
                    1,
                    forward={"line": 87, "names": ["url", "params", "kwargs"]},
                ),
            ),
            function(
                "options",
                90,
                99,
                [param("url"), param("kwargs")],
                body=body(1, forward={"line": 99, "names": ["url", "kwargs"]}),
            ),
            function(
                "head",
                102,
                114,
                [param("url"), param("kwargs")],
                body=body(
                    2,
                    chains=[chain(113, False, [hop("setdefault", True)])],
                ),
            ),
            function(
                "post",
                117,
                134,
                [param("url"), param("data"), param("json"), param("kwargs")],
                body=body(
                    1,
                    forward={
                        "line": 134,
                        "names": ["url", "data", "json", "kwargs"],
                    },
                ),
            ),
            function(
                "put",
                137,
                151,
                [param("url"), param("data"), param("kwargs")],
                body=body(
                    1,
                    forward={"line": 151, "names": ["url", "data", "kwargs"]},
                ),
            ),
            function(
                "patch",
                154,
                168,
                [param("url"), param("data"), param("kwargs")],
                body=body(
                    1,
                    forward={"line": 168, "names": ["url", "data", "kwargs"]},
                ),
            ),
            function(
                "delete",
                171,
                180,
                [param("url"), param("kwargs")],
                body=body(1, forward={"line": 180, "names": ["url", "kwargs"]}),
            ),
        ],
    },
    {
        "path": "src/buf/take.rs",
        "language": "rust",
        "unparsed_ratio": 0.0,
        "functions": [
            function("new", 18, 20, [param("inner"), param("limit")]),
            function(
                "Take::into_inner",
                42,
                44,
                [param("self", receiver=True)],
                body=body(
                    1,
                    chains=[chain(43, True, [hop("inner", False)])],
                ),
            ),
            function("Take::get_ref", 59, 61, [param("self", receiver=True)]),
            function("Take::get_mut", 80, 82, [param("self", receiver=True)]),
            function("Take::limit", 102, 104, [param("self", receiver=True)]),
            function(
                "Take::set_limit",
                130,
                132,
                [param("self", receiver=True), param("lim")],
            ),
            function(
                "<Take<T> as Buf>::remaining",
                136,
                138,
                [param("self", receiver=True)],
            ),
            function(
                "<Take<T> as Buf>::chunk",
                140,
                143,
                [param("self", receiver=True)],
            ),
            function(
                "<Take<T> as Buf>::advance",
                145,
                149,
                [param("self", receiver=True), param("cnt")],
            ),
            function(
                "<Take<T> as Buf>::copy_to_bytes",
                151,
                157,
                [param("self", receiver=True), param("len")],
            ),
            function(
                "<Take<T> as Buf>::chunks_vectored",
                160,
                186,
                [param("self", receiver=True), param("dst")],
                touch=(159, 186),
                body=body(
                    15,
                    decisions=[
                        decision(161, 1),
                        decision(172, 1),
                        decision(173, 2),
                    ],
                    chains=[
                        chain(161, True, [hop("limit", False)]),
                        chain(
                            168,
                            True,
                            [hop("inner", False), hop("chunks_vectored", True)],
                        ),
                        chain(
                            170,
                            False,
                            [hop("len", True), hop("min", True)],
                        ),
                        chain(171, True, [hop("limit", False)]),
                        chain(
                            172,
                            False,
                            [
                                hop("iter_mut", True),
                                hop("zip", True),
                                hop("enumerate", True),
                            ],
                        ),
                        chain(172, False, [hop("iter", True)]),
                        chain(173, False, [hop("get", True)]),
                        chain(181, False, [hop("len", True)]),
                    ],
                ),
            ),
        ],
    },
    {
        "path": "include/CLI/StringTools.hpp",
        "language": "cpp",
        "unparsed_ratio": 0.0,
        "functions": [
            function(
                "CLI::enums::operator<<",
                33,
                38,
                [param("in"), param("item")],
                param_types=["std::ostream &", "const T &"],
                body=body(1),
            ),
            function(
                "CLI::detail::join",
                54,
                69,
                [param("v"), param("delim")],
                param_types=["const T &", "std::string"],
                body=body(
                    10,
                    decisions=[
                        decision(58, 1),
                        decision(60, 1),
                        decision(64, 1),
                        decision(64, 1),
                        decision(64, 1),
                    ],
                    chains=[
                        chain(63, False, [hop("str", True)]),
                        chain(64, False, [hop("empty", True)]),
                        chain(64, False, [hop("size", True)]),
                        chain(64, False, [hop("back", True)]),
                        chain(65, False, [hop("pop_back", True)]),
                    ],
                ),
            ),
            function(
                "CLI::detail::trim",
                117,
                117,
                [param("str")],
                param_types=["std::string &"],
                body=body(1),
            ),
            function(
                "CLI::detail::trim",
                120,
                120,
                [param("str"), param("filter")],
                param_types=["std::string &", "const std::string &"],
                body=body(1),
            ),
            function(
                "CLI::detail::valid_first_char",
                154,
                156,
                [param("c")],
                param_types=["T"],
                body=body(1, decisions=[decision(155, 1)]),
            ),
        ],
    },
]
