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
pub struct FunctionSourceCoverage2 {
    /// Is this a native function?
    /// If so, then `uncovered_locations` is empty.
    pub fn_is_native: bool,

    /// List of source locations in the function that were covered.
    pub covered_locations: Vec<Loc>,

    /// List of all source locations in the function.
    pub uncovered_locations: Vec<Loc>,
}

fn minimize_locations(mut locs: Vec<Loc>) -> Vec<Loc> {
    locs.sort();
    let mut result = vec![];
    let mut locs_iter = locs.into_iter();
    if let Some(mut current_loc) = locs_iter.next() {
        for next_loc in locs_iter {
            let loc_tmp = current_loc;
            if !current_loc.try_merge(&next_loc) {
                eprintln!("Not merging {:?} with {:?}", loc_tmp, next_loc);
                result.push(current_loc);
                current_loc = next_loc;
            }
        }
        result.push(current_loc);
    }
    result
}

// locs1 and locs2 should each be sorted or this won't work.
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

/// Builder for the source code coverage.
#[derive(Debug, Serialize)]
pub struct SourceCoverageBuilder<'a> {
    /// Mapping from function name to the source-level uncovered locations for that function.
    pub uncovered_locations: Vec<Loc>,
    pub covered_locations: Vec<Loc>,

    source_map: &'a SourceMap,
}

#[derive(Debug, Serialize, Eq, PartialEq)]
pub enum AbstractSegment {
    Bounded {
        is_covered: bool,
        start: u32,
        end: u32,
    },
    BoundedRight {
        is_covered: bool,
        end: u32,
    },
    BoundedLeft {
        is_covered: bool,
        start: u32,
    },
    // Unbounded {},
}

impl AbstractSegment {
    fn is_covered(&self) -> bool {
        use AbstractSegment::*;
        match self {
            Bounded { is_covered, .. }
            | BoundedRight { is_covered, .. }
            | BoundedLeft { is_covered, .. } => *is_covered,
        }
    }

    fn get_start(&self) -> u32 {
        use AbstractSegment::*;
        match self {
            Bounded { start, .. } => *start,
            BoundedRight { .. } => 0u32,
            BoundedLeft { start, .. } => *start,
            // Unbounded {} => 0u32,
        }
    }

