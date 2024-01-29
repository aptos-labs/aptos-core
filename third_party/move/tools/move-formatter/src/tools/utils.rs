// Copyright (c) The BitsLab.MoveBit Contributors
// SPDX-License-Identifier: Apache-2.0

use lsp_types::{Location, Position};
use move_command_line_common::files::FileHash;
use move_ir_types::location::*;

use std::collections::HashMap;
use std::time::{Duration, Instant};
use std::{path::*, vec};

/// Double way mapping between FileHash and FilePath.
#[derive(Debug, Default)]
pub struct PathBufHashMap {
    path_2_hash: HashMap<PathBuf, FileHash>,
    hash_2_path: HashMap<FileHash, PathBuf>,
}

impl PathBufHashMap {
    pub fn update(&mut self, path: PathBuf, hash: FileHash) {
        if let Some(hash) = self.path_2_hash.get(&path) {
            self.hash_2_path.remove(hash);
        }
        self.path_2_hash.insert(path.clone(), hash);
        self.hash_2_path.insert(hash, path);
    }
}
/// A thin wrapper on `FileLineMapping`
/// Sometimes only handle one file.
#[derive(Debug, Default)]
#[repr(transparent)]
pub struct FileLineMappingOneFile {
    mapping: FileLineMapping,
}

impl From<FileLineMapping> for FileLineMappingOneFile {
    fn from(value: FileLineMapping) -> Self {
        Self { mapping: value }
    }
}

impl FileLineMappingOneFile {
    pub fn update(&mut self, content: &str) {
        self.mapping.update(Path::new(".").to_path_buf(), content);
    }
    pub(crate) fn translate(
        &self,
        start_index: ByteIndex,
        end_index: ByteIndex,
    ) -> Option<lsp_types::Range> {
        self.mapping
            .translate(&Path::new(".").to_path_buf(), start_index, end_index)
            .map(|x| x.mk_range())
    }
}

#[derive(Debug, Default)]
pub struct FileLineMapping {
    m: HashMap<PathBuf /* filepath */, Vec<ByteIndex>>,
}

impl FileLineMapping {
    pub fn update(&mut self, filepath: PathBuf, content: &str) {
        let mut v = vec![0];
        for (index, s) in content.as_bytes().iter().enumerate() {
            // TODO how to support windows \r\n
            if *s == 10 {
                // \n
                v.push((index + 1) as ByteIndex);
            }
        }
        if let Some(last) = content.as_bytes().last() {
            if *last != 10 {
                v.push((content.as_bytes().len()) as ByteIndex);
            }
        }
        self.m.insert(filepath, v);
    }

