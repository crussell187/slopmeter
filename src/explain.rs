pub fn page(rule: &str) -> Option<&'static str> {
    if let Some(text) = function_page(rule) {
        return Some(text);
    }
    scope_page(rule)
}

fn function_page(rule: &str) -> Option<&'static str> {
    let early = count_page(rule);
    if early.is_some() {
        return early;
    }
    later_page(rule)
}

fn count_page(rule: &str) -> Option<&'static str> {
    Some(match rule {
        "F01" => include_str!("../docs/explain/F01.txt"),
        "F02" => include_str!("../docs/explain/F02.txt"),
        "F03" => include_str!("../docs/explain/F03.txt"),
        "F04" => include_str!("../docs/explain/F04.txt"),
        "F05" => include_str!("../docs/explain/F05.txt"),
        "F06" => include_str!("../docs/explain/F06.txt"),
        _ => return None,
    })
}

fn later_page(rule: &str) -> Option<&'static str> {
    Some(match rule {
        "F07" => include_str!("../docs/explain/F07.txt"),
        "F08" => include_str!("../docs/explain/F08.txt"),
        "F09" => include_str!("../docs/explain/F09.txt"),
        "F15" => include_str!("../docs/explain/F15.txt"),
        _ => return None,
    })
}

fn scope_page(rule: &str) -> Option<&'static str> {
    Some(match rule {
        "F11" => include_str!("../docs/explain/F11.txt"),
        "S06" => include_str!("../docs/explain/S06.txt"),
        "S07" => include_str!("../docs/explain/S07.txt"),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::page;

    const RULES: [&str; 13] = [
        "F01", "F02", "F03", "F04", "F05", "F06", "F07", "F08", "F09", "F11", "F15", "S06", "S07",
    ];
    const GATES: [&str; 9] = [
        "F01", "F02", "F03", "F04", "F05", "F06", "F08", "F11", "F15",
    ];
    const WAIVER: &str = "A contiguous `# slopmeter: waive <RULE> <NUMBER> <REASON>` run directly above the function excuses this gate in Python, and the same tokens after `//` do so in Rust and C++, with a whole number except on F02, which may be fractional. The comment has to be in a change a person already reviewed.";

    #[test]
    fn a_known_rule_has_a_page_and_an_unknown_rule_does_not() {
        for rule in RULES {
            assert!(page(rule).is_some(), "{rule}");
        }
        assert!(page("F10").is_none());
        assert!(page("f01").is_none());
    }

    #[test]
    fn each_page_has_the_frozen_sections_in_order() {
        for rule in RULES {
            let text = page(rule).unwrap();
            let counting = text.find("\nCounting\n").unwrap();
            let threshold = text.find("\nThreshold\n").unwrap();
            let before = text.find("\nBefore\n").unwrap();
            let after = text.find("\nAfter\n").unwrap();
            let anti = text.find("\nAnti-fixes\n").unwrap();
            assert!(counting < threshold && threshold < before && before < after && after < anti);
            assert!(!text[anti + "\nAnti-fixes\n".len()..].trim().is_empty());
        }
    }

    #[test]
    fn every_gate_page_carries_the_same_waiver_sentence() {
        for rule in GATES {
            let text = page(rule)
                .unwrap()
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ");
            assert!(text.contains(WAIVER), "{rule}");
        }
        let f07 = page("F07").unwrap();
        assert!(f07.contains("A waiver line for F07 fails the run, because F07 is not a gate."));
        assert!(!f07.contains("excuses this gate"));
        let f09 = page("F09").unwrap();
        assert!(f09.contains("A waiver line for F09 fails the run, because F09 is not a gate."));
        assert!(!f09.contains("excuses this gate"));
        let s06 = page("S06").unwrap();
        assert!(s06.contains("A waiver line for S06 fails the run."));
        assert!(!s06.contains("excuses this gate"));
        let s07 = page("S07").unwrap();
        assert!(s07.contains("A waiver line for S07 fails the run."));
        assert!(!s07.contains("excuses this gate"));
    }

    #[test]
    fn f02_states_the_crap_formula() {
        let text = page("F02")
            .unwrap()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        assert!(
            text.contains("The score is CC squared times the uncovered fraction cubed, plus CC.")
        );
    }

    #[test]
    fn each_threshold_is_one_table_and_names_no_stand_in() {
        let tables = [
            (
                "F01",
                "        gate\nPython    10\nRust      10\nC++       12\n",
            ),
            (
                "F02",
                "        gate\nPython     8\nRust       8\nC++        8\n",
            ),
            (
                "F03",
                "        warn  gate\nPython     8    12\nRust       3     6\nC++        7    15\n",
            ),
            (
                "F04",
                "        gate\nPython    10\nRust       3\nC++        4\n",
            ),
            (
                "F05",
                "        gate\nPython     3\nRust       3\nC++        3\n",
            ),
            (
                "F06",
                "        gate\nPython    51\nRust      45\nC++       41\n",
            ),
            (
                "F07",
                "        warn\nPython    80\nRust      80\nC++       80\n",
            ),
            (
                "F08",
                "        receiver  other\nPython         2      1\nRust           2      1\nC++            2      1\n",
            ),
            (
                "F09",
                "        warn\nPython   any\nRust     any\nC++      any\n",
            ),
            (
                "F11",
                "        gate\nPython    19\nRust      17\nC++       25\n",
            ),
            (
                "F15",
                "        gate\nPython   any\nRust     any\nC++      any\n",
            ),
            (
                "S06",
                "        gate\nPython     1\nRust       1\nC++        1\n",
            ),
            (
                "S07",
                "        gate\nPython     1\nRust       1\nC++        1\n",
            ),
        ];
        for (rule, table) in tables {
            let text = page(rule).unwrap();
            let threshold = text.find("\nThreshold\n").unwrap();
            let before = text.find("\nBefore\n").unwrap();
            let section = &text[threshold..before];
            assert!(
                !section.contains("Shipped stand-in:"),
                "{rule} still marks a stand-in"
            );
            assert!(
                section.contains(table),
                "{rule} threshold is not the table:\n{section}"
            );
        }
        let f03 = page("F03").unwrap();
        assert!(!f03.contains("8 and 12 pass"));
    }

    #[test]
    fn f02_after_adds_a_test_and_keeps_the_branches() {
        let text = page("F02").unwrap();
        let after = text.find("\nAfter\n").unwrap();
        let anti = text.find("\nAnti-fixes\n").unwrap();
        let section = &text[after..anti];
        assert!(section.contains("for i in range(n):"));
        assert!(section.contains("def test_classify():"));
        assert!(section.contains("assert classify("));
    }

    #[test]
    fn each_page_stays_within_a_page() {
        for rule in RULES {
            let text = page(rule).unwrap();
            let lines: Vec<&str> = text.lines().collect();
            assert!(lines.len() <= 60, "{rule} has {} lines", lines.len());
            for line in lines {
                assert!(line.chars().count() <= 72, "{rule}: {line}");
                assert!(!line.ends_with(' '), "{rule}");
            }
        }
    }
}
