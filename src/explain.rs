pub fn page(rule: &str) -> Option<&'static str> {
    Some(match rule {
        "F01" => include_str!("../docs/explain/F01.txt"),
        "F02" => include_str!("../docs/explain/F02.txt"),
        "F03" => include_str!("../docs/explain/F03.txt"),
        "F04" => include_str!("../docs/explain/F04.txt"),
        "F05" => include_str!("../docs/explain/F05.txt"),
        "F06" => include_str!("../docs/explain/F06.txt"),
        "F07" => include_str!("../docs/explain/F07.txt"),
        "F08" => include_str!("../docs/explain/F08.txt"),
        "F11" => include_str!("../docs/explain/F11.txt"),
        "S06" => include_str!("../docs/explain/S06.txt"),
        "S07" => include_str!("../docs/explain/S07.txt"),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::page;

    const RULES: [&str; 11] = [
        "F01", "F02", "F03", "F04", "F05", "F06", "F07", "F08", "F11", "S06", "S07",
    ];
    const GATES: [&str; 8] = ["F01", "F02", "F03", "F04", "F05", "F06", "F08", "F11"];
    const WAIVER: &str = "A contiguous `# slopmeter: waive <RULE> <NUMBER> <REASON>` run directly above the function excuses this gate in Python, and the same tokens after `//` do so in Rust and C++, with a whole number except on F02, which may be fractional.";

    #[test]
    fn a_known_rule_has_a_page_and_an_unknown_rule_does_not() {
        for rule in RULES {
            assert!(page(rule).is_some(), "{rule}");
        }
        assert!(page("F09").is_none());
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
    fn python_thresholds_are_the_calibrated_gates() {
        let pages = [
            ("F01", "Python gate at 10."),
            ("F03", "gates above 12."),
            ("F04", "Python gate at 10."),
            ("F05", "gate at 3 in every language."),
            ("F06", "Python gate at 51."),
            ("F08", "1 otherwise, in every language."),
            ("F11", "Python gate at 19."),
        ];
        for (rule, phrase) in pages {
            assert!(page(rule).unwrap().contains(phrase), "{rule}");
        }
        assert!(page("F02").unwrap().contains("gate at 8 in every language"));
        assert!(page("F07")
            .unwrap()
            .contains("warn at 80 percent in every language"));
    }

    #[test]
    fn rust_thresholds_are_the_calibrated_gates() {
        let pages = [
            ("F01", "Rust gate at 10."),
            ("F03", "gates above 6."),
            ("F04", "Rust gate at 3."),
            ("F05", "gate at 3 in every language."),
            ("F06", "Rust gate at 45."),
            ("F08", "2 hops from the receiver"),
            ("F11", "Rust gate at 17."),
        ];
        for (rule, phrase) in pages {
            assert!(page(rule).unwrap().contains(phrase), "{rule}");
        }
    }

    #[test]
    fn cpp_thresholds_are_the_calibrated_gates() {
        let pages = [
            ("F01", "C++ gate at 12."),
            ("F03", "gates above 15."),
            ("F04", "C++ gate at 4."),
            ("F05", "gate at 3 in every language."),
            ("F06", "C++ gate at 41."),
            ("F08", "1 otherwise, in every language."),
            ("F11", "C++ gate at 25."),
        ];
        for (rule, phrase) in pages {
            assert!(page(rule).unwrap().contains(phrase), "{rule}");
        }
    }

    #[test]
    fn each_threshold_is_marked_as_the_shipped_stand_in() {
        for rule in RULES {
            let text = page(rule).unwrap();
            let threshold = text.find("\nThreshold\n").unwrap();
            let before = text.find("\nBefore\n").unwrap();
            assert!(text[threshold..before].contains("Shipped stand-in:"));
        }
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
