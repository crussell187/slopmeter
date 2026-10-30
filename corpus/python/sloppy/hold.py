def hold(flag):
    if flag:
        return 1
    if flag:
        return 2
    try:
        flag = flag
    except Exception:
        pass
    return 0
