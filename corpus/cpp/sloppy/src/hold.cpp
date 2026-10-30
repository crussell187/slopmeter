int hold(bool flag) {
    if (flag) {
        return 1;
    }
    if (flag) {
        return 2;
    }
    try {
        (void)flag;
    } catch (...) {
    }
    return 0;
}

void read(const char* path) {
    fopen(path, "r");
}

void fill(int& n) {
    n += 1;
}
