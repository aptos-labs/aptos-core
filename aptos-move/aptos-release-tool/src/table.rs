// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Minimal, content-agnostic markdown table rendering.

/// Per-column horizontal alignment.
#[derive(Clone, Copy)]
pub enum Align {
    Left,
    Right,
}

/// Render a markdown table. `headers` defines the columns, `aligns` has one
/// entry per column, and every row must have `headers.len()` cells.
///
/// Cells are padded at the source level, so the raw `.md` is aligned and readable, not
/// just the rendered view.
pub fn render(headers: &[&str], aligns: &[Align], rows: &[Vec<String>]) -> String {
    assert_eq!(headers.len(), aligns.len(), "one alignment per column");

    // Column width = widest cell (header or body), min 3 so the separator
    // (`---`) stays well-formed for narrow or empty columns.
    let mut widths: Vec<usize> = headers.iter().map(|h| h.chars().count().max(3)).collect();
    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            widths[i] = widths[i].max(cell.chars().count());
        }
    }

    let mut out = String::new();
    push_row(
        &mut out,
        headers.iter().map(|h| h.to_string()),
        aligns,
        &widths,
    );
    push_separator(&mut out, aligns, &widths);
    for row in rows {
        push_row(&mut out, row.iter().cloned(), aligns, &widths);
    }
    out
}

fn push_row(
    out: &mut String,
    cells: impl Iterator<Item = String>,
    aligns: &[Align],
    widths: &[usize],
) {
    out.push('|');
    for ((cell, align), width) in cells.zip(aligns).zip(widths) {
        let w = *width;
        let cell = match align {
            Align::Left => format!("{:<width$}", cell, width = w),
            Align::Right => format!("{:>width$}", cell, width = w),
        };
        out.push_str(&format!(" {} |", cell));
    }
    out.push('\n');
}

fn push_separator(out: &mut String, aligns: &[Align], widths: &[usize]) {
    out.push('|');
    for (align, width) in aligns.iter().zip(widths) {
        let w = *width;
        // GFM alignment markers: `---` left (default), `--:` right.
        let bar = match align {
            Align::Left => "-".repeat(w),
            Align::Right => format!("{}:", "-".repeat(w - 1)),
        };
        out.push_str(&format!(" {} |", bar));
    }
    out.push('\n');
}