    pub fn translate(
        &self,
        filepath: &PathBuf,
        start_index: ByteIndex,
        mut end_index: ByteIndex,
    ) -> Option<FileRange> {
        if end_index < start_index {
            // maybe something goes wrong with syntax.rs
            // sometimes end_index < start_index.
            // this is a dummy fix.
            end_index = start_index;
        }
        let vec = self.m.get(filepath)?;
        let too_big = vec.last().map(|x| *x <= end_index).unwrap_or(false);
        if too_big {
            return None;
        }
        fn search(vec: &[ByteIndex], byte_index: ByteIndex) -> (u32, u32) {
            let mut index = bisection::bisect_left(vec, &byte_index);
            if vec[index] != byte_index {
                index -= 1;
            }
            (index as u32, byte_index - vec[index])
        }

        let (line_start, col_start) = search(&vec[..], start_index);
        let end = if let Some(t) = vec.get(line_start as usize + 1) {
            if *t > end_index {
                // Most case O(1) so we can have the same result but more fast.
                Some((line_start, end_index - vec[line_start as usize]))
            } else {
                None
            }
        } else {
            None
        };
        let (line_end, col_end) = end.unwrap_or(search(&vec[..], end_index));
        Some(FileRange {
            path: filepath.clone(),
            line_start,
            col_start,
            line_end,
            col_end,
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn file_mapping() {
        let filepath = PathBuf::from("test");

        let mut f = FileLineMapping::default();
        f.update(
            filepath.clone(),
            r#"123456
123456
abc        "#,
        );

        let r = f.translate(&filepath, 0, 2).unwrap();
        assert_eq!(
            r,
            FileRange {
                path: filepath.clone(),
                line_start: 0,
                line_end: 0,
                col_start: 0,
                col_end: 2
            }
        );

        let r = f.translate(&filepath, 9, 10).unwrap();
        assert_eq!(
            r,
            FileRange {
                path: filepath.clone(),
                line_start: 1,
                line_end: 1,
                col_start: 2,
                col_end: 3
            }
        );
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileRange {
    pub path: PathBuf,
    /// Start.
    pub line_start: u32,
    pub col_start: u32,

    /// End.
    pub line_end: u32,
    pub col_end: u32,
}

impl FileRange {
    pub fn mk_location(&self) -> lsp_types::Location {
        let range = self.mk_range();
        let uri = url::Url::from_file_path(self.path.as_path()).unwrap();
        lsp_types::Location::new(uri, range)
    }
    pub fn mk_range(&self) -> lsp_types::Range {
        lsp_types::Range {
            start: lsp_types::Position {
                line: self.line_start,
                character: self.col_start,
            },
            end: Position {
                line: self.line_end,
                character: self.col_end,
            },
        }
    }
}

impl std::fmt::Display for FileRange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:?}:({},{}):({},{})",
            self.path.as_path(),
            self.line_start,
            self.col_start,
            self.line_end,
            self.col_end
        )
    }
}

/// Path concat from
pub fn path_concat(p1: &Path, p2: &Path) -> PathBuf {
    let p2: Vec<_> = p2.components().collect();
    let is_abs = matches!(
        p2.get(0).unwrap(),
        Component::RootDir | Component::Prefix(_)
    );
    let mut p1: Vec<_> = p1.components().collect();
    normal_path_components(if is_abs {
        &p2
    } else {
        {
            p1.extend(p2);
            &p1
        }
    })
}

/// concat Move.toml file.
pub fn path_concat_move_toml(p1: &Path, p2: &Path) -> PathBuf {
    let p1_is_move_toml = match p1.to_str() {
        Some(x) => x.ends_with(PROJECT_FILE_NAME),
        None => false,
    };
    if p1_is_move_toml {
        let mut p1 = p1.to_path_buf();
        p1.pop();
        path_concat(p1.as_path(), p2)
    } else {
        path_concat(p1, p2)
    }
}

pub fn normal_path_components(x: &Vec<Component<'_>>) -> PathBuf {
    let mut ret = PathBuf::new();
    for v in x {
        match v {
            Component::Prefix(x) => ret.push(x.as_os_str()),
            Component::RootDir => ret.push("/"),
            Component::CurDir => {}
            Component::ParentDir => {
                let _ = ret.pop();
            }
            Component::Normal(x) => ret.push(*x),
        }
    }
    if ret.to_str().unwrap() == "" {
        ret.push(".")
    }
    ret
}

use lsp_types::Range;

#[derive(Clone, serde::Serialize)]
pub struct PathAndRange {
    range: Range,
    fpath: String,
}

impl From<&Location> for PathAndRange {
    fn from(value: &Location) -> Self {
        Self {
            range: value.range,
            fpath: value
                .uri
                .to_file_path()
                .unwrap()
                .to_str()
                .unwrap()
                .to_string(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Timer {
    Disabled,
    Initialized(Instant),
    DoneParsing(Instant, Instant),
    DoneFormatting(Instant, Instant, Instant),
}

impl Timer {
    pub fn start() -> Timer {
        if cfg!(target_arch = "wasm32") {
            Timer::Disabled
        } else {
            Timer::Initialized(Instant::now())
        }
    }
    pub fn done_parsing(self) -> Self {
        match self {
            Timer::Disabled => Timer::Disabled,
            Timer::Initialized(init_time) => Timer::DoneParsing(init_time, Instant::now()),
            _ => panic!("Timer can only transition to DoneParsing from Initialized state"),
        }
    }

    pub fn done_formatting(self) -> Self {
        match self {
            Timer::Disabled => Timer::Disabled,
            Timer::DoneParsing(init_time, parse_time) => {
                Timer::DoneFormatting(init_time, parse_time, Instant::now())
            }
            _ => panic!("Timer can only transition to DoneFormatting from DoneParsing state"),
        }
    }

    /// Returns the time it took to parse the source files in seconds.
    pub fn get_parse_time(&self) -> f32 {
        match *self {
            Timer::Disabled => panic!("this platform cannot time execution"),
            Timer::DoneParsing(init, parse_time) | Timer::DoneFormatting(init, parse_time, _) => {
                // This should never underflow since `Instant::now()` guarantees monotonicity.
                Self::duration_to_f32(parse_time.duration_since(init))
            }
            Timer::Initialized(..) => unreachable!(),
        }
    }

    /// Returns the time it took to go from the parsed AST to the formatted output. Parsing time is
    /// not included.
    pub fn get_format_time(&self) -> f32 {
        match *self {
            Timer::Disabled => panic!("this platform cannot time execution"),
            Timer::DoneFormatting(_init, parse_time, format_time) => {
                Self::duration_to_f32(format_time.duration_since(parse_time))
            }
            Timer::DoneParsing(..) | Timer::Initialized(..) => unreachable!(),
        }
    }

    pub fn duration_to_f32(d: Duration) -> f32 {
        d.as_secs() as f32 + d.subsec_nanos() as f32 / 1_000_000_000f32
    }
}

pub fn mk_result_filepath(x: &Path) -> PathBuf {
    let mut x = x.to_path_buf();
    let b = x
        .components()
        .last()
        .and_then(|x| x.as_os_str().to_str())
        .unwrap()
        .to_string();
    let index = b.as_str().rfind('.').unwrap();
    x.pop();
    let mut ret = x.clone();
    ret.push(format!("{}{}", &b.as_str()[0..index], ".fmt.out"));
    ret
}

pub const PROJECT_FILE_NAME: &str = "Move.toml";

#[cfg(not(target_os = "windows"))]
pub fn cpu_pprof(_seconds: u64) {
    use std::fs::File;
    use std::str::FromStr;
    let guard = pprof::ProfilerGuardBuilder::default()
        .frequency(1000)
        .blocklist(&["libc", "libgcc", "pthread", "vdso"])
        .build()
        .unwrap();
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::new(_seconds, 0));
        match guard.report().build() {
            Result::Ok(report) => {
                // let mut tmp = std::env::temp_dir();
                let mut tmp = PathBuf::from_str("/Users/yuyang/.movefmt").unwrap();

                tmp.push("movefmt-flamegraph.svg");
                let file = File::create(tmp.clone()).unwrap();
                report.flamegraph(file).unwrap();
                tracing::debug!("pprof file at {:?}", tmp.as_path());
            }
            Result::Err(e) => {
                tracing::error!("build report failed,err:{}", e);
            }
        };
    });
}
#[cfg(target_os = "windows")]
pub fn cpu_pprof(_seconds: u64) {
    log::error!("Can't run pprof in Windows");
}
