use std::collections::HashMap;
use std::collections::VecDeque;
use std::fmt;
use std::fs;
use std::io;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

pub const DIFF_CONTEXT_SIZE: usize = 3;

#[derive(Debug, PartialEq)]
pub(crate) enum DiffLine {
    Context(String),
    Expected(String),
    Resulting(String),
}

#[derive(Debug, PartialEq)]
pub struct Mismatch {
    /// The line number in the formatted version.
    pub(crate) line_number: u32,
    /// The line number in the original version.
    pub(crate) line_number_orig: u32,
    /// The set of lines (context and old/new) in the mismatch.
    pub(crate) lines: Vec<DiffLine>,
}

impl Mismatch {
    fn new(line_number: u32, line_number_orig: u32) -> Mismatch {
        Mismatch {
            line_number,
            line_number_orig,
            lines: Vec::new(),
        }
    }
}

/// A single span of changed lines, with 0 or more removed lines
/// and a vector of 0 or more inserted lines.
#[derive(Debug, PartialEq, Eq)]
pub struct ModifiedChunk {
    /// The first to be removed from the original text
    pub line_number_orig: u32,
    /// The number of lines which have been replaced
    pub lines_removed: u32,
    /// The new lines
    pub lines: Vec<String>,
}

/// Set of changed sections of a file.
#[derive(Debug, PartialEq, Eq)]
pub struct ModifiedLines {
    /// The set of changed chunks.
    pub chunks: Vec<ModifiedChunk>,
}

impl From<Vec<Mismatch>> for ModifiedLines {
    fn from(mismatches: Vec<Mismatch>) -> ModifiedLines {
        let chunks = mismatches.into_iter().map(|mismatch| {
            let lines = mismatch.lines.iter();
            let num_removed = lines
                .filter(|line| matches!(line, DiffLine::Resulting(_)))
                .count();

            let new_lines = mismatch.lines.into_iter().filter_map(|line| match line {
                DiffLine::Context(_) | DiffLine::Resulting(_) => None,
                DiffLine::Expected(str) => Some(str),
            });

            ModifiedChunk {
                line_number_orig: mismatch.line_number_orig,
                lines_removed: num_removed as u32,
                lines: new_lines.collect(),
            }
        });

        ModifiedLines {
            chunks: chunks.collect(),
        }
    }
}

// Converts a `Mismatch` into a serialized form, which just includes
// enough information to modify the original file.
// Each section starts with a line with three integers, space separated:
//     lineno num_removed num_added
// followed by (`num_added`) lines of added text. The line numbers are
// relative to the original file.
impl fmt::Display for ModifiedLines {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for chunk in &self.chunks {
            writeln!(
                f,
                "{} {} {}",
                chunk.line_number_orig,
                chunk.lines_removed,
                chunk.lines.len()
            )?;

            for line in &chunk.lines {
                writeln!(f, "{line}")?;
            }
        }

        Ok(())
    }
}

// Allows to convert `Display`ed `ModifiedLines` back to the structural data.
impl std::str::FromStr for ModifiedLines {
    type Err = ();

    fn from_str(s: &str) -> Result<ModifiedLines, ()> {
        let mut chunks = vec![];

        let mut lines = s.lines();
        while let Some(header) = lines.next() {
            let mut header = header.split_whitespace();
            let (orig, rem, new_lines) = match (header.next(), header.next(), header.next()) {
                (Some(orig), Some(removed), Some(added)) => (orig, removed, added),
                _ => return Err(()),
            };
            let (orig, rem, new_lines): (u32, u32, usize) =
                match (orig.parse(), rem.parse(), new_lines.parse()) {
                    (Ok(a), Ok(b), Ok(c)) => (a, b, c),
                    _ => return Err(()),
                };
            let lines = lines.by_ref().take(new_lines);
            let lines: Vec<_> = lines.map(ToOwned::to_owned).collect();
            if lines.len() != new_lines {
                return Err(());
            }

            chunks.push(ModifiedChunk {
                line_number_orig: orig,
                lines_removed: rem,
                lines,
            });
        }

        Ok(ModifiedLines { chunks })
    }
}

// This struct handles writing output to stdout and abstracts away the logic
// of printing in color, if it's possible in the executing environment.
pub(crate) struct OutputWriter {
    terminal: Option<Box<dyn term::Terminal<Output = io::Stdout>>>,
}

impl OutputWriter {
    // Create a new OutputWriter instance based on the caller's preference
    // for colorized output and the capabilities of the terminal.
    pub(crate) fn new() -> Self {
        if let Some(t) = term::stdout() {
            if t.supports_color() {
                return OutputWriter { terminal: Some(t) };
            }
        }
        OutputWriter { terminal: None }
    }

