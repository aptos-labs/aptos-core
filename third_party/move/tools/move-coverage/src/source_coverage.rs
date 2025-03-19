// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use crate::coverage_map::CoverageMap;
use clap::ValueEnum;
use codespan::{FileId, Files, Span};
use colored::{self, Colorize};
use move_binary_format::{
    access::ModuleAccess,
    file_format::{CodeOffset, FunctionDefinitionIndex},
    CompiledModule,
};
use move_bytecode_source_map::source_map::SourceMap;
use move_command_line_common::files::FileHash;
use move_ir_types::location::Loc;
use serde::Serialize;
use std::{
    cmp::{Ordering, PartialOrd},
    collections::{BTreeMap, BTreeSet},
    fmt::{Display, Formatter},
    fs,
    io::{self, Write},
    path::Path,
    str::FromStr,
};

/// Source-level coverage information for a function.
/// Deprecated, use `FunctionSourceCoverageV2` instead.
/// Kept around for legacy usages.
#[derive(Clone, Debug, Serialize)]
pub struct FunctionSourceCoverage {
    /// Is this a native function?
    /// If so, then `uncovered_locations` is empty.
    pub fn_is_native: bool,

    /// List of source locations in the function that were not covered.
    pub uncovered_locations: Vec<Loc>,
}

/// Source-level positive coverage information for a function.
#[derive(Clone, Debug, Serialize)]
pub struct FunctionSourceCoverageV2 {
    /// Is this a native function?
    /// If so, the remaining fields are empty.
    pub fn_is_native: bool,

    /// List of source locations in the function that were covered.
    pub covered_locations: Vec<Loc>,

    /// List of all (executable) source locations in the function.
    pub all_locations: Vec<Loc>,
}

impl FunctionSourceCoverageV2 {
    /// Coverage information for a native function.
    fn for_native() -> Self {
        Self {
            fn_is_native: true,
            covered_locations: vec![],
            all_locations: vec![],
        }
    }
}

/// Builder for the source code coverage.
#[derive(Debug, Serialize)]
pub struct SourceCoverageBuilder<'a> {
    /// Source-level uncovered locations.
    pub uncovered_locations: Vec<Loc>,

    source_map: &'a SourceMap,
}

#[derive(Debug, Serialize, Eq, PartialEq)]
pub enum AbstractSegment {
    Bounded { start: u32, end: u32 },
    BoundedRight { end: u32 },
    BoundedLeft { start: u32 },
}

impl AbstractSegment {
    fn get_start(&self) -> u32 {
        use AbstractSegment::*;
        match self {
            Bounded { start, .. } => *start,
            BoundedRight { .. } => 0u32,
            BoundedLeft { start, .. } => *start,
        }
    }

    fn get_number(&self) -> u8 {
        use AbstractSegment::*;
        match self {
            Bounded { .. } => 0,
            BoundedRight { .. } => 1,
            BoundedLeft { .. } => 2,
        }
    }
}

impl PartialOrd for AbstractSegment {
    fn partial_cmp(&self, other: &AbstractSegment) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for AbstractSegment {
    fn cmp(&self, other: &AbstractSegment) -> Ordering {
        use Ordering::*;
        match self.get_start().cmp(&other.get_start()) {
            Less => Less,
            Greater => Greater,
            Equal => self.get_number().cmp(&other.get_number()),
        }
    }
}

/// Option to control use of color escape codes in coverage output
/// to indicate source code coverage.  Unless `None`
/// is selected, code which is covered is green, uncovered
/// code is red.  By `Default`, color is only shown when
/// output goes to a terminal.  If `Always`, then color
/// escapes are included in the output even to a file
/// or other program.
#[derive(ValueEnum, Clone, Debug, Serialize)]
pub enum ColorChoice {
    /// Color is never shown
    None,
    /// Color is shown only on a terminal
    Default,
    /// Color is always shown
    Always,
}

impl Display for ColorChoice {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        use ColorChoice::*;
        match self {
            None => f.write_str("none"),
            Default => f.write_str("default"),
            Always => f.write_str("always"),
        }
    }
}

impl FromStr for ColorChoice {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use ColorChoice::*;
        match s {
            "none" => Ok(None),
            "default" => Ok(Default),
            "always" => Ok(Always),
            _ => Err("unknown variant"),
        }
    }
}

