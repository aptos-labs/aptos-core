// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn count(n: u32) -> u32 {
    let mut i = 0;
    while i < n {
        i += 1;
    }
    i
}

pub fn sum_to(n: u32) -> u32 {
    let mut i = 0;
    let mut s = 0;
    while i < n {
        i += 1;
        s += 1000;
    }
    s
}

pub fn search(limit: u32, target: u32) -> u32 {
    let mut i = 0;
    while i < limit {
        if i == target {
            return i;
        }
        i += 1;
    }
    limit
}

pub fn nested(rows: u32, columns: u32) -> u32 {
    let mut total = 0;
    let mut row = 0;
    while row < rows {
        let mut column = 0;
        while column < columns {
            total += 1;
            column += 1;
        }
        row += 1;
    }
    total
}

pub fn break_outer(rows: u32) -> u32 {
    let mut row = 0;
    'outer: while row < rows {
        let mut column = 0;
        while column < rows {
            if column > row {
                break 'outer;
            }
            column += 1;
        }
        row += 1;
    }
    row
}

pub fn skip_rows(rows: u32, columns: u32) -> u32 {
    let mut count = 0;
    let mut row = 0;
    'outer: while row < rows {
        row += 1;
        let mut column = 0;
        while column < columns {
            column += 1;
            if column == row {
                continue 'outer;
            }
            count += 1;
        }
        count += 2;
    }
    count
}

pub fn classify(n: u32, limit: u32) -> u32 {
    let mut index = 0;
    let kind = loop {
        if index >= n {
            break 0;
        }
        if index > limit {
            break 1;
        }
        index += 1;
    };
    let mut total = kind;
    while total < limit {
        total += 1;
    }
    total
}

pub fn branches(flag: bool, n: u32) -> u32 {
    let mut i = 0;
    if flag {
        while i < n {
            i += 1;
        }
    } else {
        while i < n {
            i += 2;
        }
    }
    i
}

pub fn sum(values: &[u32]) -> u64 {
    let mut total = 0u64;
    let mut index = 0;
    while index < values.len() {
        total += values[index] as u64;
        index += 1;
    }
    total
}

pub fn position(values: &[u32], target: u32) -> usize {
    let mut index = 0;
    while index < values.len() {
        if values[index] == target {
            return index;
        }
        index += 1;
    }
    values.len()
}