    fn get_number(&self) -> u8 {
        use AbstractSegment::*;
        match self {
            Bounded { .. } => 0,
            BoundedRight { .. } => 1,
            BoundedLeft { .. } => 2,
            // Unbounded {} => 3,
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
    NotCounted(String),
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
        packages: Vec<(&CompiledModule, &'a SourceMap)>,
    ) -> Self {
        eprintln!("coverage_map is {:#?}", coverage_map);
        eprintln!("source_map is {:#?}", source_map);
        eprintln!("module is {:#?}", module);

        let module_loc = source_map.definition_location;

        let module_name = module.self_id();
        let unified_exec_map = coverage_map.to_unified_exec_map();
        let module_map = unified_exec_map
            .module_maps
            .get(&(*module_name.address(), module_name.name().to_owned()));

        eprintln!("unified_exec_map is {:#?}", &unified_exec_map);
        eprintln!("module_map is {:#?}", &module_map);

        eprintln!("Computing covered_locations");

        let mut fun_coverage: Vec<FunctionSourceCoverage2> = Vec::new();
        for (module, source_map) in packages.iter() {
            let module_name = module.self_id();
            let module_map = unified_exec_map
                .module_maps
                .get(&(*module_name.address(), module_name.name().to_owned()));
            if let Some(module_map) = module_map {
                for (function_def_idx, function_def) in module.function_defs().iter().enumerate() {
                    let fn_handle = module.function_handle_at(function_def.function);
                    let fn_name = module.identifier_at(fn_handle.name).to_owned();
                    let function_def_idx = FunctionDefinitionIndex(function_def_idx as u16);
                    let function_map = source_map
                        .get_function_source_map(function_def_idx)
                        .unwrap();
                    let _function_loc = function_map.definition_location;
                    let function_covered_locations: FunctionSourceCoverage2 = match &function_def
                        .code
                    {
                        None => FunctionSourceCoverage2 {
                            fn_is_native: true,
                            covered_locations: vec![],
                            uncovered_locations: vec![],
                        },
                        Some(code_unit) => match module_map.function_maps.get(&fn_name) {
                            None => {
                                let locations: Vec<_> = (0..code_unit.code.len())
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
                                    .collect();
                                FunctionSourceCoverage2 {
                                    fn_is_native: false,
                                    covered_locations: vec![],
                                    uncovered_locations: locations,
                                }
                            },
                            Some(function_coverage) => {
                                let (fun_cov, fun_uncov): (Vec<_>, Vec<_>) = (0..code_unit
                                    .code
                                    .len())
                                    .filter_map(|code_offset| {
                                        if let Ok(loc) = source_map.get_code_location(
                                            function_def_idx,
                                            code_offset as CodeOffset,
                                        ) {
                                            if function_coverage.contains_key(&(code_offset as u64))
                                            {
                                                Some((loc, true))
                                            } else {
                                                Some((loc, false))
                                            }
                                        } else {
                                            None
                                        }
                                    })
                                    .partition(|(_loc, test)| *test);

                                let covered_locations: Vec<_> = fun_cov
                                    .iter()
                                    .filter_map(
                                        |(loc, covered)| if *covered { Some(*loc) } else { None },
                                    )
                                    .collect();
                                let uncovered_locations: Vec<_> = fun_uncov
                                    .iter()
                                    .filter_map(
                                        |(loc, covered)| if !*covered { Some(*loc) } else { None },
                                    )
                                    .collect();

                                FunctionSourceCoverage2 {
                                    fn_is_native: false,
                                    covered_locations,
                                    uncovered_locations,
                                }
                            },
                        },
                    };
                    fun_coverage.push(function_covered_locations);
                }
            }
        }
        eprintln!("fun_coverage is {:#?}", fun_coverage);

        // Filter locations for this module and build 2 sets: covered and uncovered locations
        // Note that Move 1 compiler sets module_loc = location of symbol in definition, so if
        // there are multiple modules in one file we may leave others in the picture.
        eprintln!("module_loc is {:?}", module_loc);
        let module_file_hash = module_loc.file_hash();
        eprintln!("module_file_hash is {:?}", module_file_hash);

        let (covered, uncovered): (BTreeSet<Loc>, BTreeSet<Loc>) = fun_coverage
            .iter()
            .map(
                |FunctionSourceCoverage2 {
                     covered_locations,
                     uncovered_locations,
                     ..
                 }| {
                    let cov: BTreeSet<_> = covered_locations
                        .iter()
                        .filter(|loc| loc.file_hash() == module_file_hash)
                        .cloned()
                        .collect();
                    let uncov: BTreeSet<_> = uncovered_locations
                        .iter()
                        .filter(|loc| loc.file_hash() == module_file_hash)
                        .cloned()
                        .collect();
                    (cov, uncov)
                },
            )
            .reduce(|(c1, u1), (c2, u2)| {
                (
                    c1.union(&c2).cloned().collect(),
                    u1.union(&u2).cloned().collect(),
                )
            })
            .unwrap_or_else(|| (BTreeSet::new(), BTreeSet::new()));

        eprintln!("covered 0 is {:#?}", covered);
        eprintln!("uncovered 0 is {:#?}", uncovered);

        // Some locations contain others, but should not subsume them, e.g.:
        //     31:  if (p) {
        //     32:     ...
        //     33:  } else {
        //     34:     ...
        //     35   }
        // May have 3 corresponding bytecodes, with locations:
        //     L1: 31-35
        //     L2: 32
        //     L3: 34
        // If L1 is covered but L3 is not, then we want to subtract
        // L3 from the covered region.
        //
        // OTOH, we may see L3 in multiple runs, once covered and once
        // not.  So we need to
        //   (1) subtract identical covered locations from uncovered locations
        //   (2) subtract uncovered locations from any covered locations they overlap
        //   (3) subtract covered locations from any uncovered lcoations they overlap

        // (1)
        let uncovered: BTreeSet<_> = uncovered.difference(&covered).cloned().collect();

        eprintln!("uncovered 1 is {:#?}", uncovered);

        // (2)
        let covered: Vec<_> = subtract_locations(covered, &uncovered);

        eprintln!("covered 2a is {:#?}", covered);

        let covered: Vec<_> = minimize_locations(covered);

        eprintln!("covered 2b is {:#?}", covered);

        let uncovered: Vec<_> = uncovered.into_iter().collect();
        eprintln!("uncovered 2a is {:#?}", uncovered);

        let uncovered: Vec<_> = minimize_locations(uncovered);

        eprintln!("uncovered 2b is {:#?}", uncovered);

        let uncovered: BTreeSet<_> = uncovered.into_iter().collect();
        let covered: BTreeSet<_> = covered.into_iter().collect();

        // (3)
        let uncovered: Vec<_> = subtract_locations(uncovered, &covered);
        let covered: Vec<_> = covered.into_iter().collect();

        eprintln!("uncovered 3 is {:#?}", uncovered);
        eprintln!("uncovered is {:#?}", uncovered);

        Self {
            uncovered_locations: uncovered,
            covered_locations: covered,
            source_map,
        }
    }

    // Converts a (sorted) list of non-overlapping `Loc` into a
    // map from line number to set of `AsbtractSegment` for each line
    // of the file `file_id` in fileset `files`.
    fn locs_to_segments(
        &self,
        files: &mut Files<String>,
        file_id: FileId,
        spans: Vec<Span>,
        is_covered: bool,
    ) -> BTreeMap<u32, Vec<AbstractSegment>> {
        let mut segments = BTreeMap::new();

        for span in spans.into_iter() {
            let start_loc = files.location(file_id, span.start()).unwrap();
            let end_loc = files.location(file_id, span.end()).unwrap();
            let start_line = start_loc.line.0;
            let end_line = end_loc.line.0;
            eprintln!(
                "Looking at span = ({}, {}), line = ({}, {})",
                span.start(),
                span.end(),
                start_line,
                end_line
            );

            let line_segments = segments.entry(start_line).or_insert_with(Vec::new);
            if start_line == end_line {
                let segment = AbstractSegment::Bounded {
                    start: start_loc.column.0,
                    end: end_loc.column.0,
                    is_covered,
                };
                if !line_segments.contains(&segment) {
                    line_segments.push(segment);
                }
            } else {
                line_segments.push(AbstractSegment::BoundedLeft {
                    start: start_loc.column.0,
                    is_covered,
                });
                for i in start_line + 1..end_line {
                    let line_segments = segments.entry(i).or_insert_with(Vec::new);
                    line_segments.push(AbstractSegment::BoundedLeft {
                        start: 0,
                        is_covered,
                    });
                }
                let last_line_segments = segments.entry(end_line).or_insert_with(Vec::new);
                last_line_segments.push(AbstractSegment::BoundedRight {
                    end: end_loc.column.0,
                    is_covered,
                });
            }
        }
        segments.values_mut().for_each(|v| v.sort());
        segments
    }

    pub fn compute_source_coverage(&self, file_path: &Path) -> SourceCoverage {
        eprintln!("Reading file {}", file_path.display());
        let file_contents = fs::read_to_string(file_path).unwrap();
        assert!(
            self.source_map.check(&file_contents),
            "File contents {} out of sync with source map",
            file_path.display()
        );
        let mut files = Files::new();
        let file_id = files.add(file_path.as_os_str().to_os_string(), file_contents.clone());

        let covs: Vec<_> = self
            .covered_locations
            .iter()
            .map(|loc| Span::new(loc.start(), loc.end()))
            .collect();
        let uncovs: Vec<_> = self
            .uncovered_locations
            .iter()
            .map(|loc| Span::new(loc.start(), loc.end()))
            .collect();

        let mut uncovered_segments = self.locs_to_segments(&mut files, file_id, uncovs, false);
        let mut covered_segments = self.locs_to_segments(&mut files, file_id, covs, true);

        covered_segments.values_mut().for_each(|v| v.sort());
        for (key, value) in covered_segments.iter_mut() {
            match uncovered_segments.get_mut(key) {
                None => {},
                Some(value2) => {
                    value.append(value2);
                    value.sort();
                },
            }
        }

        let mut annotated_lines = Vec::new();
        for (line_number, mut line) in file_contents.lines().map(|x| x.to_owned()).enumerate() {
            eprintln!("looking at {}, {}", line_number, line);
            let line_no_u32 = line_number as u32;
            match covered_segments.get(&line_no_u32) {
                None => annotated_lines.push(vec![StringSegment::NotCounted(line)]),
                Some(segments) => {
                    eprintln!(
                        "Segments for line {} are: {}",
                        line_number,
                        segments
                            .iter()
                            .map(|seg| format!("{:?}", seg).to_string())
                            .collect::<Vec<_>>()
                            .join(", ")
                    );
                    let mut line_acc = Vec::new();
                    let mut cursor = 0;
                    // Note: segments are already pre-sorted by construction so don't need to be
                    // resorted.
                    // let mut segment_start: Option<u32> = None;
                    // let mut segment_end: Option<u32> = None;
                    for segment in segments {
                        match segment {
                            AbstractSegment::Bounded {
                                start,
                                end,
                                is_covered,
                            } => {
                                eprintln!("Bounded {}, {}, cursor = {}", start, end, cursor);

                                let length = end - start;
                                let (before, after) = line.split_at((start - cursor) as usize);
                                let (covered, rest) = after.split_at(length as usize);
                                line_acc.push(StringSegment::NotCounted(before.to_string()));
                                line_acc.push(
                                    if *is_covered {
                                        StringSegment::Covered(covered.to_string())
                                    } else {
                                        StringSegment::Uncovered(covered.to_string())
                                    },
                                );
                                line = rest.to_string();
                                cursor = *end;
                            },
                            AbstractSegment::BoundedRight { end, is_covered } => {
                                eprintln!("BoundedRight {}, cursor = {}", end, cursor);
                                let (uncovered, rest) = line.split_at((end - cursor) as usize);
                                line_acc.push(
                                    if *is_covered {
                                        StringSegment::Covered(uncovered.to_string())
                                    } else {
                                        StringSegment::Uncovered(uncovered.to_string())
                                    },
                                );
                                line = rest.to_string();
                                cursor = *end;
                            },
                            AbstractSegment::BoundedLeft { start, is_covered } => {
                                eprintln!("BoundedLeft {}, cursor = {}", start, cursor);
                                let (before, after) = line.split_at((start - cursor) as usize);
                                line_acc.push(StringSegment::NotCounted(before.to_string()));
                                line_acc.push(
                                    if *is_covered {
                                        StringSegment::Covered(after.to_string())
                                    } else {
                                        StringSegment::Uncovered(after.to_string())
                                    },
                                );
                                line = "".to_string();
                                cursor = 0;
                            },
                        }
                    }
                    if !line.is_empty() {
                        line_acc.push(StringSegment::NotCounted(line))
                    }
                    annotated_lines.push(line_acc)
                },
            };
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
                    "Code coverage per line of code:\n  {} indicates the line is covered during execution\n  {} indicates the line is executable but NOT fully covered during execution\n  {} indicates the line is either test code or is not executable\nSource code follows:\n",
                    "+".to_string().green(),
                    "-".to_string().bold().red(),
                    " ".to_string(),
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
                let has_covered = line
                    .iter()
                    .any(|string_segment| matches!(string_segment, StringSegment::Covered(_)));
                write!(
                    output_writer,
                    "{} ",
                    if has_uncovered {
                        "-".to_string().red()
                    } else if has_covered {
                        "+".to_string().green()
                    } else {
                        " ".to_string().normal()
                    }
                )?;
            }
            for string_segment in line.iter() {
                match string_segment {
                    StringSegment::Covered(s) => write!(output_writer, "{}", s.green())?,
                    StringSegment::Uncovered(s) => write!(output_writer, "{}", s.bold().red())?,
                    StringSegment::NotCounted(s) => write!(output_writer, "{}", s.normal())?,
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

    let mut last_loc: Option<Loc> = None;
    for loc in cov.uncovered_locations.iter() {
        if loc.file_hash() != file_hash {
            if let Some(last_loc) = last_loc {
                eprintln!(
                    "dropping loc ({}, {}, {}) after ({}, {}, {})",
                    loc.file_hash(),
                    loc.start(),
                    loc.end(),
                    last_loc.file_hash(),
                    loc.start(),
                    loc.end(),
                );
            }
            last_loc = None;
        } else {
            last_loc = Some(*loc);
        }
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