/// Option to control use of explicit textual indication of lines
/// covered or not in test coverage listings.  If `On` or
/// `Explicit` is selected, then lines with missing coverage
/// are tagged with `-`; otherwise, they have `+`.
#[derive(ValueEnum, Clone, Debug, Serialize)]
pub enum TextIndicator {
    /// No textual indicator of coverage.
    None,
    /// Prefix each line with some code missing coverage by `-`;
    /// other lines are prefixed with `+`.
    Explicit,
    /// Same behavior as Explicit.
    On,
}

impl Display for TextIndicator {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        use TextIndicator::*;
        match self {
            None => f.write_str("none"),
            Explicit => f.write_str("explicit"),
            On => f.write_str("on"),
        }
    }
}

impl FromStr for TextIndicator {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use TextIndicator::*;
        match s {
            "none" => Ok(None),
            "explicit" => Ok(Explicit),
            "on" => Ok(On),
            _ => Err("unknown variant"),
        }
    }
}

impl ColorChoice {
    fn execute(&self) {
        use ColorChoice::*;
        match self {
            None => {
                colored::control::set_override(false);
            },
            Default => {},
            Always => {
                colored::control::set_override(true);
            },
        }
    }

    fn undo(&self) {
        use ColorChoice::*;
        match self {
            None => {
                colored::control::unset_override();
            },
            Default => {},
            Always => {
                colored::control::unset_override();
            },
        }
    }
}

#[derive(Debug, Serialize)]
pub enum StringSegment {
    Covered(String),
    Uncovered(String),
}

pub type AnnotatedLine = Vec<StringSegment>;

#[derive(Debug, Serialize)]
pub struct SourceCoverage {
    pub annotated_lines: Vec<AnnotatedLine>,
}

