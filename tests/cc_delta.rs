use slopmeter::adapters::python::parse;

struct Row {
    name: &'static str,
    slopmeter: u32,
    radon: u32,
    delta: i32,
}

fn recorded() -> Vec<Row> {
    let text = include_str!("cc/delta.txt");
    text.lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| {
            let mut columns = line.split_whitespace();
            let name = columns.next().unwrap();
            let slopmeter = columns.next().unwrap().parse().unwrap();
            let radon = columns.next().unwrap().parse().unwrap();
            let delta = columns.next().unwrap().parse().unwrap();
            Row {
                name,
                slopmeter,
                radon,
                delta,
            }
        })
        .collect()
}

#[test]
fn the_recorded_delta_matches_the_parser_and_only_the_or_pattern_differs() {
    let source = include_str!("cc/decisions.py");
    let file = parse("tests/cc/decisions.py", source.as_bytes());
    let rows = recorded();
    assert_eq!(file.functions.len(), rows.len());
    for row in &rows {
        let function = file
            .functions
            .iter()
            .find(|function| function.name == row.name)
            .unwrap_or_else(|| panic!("missing {}", row.name));
        let cyclomatic = 1 + function.body.decisions.len() as u32;
        assert_eq!(cyclomatic, row.slopmeter, "{}", row.name);
        assert_eq!(
            row.slopmeter as i32 - row.radon as i32,
            row.delta,
            "{}",
            row.name
        );
    }
    let nonzero: Vec<_> = rows
        .iter()
        .filter(|row| row.delta != 0)
        .map(|row| row.name)
        .collect();
    assert_eq!(nonzero, vec!["or_pattern"]);
}
