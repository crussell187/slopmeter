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
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::page;

    const RULES: [&str; 10] = [
        "F01", "F02", "F03", "F04", "F05", "F06", "F07", "F08", "F11", "S06",
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
            ("F01", "Python gate at 48."),
            ("F03", "gates above 58."),
            ("F04", "Python gate at 19."),
            ("F05", "Python gate at 6."),
            ("F06", "Python gate at 196."),
            ("F08", "Python gates at 3 hops from the"),
            ("F11", "Python gate at 64."),
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
            ("F01", "Rust gate at 94."),
            ("F03", "gates above 192."),
            ("F04", "Rust gate at 9."),
            ("F05", "Rust gate at 9."),
            ("F06", "Rust gate at"),
            ("F08", "Rust gates at 2 hops from the receiver"),
            ("F11", "Rust gate at 138."),
        ];
        for (rule, phrase) in pages {
            assert!(page(rule).unwrap().contains(phrase), "{rule}");
        }
        assert!(page("F06").unwrap().contains("581."));
    }

    #[test]
    fn cpp_thresholds_are_the_calibrated_gates() {
        let pages = [
            ("F01", "C++ gate at 136."),
            ("F03", "gates above 295."),
            ("F04", "C++ gate at 18."),
            ("F05", "C++ gate at 8."),
            ("F06", "C++ gate at 713."),
            ("F08", "C++ gates at 3 hops from the receiver"),
            ("F11", "C++ gate at 217."),
        ];
        for (rule, phrase) in pages {
            assert!(page(rule).unwrap().contains(phrase), "{rule}");
        }
        assert!(page("F08").unwrap().contains("14 otherwise."));
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
