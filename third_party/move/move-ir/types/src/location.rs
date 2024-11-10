// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use move_command_line_common::files::FileHash;
use serde::{Deserialize, Serialize};
use std::{
    cmp::Ordering,
    fmt,
    hash::{Hash, Hasher},
    ops::Range,
};

//**************************************************************************************************
// Loc
//**************************************************************************************************

/// An index into a file.
/// Much like the `codespan` crate, a `u32` is used here to for space efficiency.
/// However, this assumes no file is larger than 4GB, so this might become a `usize` in the future
/// if the space concerns turn out to not be an issue.
pub type ByteIndex = u32;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize, Hash)]
/// The `Loc` struct is used to define a location in a file; where the file is considered to be a
/// vector of bytes, and the range for a given `Loc` is defined by start and end index into that
/// byte vector
pub struct Loc {
    /// The file the location points to
    file_hash: FileHash,
    /// The start byte index into file
    start: ByteIndex,
    /// The end byte index into file
    end: ByteIndex,
}

impl Loc {
    pub fn new(file_hash: FileHash, start: ByteIndex, end: ByteIndex) -> Loc {
        assert!(start <= end);
        Loc {
            file_hash,
            start,
            end,
        }
    }

    pub fn file_hash(self) -> FileHash {
        self.file_hash
    }

    pub fn start(self) -> ByteIndex {
        self.start
    }

    pub fn end(self) -> ByteIndex {
        self.end
    }

    pub fn usize_range(self) -> Range<usize> {
        Range {
            start: self.start as usize,
            end: self.end as usize,
        }
    }

    pub fn contains(&self, other: &Self) -> bool {
        if self.file_hash != other.file_hash {
            false
        } else {
            self.start <= other.start && other.end <= self.end
        }
    }

    pub fn overlaps(&self, other: &Self) -> bool {
        if self.file_hash != other.file_hash {
            false
        } else {
            // [a, b] overlaps? [c, d]
            // a <= b
            // c <= d
            other.start <= self.end && self.start <= other.end
            // c <= b
            // a <= d
            // One of these:
            // [a <= (c <= b] <= d)
            // (c <= [a <= b] <= d)
            // (c <= [a <= d) <= b]
        }
    }

    pub fn overlaps_or_abuts(&self, other: &Self) -> bool {
        if self.file_hash != other.file_hash {
            false
        } else {
            // [a, b] overlaps? [c, d]
            // a <= b
            // c <= d
            other.start <= self.end + 1 && self.start <= other.end + 1
            // c <= b + 1
            // a <= d + 1
            // One of these:
            // a <= c <= b+1 <= d+1
            // c <= a <= b+1 <= d+1
            // c <= a <= d+1 <= b+1
        }
    }

    pub fn try_merge(&mut self, other: &Self) -> bool {
        if self.overlaps_or_abuts(other) {
            self.start = std::cmp::min(self.start, other.start);
            self.end = std::cmp::max(self.end, other.end);
            true
        } else {
            false
        }
    }

    // if other overlaps with this, then subtract it out.
    pub fn subtract(self, other: &Self) -> Vec<Loc> {
        if !self.overlaps(other) {
            vec![self]
        } else {
            if other.start <= self.start {
                if self.end <= other.end {
                    vec![]
                } else {
                    vec![Loc {
                        start: other.end + 1,
                        ..self
                    }]
                }
            } else {
                if self.end <= other.end {
                    vec![Loc {
                        end: other.start - 1,
                        ..self
                    }]
                } else {
                    vec![
                        Loc {
                            end: other.start - 1,
                            ..self
                        },
                        Loc {
                            start: other.end + 1,
                            ..self
                        },
                    ]
                }
            }
        }
    }
}

impl PartialOrd for Loc {
    fn partial_cmp(&self, other: &Loc) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Loc {
    fn cmp(&self, other: &Loc) -> Ordering {
        let file_ord = self.file_hash.cmp(&other.file_hash);
        if file_ord != Ordering::Equal {
            return file_ord;
        }

        let start_ord = self.start.cmp(&other.start);
        if start_ord != Ordering::Equal {
            return start_ord;
        }

        self.end.cmp(&other.end)
    }
}

//**************************************************************************************************
// Spanned
//**************************************************************************************************

#[derive(Copy, Clone)]
pub struct Spanned<T> {
    pub loc: Loc,
    pub value: T,
}

impl<T> Spanned<T> {
    pub fn new(loc: Loc, value: T) -> Spanned<T> {
        Spanned { loc, value }
    }

    pub fn unsafe_no_loc(value: T) -> Spanned<T> {
        Spanned {
            value,
            loc: Loc::new(FileHash::empty(), 0, 0),
        }
    }
}

impl<T: PartialEq> PartialEq for Spanned<T> {
    fn eq(&self, other: &Spanned<T>) -> bool {
        self.value == other.value
    }
}

impl<T: Eq> Eq for Spanned<T> {}

impl<T: Hash> Hash for Spanned<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.value.hash(state);
    }
}

impl<T: PartialOrd> PartialOrd for Spanned<T> {
    fn partial_cmp(&self, other: &Spanned<T>) -> Option<Ordering> {
        self.value.partial_cmp(&other.value)
    }
}

impl<T: Ord> Ord for Spanned<T> {
    fn cmp(&self, other: &Spanned<T>) -> Ordering {
        self.value.cmp(&other.value)
    }
}

impl<T: fmt::Display> fmt::Display for Spanned<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", &self.value)
    }
}

impl<T: fmt::Debug> fmt::Debug for Spanned<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", &self.value)
    }
}

/// Function used to have nearly tuple-like syntax for creating a Spanned
pub const fn sp<T>(loc: Loc, value: T) -> Spanned<T> {
    Spanned { loc, value }
}

/// Macro used to create a tuple-like pattern match for Spanned
#[macro_export]
macro_rules! sp {
    (_, $value:pat) => {
        $crate::location::Spanned { value: $value, .. }
    };
    ($loc:pat,_) => {
        $crate::location::Spanned { loc: $loc, .. }
    };
    ($loc:pat, $value:pat) => {
        $crate::location::Spanned {
            loc: $loc,
            value: $value,
        }
    };
}
