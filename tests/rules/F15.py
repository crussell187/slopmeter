def load(path):
    try:
        text = path.read()
    except OSError:
        pass
