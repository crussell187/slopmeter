def plain():
    return 1

def decision(flag):
    if flag:
        return 1
    return 0

def elif_arm(flag, other):
    if flag:
        return 1
    elif other:
        return 2
    else:
        return 0

def boolean(a, b, c):
    if a and b and c:
        return 1
    return 0

def loop(items):
    for item in items:
        return item
    return 0

def handler(path):
    try:
        return path
    except OSError:
        return 0

def ternary(flag):
    return 1 if flag else 0

def comprehension(xs):
    return [x for x in xs if x]

def loop_else(items):
    for item in items:
        return item
    else:
        return 0

def final(path):
    try:
        return path
    finally:
        path = None

def with_block(path):
    with path:
        return path

def or_pattern(value):
    match value:
        case 1 | 2 | 3:
            return 1
        case _:
            return 0
