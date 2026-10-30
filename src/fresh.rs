use std::time::SystemTime;

pub struct Stamped {
    pub path: String,
    pub time: Option<SystemTime>,
}

pub fn newest_stale(report: SystemTime, paths: &[Stamped]) -> Option<String> {
    let timed = newest_timed(report, paths);
    if timed.is_some() {
        return timed;
    }
    newest_gone(paths)
}

pub fn older_than(report: &str, path: &str) -> String {
    format!("{report} is older than {path}")
}

fn newest_timed(report: SystemTime, paths: &[Stamped]) -> Option<String> {
    let mut best: Option<(SystemTime, String)> = None;
    for path in paths {
        best = consider(best, report, path);
    }
    best.map(|pair| pair.1)
}

fn consider(
    best: Option<(SystemTime, String)>,
    report: SystemTime,
    path: &Stamped,
) -> Option<(SystemTime, String)> {
    let Some(time) = path.time else {
        return best;
    };
    if time <= report {
        return best;
    }
    Some(prefer(best, time, &path.path))
}

fn prefer(
    best: Option<(SystemTime, String)>,
    time: SystemTime,
    path: &str,
) -> (SystemTime, String) {
    let Some((have, name)) = best else {
        return (time, path.to_string());
    };
    if time > have || (time == have && path > name.as_str()) {
        return (time, path.to_string());
    }
    (have, name)
}

fn newest_gone(paths: &[Stamped]) -> Option<String> {
    let mut name: Option<String> = None;
    for path in paths {
        name = gone_name(name, path);
    }
    name
}

fn gone_name(have: Option<String>, path: &Stamped) -> Option<String> {
    let time = &path.time;
    if time.is_some() {
        return have;
    }
    let name = &path.path;
    later_name(have, name)
}

fn later_name(have: Option<String>, path: &str) -> Option<String> {
    match have {
        Some(have) if have.as_str() >= path => Some(have),
        _ => Some(path.to_string()),
    }
}