impl<'a> SourceCoverageBuilder<'a> {
    pub fn new(
        coverage_map: &CoverageMap,
        source_map: &'a SourceMap,
        root_modules: Vec<(&CompiledModule, &'a SourceMap)>,
    ) -> Self {
        let module_loc = source_map.definition_location;
        let unified_exec_map = coverage_map.to_unified_exec_map();

        let mut fun_coverage: Vec<FunctionSourceCoverageV2> = Vec::new();
        for (module, source_map) in root_modules.iter() {
            let module_name = module.self_id();
            let module_map = unified_exec_map
                .module_maps
                .get(&(*module_name.address(), module_name.name().to_owned()));
            if let Some(module_map) = module_map {
                for (function_def_idx, function_def) in module.function_defs().iter().enumerate() {
                    let fn_handle = module.function_handle_at(function_def.function);
                    let fn_name = module.identifier_at(fn_handle.name).to_owned();
                    let function_def_idx = FunctionDefinitionIndex(function_def_idx as u16);
                    let function_covered_locations: FunctionSourceCoverageV2 =
                        match &function_def.code {
                            None => FunctionSourceCoverageV2::for_native(),
                            Some(code_unit) => match module_map.function_maps.get(&fn_name) {
                                None => {
                                    let all_locations = minimize_locations(
                                        (0..code_unit.code.len())
                                            .filter_map(|code_offset| {
                                                if let Ok(loc) = source_map.get_code_location(
                                                    function_def_idx,
                                                    code_offset as CodeOffset,
                                                ) {
                                                    Some(loc)
                                                } else {
                                                    None
                                                }
                                            })
                                            .collect(),
                                    );
                                    FunctionSourceCoverageV2 {
                                        fn_is_native: false,
                                        covered_locations: vec![],
                                        all_locations,
                                    }
                                },
                                Some(function_coverage) => {
                                    let (fun_cov, fun_uncov): (Vec<_>, Vec<_>) =
                                        (0..code_unit.code.len())
                                            .filter_map(|code_offset| {
                                                if let Ok(loc) = source_map.get_code_location(
                                                    function_def_idx,
                                                    code_offset as CodeOffset,
                                                ) {
                                                    if function_coverage
                                                        .get(&(code_offset as u64))
                                                        .unwrap_or(&0)
                                                        != &0
                                                    {
                                                        // Non-zero execution count, so covered.
                                                        Some((loc, true))
                                                    } else {
                                                        Some((loc, false))
                                                    }
                                                } else {
                                                    None
                                                }
                                            })
                                            .partition(|(_, covered)| *covered);

                                    let covered_locations: Vec<_> = minimize_locations(
                                        fun_cov.iter().map(|(loc, _)| *loc).collect(),
                                    );
                                    let uncovered_locations: Vec<_> = minimize_locations(
                                        fun_uncov.iter().map(|(loc, _)| *loc).collect(),
                                    );
                                    // If any uncovered locations are the same as covered locations,
                                    // remove them from uncovered locations.
                                    let uncovered_locations =
                                        BTreeSet::from_iter(uncovered_locations.into_iter())
                                            .difference(&BTreeSet::from_iter(
                                                covered_locations.iter().cloned(),
                                            ))
                                            .cloned()
                                            .collect();
                                    // Covered locations may be an over-approximation, so uncovered
                                    // locations are subtracted from covered locations.
                                    let covered_locations = subtract_locations(
                                        covered_locations.into_iter().collect(),
                                        &uncovered_locations,
                                    );
                                    let all_locations: Vec<_> = minimize_locations(
                                        fun_cov
                                            .iter()
                                            .chain(fun_uncov.iter())
                                            .map(|(loc, _)| *loc)
                                            .collect(),
                                    );

                                    FunctionSourceCoverageV2 {
                                        fn_is_native: false,
                                        covered_locations,
                                        all_locations,
                                    }
                                },
                            },
                        };
                    fun_coverage.push(function_covered_locations);
                }
            }
        }

        // Filter locations for this module and build 2 sets: covered and all locations.
        // Note that compiler v1 sets module_loc to the location of symbol in definition, so if
        // there are multiple modules in one file we may leave others in the picture.
        let module_file_hash = module_loc.file_hash();

        let (covered, all): (BTreeSet<Loc>, BTreeSet<Loc>) = fun_coverage
            .iter()
            .map(
                |FunctionSourceCoverageV2 {
                     covered_locations,
                     all_locations,
                     ..
                 }| {
                    let cov: BTreeSet<_> = covered_locations
                        .iter()
                        .filter(|loc| loc.file_hash() == module_file_hash)
                        .cloned()
                        .collect();
                    let all: BTreeSet<_> = all_locations
                        .iter()
                        .filter(|loc| loc.file_hash() == module_file_hash)
                        .cloned()
                        .collect();
                    (cov, all)
                },
            )
            .reduce(|(c1, a1), (c2, a2)| {
                (
                    c1.union(&c2).cloned().collect(),
                    a1.union(&a2).cloned().collect(),
                )
            })
            .unwrap_or_else(|| (BTreeSet::new(), BTreeSet::new()));

        let covered = minimize_locations(covered.into_iter().collect())
            .into_iter()
            .collect();
        let all = minimize_locations(all.into_iter().collect())
            .into_iter()
            .collect::<BTreeSet<_>>();
        let uncovered_locations = subtract_locations(all, &covered);
        Self {
            uncovered_locations,
            source_map,
        }
    }

    pub fn compute_source_coverage(&self, file_path: &Path) -> SourceCoverage {
        let file_contents = fs::read_to_string(file_path).unwrap();
        assert!(
            self.source_map.check(&file_contents),
            "File contents {} out of sync with source map",
            file_path.display()
        );
        let mut files = Files::new();
        let file_id = files.add(file_path.as_os_str().to_os_string(), file_contents.clone());

        let uncovered_spans: Vec<_> = self
            .uncovered_locations
            .iter()
            .map(|loc| Span::new(loc.start(), loc.end()))
            .collect();

        let uncovered_segments = spans_to_segments(&mut files, file_id, uncovered_spans);

        let mut annotated_lines = Vec::new();
        for (line_number, mut line) in file_contents.lines().map(|x| x.to_owned()).enumerate() {
            match uncovered_segments.get(&(line_number as u32)) {
                None => annotated_lines.push(vec![StringSegment::Covered(line)]),
                Some(segments) => {
                    // Note: segments are already pre-sorted by construction so don't need to be
                    // resorted.
                    let mut line_acc = Vec::new();
                    let mut cursor = 0;
                    for segment in segments {
                        match segment {
                            AbstractSegment::Bounded { start, end } => {
                                let length = end - start;
                                let (before, after) = line.split_at((start - cursor) as usize);
                                let (uncovered, rest) = after.split_at(length as usize);
                                line_acc.push(StringSegment::Covered(before.to_string()));
                                line_acc.push(StringSegment::Uncovered(uncovered.to_string()));
                                line = rest.to_string();
                                cursor = *end;
                            },
                            AbstractSegment::BoundedRight { end } => {
                                let (uncovered, rest) = line.split_at((end - cursor) as usize);
                                line_acc.push(StringSegment::Uncovered(uncovered.to_string()));
                                line = rest.to_string();
                                cursor = *end;
                            },
                            AbstractSegment::BoundedLeft { start } => {
                                let (before, after) = line.split_at((start - cursor) as usize);
                                line_acc.push(StringSegment::Covered(before.to_string()));
                                line_acc.push(StringSegment::Uncovered(after.to_string()));
                                line = "".to_string();
                                cursor = 0;
                            },
                        }
                    }
                    if !line.is_empty() {
                        line_acc.push(StringSegment::Covered(line))
                    }
                    annotated_lines.push(line_acc)
                },
            }
        }

        SourceCoverage { annotated_lines }
    }
}

impl SourceCoverage {
    pub fn output_source_coverage<W: Write>(
        &self,
        output_writer: &mut W,
        color: ColorChoice,
        text_indicator: TextIndicator,
    ) -> io::Result<()> {
        color.execute();
        let be_explicit = match text_indicator {
            TextIndicator::Explicit | TextIndicator::On => {
                write!(
                    output_writer,
                    "Code coverage per line of code:\n  {} indicates the line is not executable or is fully covered during execution\n  {} indicates the line is executable but NOT fully covered during execution\nSource code follows:\n",
                    "+".to_string().green(),
                    "-".to_string().bold().red(),
                )?;
                true
            },
            TextIndicator::None => false,
        };
        for line in self.annotated_lines.iter() {
            if be_explicit {
                let has_uncovered = line
                    .iter()
                    .any(|string_segment| matches!(string_segment, StringSegment::Uncovered(_)));
                write!(
                    output_writer,
                    "{} ",
                    if has_uncovered {
                        "-".to_string().red()
                    } else {
                        "+".to_string().green()
                    }
                )?;
            }
            for string_segment in line.iter() {
                match string_segment {
                    StringSegment::Covered(s) => write!(output_writer, "{}", s.green())?,
                    StringSegment::Uncovered(s) => write!(output_writer, "{}", s.bold().red())?,
                }
            }
            writeln!(output_writer)?;
        }
        color.undo();
        Ok(())
    }
}

/// Converts a (sorted) list of non-overlapping `Loc` into a map from line number to
/// set of `AbstractSegment` for each line of the file `file_id` in fileset `files`.
fn spans_to_segments(
    files: &mut Files<String>,
    file_id: FileId,
    spans: Vec<Span>,
) -> BTreeMap<u32, Vec<AbstractSegment>> {
    let mut segments = BTreeMap::new();

    for span in spans.into_iter() {
        let start_loc = files.location(file_id, span.start()).unwrap();
        let end_loc = files.location(file_id, span.end()).unwrap();
        let start_line = start_loc.line.0;
        let end_line = end_loc.line.0;
        let line_segments = segments.entry(start_line).or_insert_with(Vec::new);
        if start_line == end_line {
            let segment = AbstractSegment::Bounded {
                start: start_loc.column.0,
                end: end_loc.column.0,
            };
            if !line_segments.contains(&segment) {
                line_segments.push(segment);
            }
        } else {
            line_segments.push(AbstractSegment::BoundedLeft {
                start: start_loc.column.0,
            });
            for i in start_line + 1..end_line {
                let line_segments = segments.entry(i).or_insert_with(Vec::new);
                line_segments.push(AbstractSegment::BoundedLeft { start: 0 });
            }
            let last_line_segments = segments.entry(end_line).or_insert_with(Vec::new);
            last_line_segments.push(AbstractSegment::BoundedRight {
                end: end_loc.column.0,
            });
        }
    }
    segments.values_mut().for_each(|v| v.sort());
    segments
}

/// Merge overlapping spans.
pub fn merge_spans(file_hash: FileHash, cov: FunctionSourceCoverage) -> Vec<Span> {
    if cov.uncovered_locations.is_empty() {
        return vec![];
    }

    let mut spans = cov
        .uncovered_locations
        .iter()
        .filter(|loc| loc.file_hash() == file_hash)
        .map(|loc| Span::new(loc.start(), loc.end()))
        .collect::<Vec<_>>();
    if spans.is_empty() {
        return vec![];
    }
    spans.sort();

    let mut unioned = Vec::with_capacity(spans.len());
    let mut curr = spans.remove(0);

    for span in spans {
        if curr.end() >= span.start() {
            curr = curr.merge(span);
        } else {
            unioned.push(curr);
            curr = span;
        }
    }

    unioned.push(curr);
    unioned
}

/// Given a list of locations, merge overlapping and abutting locations.
fn minimize_locations(mut locs: Vec<Loc>) -> Vec<Loc> {
    locs.sort();
    let mut result = vec![];
    let mut locs_iter = locs.into_iter();
    if let Some(mut current_loc) = locs_iter.next() {
        for next_loc in locs_iter {
            if !current_loc.try_merge(&next_loc) {
                result.push(current_loc);
                current_loc = next_loc;
            }
        }
        result.push(current_loc);
    }
    result
}

/// Given two (sorted) sets of locations, subtract the second set from the first set.
fn subtract_locations(locs1: BTreeSet<Loc>, locs2: &BTreeSet<Loc>) -> Vec<Loc> {
    let mut result = vec![];
    let mut locs1_iter = locs1.into_iter();
    let mut locs2_iter = locs2.iter();
    if let Some(mut current_loc1) = locs1_iter.next() {
        if let Some(mut current_loc2) = locs2_iter.next() {
            loop {
                if current_loc1.overlaps(current_loc2) {
                    let mut diff = current_loc1.subtract(current_loc2);
                    if let Some(new_loc1) = diff.pop() {
                        result.append(&mut diff);
                        current_loc1 = new_loc1;
                        // continue
                    } else {
                        // diff was empty, get a new loc1
                        if let Some(new_loc1) = locs1_iter.next() {
                            current_loc1 = new_loc1;
                            // retry loc2
                            // continue
                        } else {
                            // no more loc1, return
                            return result;
                        }
                    }
                } else {
                    // no overlap
                    if current_loc1 <= *current_loc2 {
                        // loc1 is done.  save it and get a new one.
                        result.push(current_loc1);
                        if let Some(new_loc1) = locs1_iter.next() {
                            current_loc1 = new_loc1;
                            // continue
                        } else {
                            // No more loc1, return
                            return result;
                        }
                    } else {
                        // loc1 might have more overlaps, try another loc2
                        if let Some(new_loc2) = locs2_iter.next() {
                            current_loc2 = new_loc2;
                            // continue
                        } else {
                            // loc2 is finished but loc1 is not,
                            // finish adding all loc1
                            break;
                        }
                    }
                }
            }
        }
        result.push(current_loc1);
        for loc1 in locs1_iter {
            result.push(loc1);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::FunctionSourceCoverage;
    use crate::source_coverage::{merge_spans, minimize_locations, subtract_locations};
    use codespan::Span;
    use move_command_line_common::files::FileHash;
    use move_ir_types::location::Loc;

    /// Ensure merging spans works as expected.
    #[test]
    fn test_merge_spans_works() {
        let test_hash = FileHash([0; 32]);
        let other_hash = FileHash([1; 32]);

        // Let's use unsorted array on purpose.
        let uncovered_locations = vec![
            // Should be merged:
            Loc::new(test_hash, 50, 51),
            Loc::new(test_hash, 51, 52),
            // Should be merged:
            Loc::new(test_hash, 3, 5),
            Loc::new(test_hash, 2, 10),
            Loc::new(test_hash, 5, 11),
            // Should stay the same:
            Loc::new(test_hash, 15, 16),
            // Should be merged:
            Loc::new(test_hash, 20, 25),
            Loc::new(test_hash, 24, 25),
            Loc::new(test_hash, 21, 25),
            // Should stay the same:
            Loc::new(test_hash, 26, 29),
            // Shouldn't interfere with other hashes.
            Loc::new(other_hash, 2, 20),
        ];
        let expected_spans = vec![
            Span::new(2, 11),
            Span::new(15, 16),
            Span::new(20, 25),
            Span::new(26, 29),
            Span::new(50, 52),
        ];

        let cov = FunctionSourceCoverage {
            fn_is_native: false,
            uncovered_locations,
        };
        assert_eq!(expected_spans, merge_spans(test_hash, cov));
    }

    /// Ensure merging spans works fine when there is no spans to be merged.
    #[test]
    fn test_merge_spans_when_spans_are_empty() {
        let test_hash = FileHash([0; 32]);
        let other_hash = FileHash([1; 32]);

        // Check that it works when are files have full coverage.
        let cov = FunctionSourceCoverage {
            fn_is_native: false,
            uncovered_locations: vec![],
        };
        assert!(merge_spans(test_hash, cov).is_empty());

        // Check that it works when the hash under test has the full coverage.
        let uncovered_locations = vec![Loc::new(other_hash, 2, 20)];
        let cov = FunctionSourceCoverage {
            fn_is_native: false,
            uncovered_locations,
        };
        assert!(merge_spans(test_hash, cov).is_empty());
    }

    #[test]
    fn test_minimize_locations_works() {
        let test_hash = FileHash([0; 32]);
        let other_hash = FileHash([1; 32]);

        let original_locations = vec![
            // Should be merged:
            Loc::new(test_hash, 50, 51),
            Loc::new(test_hash, 51, 52),
            // Should be merged:
            Loc::new(test_hash, 3, 5),
            Loc::new(test_hash, 2, 10),
            Loc::new(test_hash, 5, 11),
            // Should stay the same:
            Loc::new(test_hash, 15, 16),
            // Should be merged:
            Loc::new(test_hash, 20, 25),
            Loc::new(test_hash, 24, 25),
            Loc::new(test_hash, 21, 25),
            // Should stay the same:
            Loc::new(test_hash, 27, 29),
            // Should be merged:
            Loc::new(test_hash, 101, 102),
            Loc::new(test_hash, 103, 105),
            Loc::new(test_hash, 106, 111),
            Loc::new(test_hash, 110, 120),
            // Shouldn't interfere with other hashes.
            Loc::new(other_hash, 2, 20),
        ];
        let minimized_locations = vec![
            Loc::new(test_hash, 2, 11),
            Loc::new(test_hash, 15, 16),
            Loc::new(test_hash, 20, 25),
            Loc::new(test_hash, 27, 29),
            Loc::new(test_hash, 50, 52),
            Loc::new(test_hash, 101, 120),
            Loc::new(other_hash, 2, 20),
        ];

        assert_eq!(minimized_locations, minimize_locations(original_locations));
    }

    #[test]
    fn test_subtract_locations_works() {
        let test_hash = FileHash([0; 32]);
        let other_hash = FileHash([1; 32]);

        let locs1 = vec![
            Loc::new(test_hash, 5, 10),
            Loc::new(test_hash, 15, 20),
            Loc::new(test_hash, 25, 30),
            Loc::new(test_hash, 35, 40),
            Loc::new(test_hash, 45, 50),
        ];

        let locs2 = vec![
            Loc::new(test_hash, 8, 12),
            Loc::new(test_hash, 12, 16),
            Loc::new(test_hash, 27, 28),
            Loc::new(test_hash, 41, 42),
            Loc::new(test_hash, 42, 44),
            Loc::new(test_hash, 51, 54),
            Loc::new(other_hash, 10, 12),
        ];

        let expected = vec![
            Loc::new(test_hash, 5, 7),
            Loc::new(test_hash, 17, 20),
            Loc::new(test_hash, 25, 26),
            Loc::new(test_hash, 29, 30),
            Loc::new(test_hash, 35, 40),
            Loc::new(test_hash, 45, 50),
        ];

        assert_eq!(
            expected,
            subtract_locations(locs1.into_iter().collect(), &locs2.into_iter().collect())
        );
    }
}
