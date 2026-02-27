// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Pattern matching support for transactional test expectations.
//!
//! This module provides FileCheck-style pattern matching for test output validation.
//! Patterns are specified using `[[regex]]` syntax in expected output files.
//!
//! # Syntax
//!
//! - `[[pattern]]` - Matches the regex pattern
//! - Text outside `[[...]]` is matched literally
//!
//! # Examples
//!
//! ```text
//! Error: [[.*variant \w+.*not found.*]]
//! status: [[SUCCESS|ABORTED]]
//! address: [[0x[0-9a-f]+]]
//! value: [[.*]]
//! ```

use regex::Regex;

/// Checks if actual output matches expected output.
pub fn output_matches(actual: &str, expected: &str) -> bool {
    // Quick check: if expected output contains no patterns, do exact comparison.
    if !expected.contains("[[") || !expected.contains("]]") {
        return actual == expected;
    }

    let actual_lines = actual.lines().collect::<Vec<&str>>();
    let expected_lines = expected.lines().collect::<Vec<&str>>();
    if actual_lines.len() != expected_lines.len() {
        return false;
    }

    actual_lines
        .iter()
        .zip(expected_lines.iter())
        .all(|(actual_line, expected_line)| {
            parse_line_to_regex(expected_line)
                .map(|regex| regex.is_match(actual_line))
                .unwrap_or(false)
        })
}

/// Parses a line with pattern markers into one single regex.
///
/// Converts a line like:
///   `Error: [[.*variant \w+]] not found`
/// Into a regex that matches:
///   - Literal "Error: "
///   - Pattern ".*variant \w+"
///   - Literal " not found"
fn parse_line_to_regex(line: &str) -> Option<Regex> {
    let mut in_pattern = false;
    let mut regex_pattern = String::new();

    let mut chars = line.chars().peekable();
    let mut current_segment = String::new();

    while let Some(ch) = chars.next() {
        match (ch, chars.peek(), in_pattern) {
            ('[', Some(&'['), false) => {
                // Pattern starts here.
                chars.next();
                if !current_segment.is_empty() {
                    regex_pattern.push_str(&regex::escape(&current_segment));
                    current_segment.clear();
                }
                in_pattern = true;
            },
            (']', Some(&']'), true) => {
                // Pattern ends here.
                chars.next();
                let pattern = current_segment.trim();
                if pattern.is_empty() {
                    return None;
                }
                regex_pattern.push_str(pattern);
                current_segment.clear();
                in_pattern = false;
            },
            _ => current_segment.push(ch),
        }
    }

    if in_pattern {
        return None;
    }

    if !current_segment.is_empty() {
        regex_pattern.push_str(&regex::escape(&current_segment));
    }

    let anchored = if regex_pattern.is_empty() {
        format!("^{}$", regex::escape(line))
    } else {
        format!("^{}$", regex_pattern)
    };

    Regex::new(&anchored).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exact_match_no_patterns() {
        let actual = "Hello, world!\nThis is a test.";
        let expected = "Hello, world!\nThis is a test.";
        assert!(output_matches(actual, expected));
    }

    #[test]
    fn test_exact_match_failure() {
        let actual = "Hello, world!";
        let expected = "Goodbye, world!";
        assert!(!output_matches(actual, expected));
    }

    #[test]
    fn test_simple_pattern() {
        let actual = "Error: variant Yellow not found";
        let expected = "Error: [[.*variant.*not found.*]]";
        assert!(output_matches(actual, expected));
    }

    #[test]
    fn test_multiple_patterns_per_line() {
        let actual = "Task 1: status SUCCESS, code 0x42";
        let expected = "Task [[\\d+]]: status [[SUCCESS|ABORTED]], code [[0x[0-9a-f]+]]";
        assert!(output_matches(actual, expected));
    }

    #[test]
    fn test_wildcard_pattern() {
        let actual = "The result is: 12345 with some extra info";
        let expected = "The result is: [[.*]] with some extra info";
        assert!(output_matches(actual, expected));
    }

    #[test]
    fn test_mixed_literal_and_pattern() {
        let actual = "processed 5 tasks";
        let expected = "processed [[\\d+]] tasks";
        assert!(output_matches(actual, expected));
    }

    #[test]
    fn test_pattern_mismatch() {
        let actual = "Error: something went wrong";
        let expected = "Success: [[.*]]";
        assert!(!output_matches(actual, expected));
    }

    #[test]
    fn test_line_count_mismatch() {
        let actual = "line 1\nline 2";
        let expected = "line 1\nline 2\nline 3";
        assert!(!output_matches(actual, expected));
    }

    #[test]
    fn test_regex_special_chars_in_literal() {
        let actual = "Price: $100 (20% off)";
        let expected = "Price: $100 (20% off)";
        assert!(output_matches(actual, expected));
    }

    #[test]
    fn test_multiline_with_patterns() {
        let actual = "processed 3 tasks\ntask 1: SUCCESS\ntask 2: ABORTED\ntask 3: SUCCESS";
        let expected =
            "processed [[\\d+]] tasks\ntask [[\\d+]]: [[SUCCESS|ABORTED]]\ntask [[\\d+]]: [[SUCCESS|ABORTED]]\ntask [[\\d+]]: SUCCESS";
        assert!(output_matches(actual, expected));
    }

    #[test]
    fn test_address_pattern() {
        let actual = "address: 0x1a2b3c4d";
        let expected = "address: [[0x[0-9a-f]+]]";
        assert!(output_matches(actual, expected));
    }

    #[test]
    fn test_word_boundary_pattern() {
        let actual = "Error: identifier foo not found in scope";
        let expected = "Error: identifier [[\\w+]] not found in scope";
        assert!(output_matches(actual, expected));
    }

    #[test]
    fn test_pattern_with_escaped_regex() {
        let actual = "Result: (success)";
        let expected = "Result: [[\\(\\w+\\)]]";
        assert!(output_matches(actual, expected));
    }

    #[test]
    fn test_empty_lines() {
        let actual = "line 1\n\nline 3";
        let expected = "line 1\n\nline 3";
        assert!(output_matches(actual, expected));
    }

    #[test]
    fn test_pattern_at_start() {
        let actual = "0x42 is the address";
        let expected = "[[0x[0-9a-f]+]] is the address";
        assert!(output_matches(actual, expected));
    }

    #[test]
    fn test_pattern_at_end() {
        let actual = "address is 0x42";
        let expected = "address is [[0x[0-9a-f]+]]";
        assert!(output_matches(actual, expected));
    }

    #[test]
    fn test_full_line_pattern() {
        let actual = "anything goes here!";
        let expected = "[[.*]]";
        assert!(output_matches(actual, expected));
    }
}
