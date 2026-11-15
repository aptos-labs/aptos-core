// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn first_index(values: &[u32], target: u32) -> u32 {
    let mut index = 0;
    while (index as usize) < values.len() {
        if values[index as usize] == target {
            return index;
        }
        index += 1;
    }
    index
}

pub fn two_returns(values: &[u32]) -> u32 {
    let mut index = 0;
    while (index as usize) < values.len() {
        if values[index as usize] == 0 {
            return 0;
        }
        if values[index as usize] == 1 {
            return 1;
        }
        index += 1;
    }
    2
}

pub fn nested(rows: u32, columns: u32) -> u32 {
    let mut row = 0;
    while row < rows {
        let mut column = 0;
        while column < columns {
            if row == column {
                return row;
            }
            column += 1;
        }
        row += 1;
    }
    rows
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

pub fn classify(values: &[u32], limit: u32) -> u32 {
    let mut index = 0;
    let kind = loop {
        if (index as usize) >= values.len() {
            break 0;
        }
        if values[index as usize] > limit {
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