    // Write output in the optionally specified color. The output is written
    // in the specified color if this OutputWriter instance contains a
    // Terminal in its `terminal` field.
    pub(crate) fn writeln(&mut self, msg: &str, color: Option<term::color::Color>) {
        match &mut self.terminal {
            Some(ref mut t) => {
                if let Some(color) = color {
                    t.fg(color).unwrap();
                }
                writeln!(t, "{msg}").unwrap();
                if color.is_some() {
                    t.reset().unwrap();
                }
            }
            None => tracing::debug!("{msg}"),
        }
    }
}

// Produces a diff between the expected output and actual output of movefmt.
pub fn make_diff(expected: &str, actual: &str, context_size: usize) -> Vec<Mismatch> {
    let mut line_number = 1;
    let mut line_number_orig = 1;
    let mut context_queue: VecDeque<&str> = VecDeque::with_capacity(context_size);
    let mut lines_since_mismatch = context_size + 1;
    let mut results = Vec::new();
    let mut mismatch = Mismatch::new(0, 0);

    for result in diff::lines(expected, actual) {
        match result {
            diff::Result::Left(str) => {
                if lines_since_mismatch >= context_size && lines_since_mismatch > 0 {
                    results.push(mismatch);
                    mismatch = Mismatch::new(
                        line_number - context_queue.len() as u32,
                        line_number_orig - context_queue.len() as u32,
                    );
                }

                while let Some(line) = context_queue.pop_front() {
                    mismatch.lines.push(DiffLine::Context(line.to_owned()));
                }

                mismatch.lines.push(DiffLine::Resulting(str.to_owned()));
                line_number_orig += 1;
                lines_since_mismatch = 0;
            }
            diff::Result::Right(str) => {
                if lines_since_mismatch >= context_size && lines_since_mismatch > 0 {
                    results.push(mismatch);
                    mismatch = Mismatch::new(
                        line_number - context_queue.len() as u32,
                        line_number_orig - context_queue.len() as u32,
                    );
                }

                while let Some(line) = context_queue.pop_front() {
                    mismatch.lines.push(DiffLine::Context(line.to_owned()));
                }

                mismatch.lines.push(DiffLine::Expected(str.to_owned()));
                line_number += 1;
                lines_since_mismatch = 0;
            }
            diff::Result::Both(str, _) => {
                if context_queue.len() >= context_size {
                    let _ = context_queue.pop_front();
                }

                if lines_since_mismatch < context_size {
                    mismatch.lines.push(DiffLine::Context(str.to_owned()));
                } else if context_size > 0 {
                    context_queue.push_back(str);
                }

                line_number += 1;
                line_number_orig += 1;
                lines_since_mismatch += 1;
            }
        }
    }

    results.push(mismatch);
    results.remove(0);

    results
}

pub(crate) fn print_diff<F>(diff: Vec<Mismatch>, get_section_title: F)
where
    F: Fn(u32) -> String,
{
    let line_terminator = "⏎";

    let mut writer = OutputWriter::new();

    for mismatch in diff {
        let title = get_section_title(mismatch.line_number_orig);
        writer.writeln(&title, None);

        for line in mismatch.lines {
            match line {
                DiffLine::Context(ref str) => {
                    writer.writeln(&format!(" {str}{line_terminator}"), None)
                }
                DiffLine::Expected(ref str) => writer.writeln(
                    &format!("+{str}{line_terminator}"),
                    Some(term::color::GREEN),
                ),
                DiffLine::Resulting(ref str) => {
                    writer.writeln(&format!("-{str}{line_terminator}"), Some(term::color::RED))
                }
            }
        }
    }
}

pub fn print_mismatches_default_message(result: HashMap<PathBuf, Vec<Mismatch>>) {
    for (file_name, diff) in result {
        let mismatch_msg_formatter =
            |line_num| format!("\nMismatch at {}:{}:", file_name.display(), line_num);
        print_diff(diff, mismatch_msg_formatter);
    }

    if let Some(mut t) = term::stdout() {
        t.reset().unwrap_or(());
    }
}

// Helper function for comparing the results of movefmt
// to a known output file generated by one of the write modes.
pub fn assert_output(actual_filename: &Path, expected_filename: &Path) {
    let mut expected_file = fs::File::open(expected_filename).expect("couldn't open target");
    let mut expected_text = String::new();
    expected_file
        .read_to_string(&mut expected_text)
        .expect("Failed reading target");

    let mut actual_file = fs::File::open(actual_filename).expect("couldn't open target");
    let mut actual_text = String::new();
    actual_file
        .read_to_string(&mut actual_text)
        .expect("Failed reading target");

    let compare = make_diff(&expected_text, &actual_text, DIFF_CONTEXT_SIZE);
    if !compare.is_empty() {
        let mut failures = HashMap::new();
        failures.insert(actual_filename.to_owned(), compare);
        print_mismatches_default_message(failures);
        panic!("Text does not match expected output");
    }
}
