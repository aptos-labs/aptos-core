// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use crate::coverage_map::CoverageMap;
use clap::ValueEnum;
use codespan::{Files, Span};
use colored::{self, Colorize};
use move_binary_format::{
    access::ModuleAccess,
    file_format::{CodeOffset, FunctionDefinitionIndex},
    CompiledModule,
};
use move_bytecode_source_map::source_map::SourceMap;
use move_command_line_common::files::FileHash;
use move_core_types::identifier::Identifier;
use move_ir_types::location::Loc;
use serde::Serialize;
use std::{
    collections::BTreeMap,
    fmt::{Display, Formatter},
    fs,
    io::{self, Write},
    path::Path,
    str::FromStr,
};

#[derive(Clone, Debug, Serialize)]
pub struct FunctionSourceCoverage {
    pub fn_is_native: bool,
    pub uncovered_locations: Vec<Loc>,
}

#[derive(Debug, Serialize)]
pub struct SourceCoverageBuilder<'a> {
    uncovered_locations: BTreeMap<Identifier, FunctionSourceCoverage>,
    source_map: &'a SourceMap,
}

#[derive(Debug, Serialize, Eq, PartialEq, Ord, PartialOrd)]
pub enum AbstractSegment {
    Bounded { start: u32, end: u32 },
    BoundedRight { end: u32 },
    BoundedLeft { start: u32 },
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
        module: &CompiledModule,
        coverage_map: &CoverageMap,
        source_map: &'a SourceMap,
    ) -> Self {
        let module_name = module.self_id();
        let unified_exec_map = coverage_map.to_unified_exec_map();
        let module_map = unified_exec_map
            .module_maps
            .get(&(*module_name.address(), module_name.name().to_owned()));

        let uncovered_locations: BTreeMap<Identifier, FunctionSourceCoverage> = module
            .function_defs()
            .iter()
            .enumerate()
            .flat_map(|(function_def_idx, function_def)| {
                let fn_handle = module.function_handle_at(function_def.function);
                let fn_name = module.identifier_at(fn_handle.name).to_owned();
                let function_def_idx = FunctionDefinitionIndex(function_def_idx as u16);

                // If the function summary doesn't exist then that function hasn't been called yet.
                let coverage = match &function_def.code {
                    None => Some(FunctionSourceCoverage {
                        fn_is_native: true,
                        uncovered_locations: Vec::new(),
                    }),
                    Some(code_unit) => {
                        module_map.map(|fn_map| match fn_map.function_maps.get(&fn_name) {
                            None => {
                                let function_map = source_map
                                    .get_function_source_map(function_def_idx)
                                    .unwrap();
                                let mut uncovered_locations =
                                    vec![function_map.definition_location];
                                uncovered_locations.extend(function_map.code_map.values());

                                FunctionSourceCoverage {
                                    fn_is_native: false,
                                    uncovered_locations,
                                }
                            },
                            Some(function_coverage) => {
                                let uncovered_locations: Vec<_> = (0..code_unit.code.len())
                                    .flat_map(|code_offset| {
                                        if !function_coverage.contains_key(&(code_offset as u64)) {
                                            Some(
                                                source_map
                                                    .get_code_location(
                                                        function_def_idx,
                                                        code_offset as CodeOffset,
                                                    )
                                                    .unwrap(),
                                            )
                                        } else {
                                            None
                                        }
                                    })
                                    .collect();
                                FunctionSourceCoverage {
                                    fn_is_native: false,
                                    uncovered_locations,
                                }
                            },
                        })
                    },
                };
                coverage.map(|x| (fn_name, x))
            })
            .collect();

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
        let file_hash = self.source_map.definition_location.file_hash();
        let mut files = Files::new();
        let file_id = files.add(file_path.as_os_str().to_os_string(), file_contents.clone());

        let mut uncovered_segments = BTreeMap::new();

        for (_key, fn_cov) in self.uncovered_locations.iter() {
            for span in merge_spans(file_hash, fn_cov.clone()).into_iter() {
                let start_loc = files.location(file_id, span.start()).unwrap();
                let end_loc = files.location(file_id, span.end()).unwrap();
                let start_line = start_loc.line.0;
                let end_line = end_loc.line.0;
                let segments = uncovered_segments
                    .entry(start_line)
                    .or_insert_with(Vec::new);
                if start_line == end_line {
                    let segment = AbstractSegment::Bounded {
                        start: start_loc.column.0,
                        end: end_loc.column.0,
                    };
                    // TODO: There is some issue with the source map where we have multiple spans
                    // from different functions. This can be seen in the source map for `Roles.move`
                    if !segments.contains(&segment) {
                        segments.push(segment);
                    }
                } else {
                    segments.push(AbstractSegment::BoundedLeft {
                        start: start_loc.column.0,
                    });
                    for i in start_line + 1..end_line {
                        let segment = uncovered_segments.entry(i).or_insert_with(Vec::new);
                        segment.push(AbstractSegment::BoundedLeft { start: 0 });
                    }
                    let last_segment = uncovered_segments.entry(end_line).or_insert_with(Vec::new);
                    last_segment.push(AbstractSegment::BoundedRight {
                        end: end_loc.column.0,
                    });
                }
            }
        }
        uncovered_segments.values_mut().for_each(|v| v.sort());

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

fn merge_spans(file_hash: FileHash, cov: FunctionSourceCoverage) -> Vec<Span> {
    if cov.uncovered_locations.is_empty() {
        return vec![];
    }

    let mut covs: Vec<_> = cov
        .uncovered_locations
        .iter()
        .filter(|loc| loc.file_hash() == file_hash)
        .map(|loc| Span::new(loc.start(), loc.end()))
        .collect();
    if covs.is_empty() {
        return vec![];
    }
    covs.sort();

    let mut unioned = Vec::new();
    let mut curr = covs.remove(0);

    for interval in covs {
        if curr.disjoint(interval) {
            unioned.push(curr);
            curr = interval;
        } else {
            curr = curr.merge(interval);
        }
    }

    unioned.push(curr);
    unioned
}
