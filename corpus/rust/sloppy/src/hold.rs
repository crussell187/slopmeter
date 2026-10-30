pub fn hold(flag: bool) -> i32 {
    if flag {
        return 1;
    }
    if flag {
        return 2;
    }
    let _ = notify();
    0
}

pub fn read(path: &str) {
    let _file = File::open(path);
}
