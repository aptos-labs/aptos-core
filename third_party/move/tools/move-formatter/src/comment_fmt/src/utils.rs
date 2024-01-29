use unicode_width::UnicodeWidthStr;
use crate::comment::{filter_normal_code, FullCodeCharKind, LineClasses};
use crate::config::Config;
use crate::shape::{Indent, Shape};

#[inline]
pub fn is_single_line(s: &str) -> bool {
    !s.chars().any(|c| c == '\n')
}

#[inline]
pub fn last_line_contains_single_line_comment(s: &str) -> bool {
    s.lines().last().map_or(false, |l| l.contains("//"))
}

#[inline]
pub fn is_attributes_extendable(attrs_str: &str) -> bool {
    !attrs_str.contains('\n') && !last_line_contains_single_line_comment(attrs_str)
}

/// The width of the first line in s.
#[inline]
pub fn first_line_width(s: &str) -> usize {
    unicode_str_width(s.splitn(2, '\n').next().unwrap_or(""))
}

/// The width of the last line in s.
#[inline]
pub fn last_line_width(s: &str) -> usize {
    unicode_str_width(s.rsplitn(2, '\n').next().unwrap_or(""))
}

pub fn count_newlines(input: &str) -> usize {
    // Using bytes to omit UTF-8 decoding
    input.as_bytes().iter().filter(|&b| *b == b'\n').count()
}

// Wraps String in an Option. Returns Some when the string adheres to the
// Rewrite constraints defined for the Rewrite trait and None otherwise.
pub fn wrap_str(s: String, max_width: usize, shape: Shape) -> Option<String> {
    if filtered_str_fits(&s, max_width, shape) {
        Some(s)
    } else {
        None
    }
}

pub fn filtered_str_fits(snippet: &str, max_width: usize, shape: Shape) -> bool {
    let snippet = &filter_normal_code(snippet);
    if !snippet.is_empty() {
        // First line must fits with `shape.width`.
        if first_line_width(snippet) > shape.width {
            return false;
        }
        // If the snippet does not include newline, we are done.
        if is_single_line(snippet) {
            return true;
        }
        // The other lines must fit within the maximum width.
        if snippet
            .lines()
            .skip(1)
            .any(|line| unicode_str_width(line) > max_width)
        {
            return false;
        }
        // A special check for the last line, since the caller may
        // place trailing characters on this line.
        if last_line_width(snippet) > shape.used_width() + shape.width {
            return false;
        }
    }
    true
}

/// Indent each line according to the specified `indent`.
pub fn trim_left_preserve_layout(
    orig: &str,
    indent: Indent,
    config: &Config,
) -> Option<String> {
    let mut lines = LineClasses::new(orig);
    let first_line = lines.next().map(|(_, s)| s.trim_end().to_owned())?;
    let mut trimmed_lines = Vec::with_capacity(16);

    let mut veto_trim = false;
    let min_prefix_space_width = lines
        .filter_map(|(kind, line)| {
            let mut trimmed = true;
            let prefix_space_width = if is_empty_line(&line) {
                None
            } else {
                Some(get_prefix_space_width(config, &line))
            };

            // just InString{Commented} in order to allow the start of a string to be indented
            let new_veto_trim_value = (kind == FullCodeCharKind::InString
                || kind == FullCodeCharKind::InStringCommented)
                && !line.ends_with('\\');
            let line = if veto_trim || new_veto_trim_value {
                veto_trim = new_veto_trim_value;
                trimmed = false;
                line
            } else {
                line.trim().to_owned()
            };
            trimmed_lines.push((trimmed, line, prefix_space_width));

            // Because there is a veto against trimming and indenting lines within a string,
            // such lines should not be taken into account when computing the minimum.
            match kind {
                FullCodeCharKind::InStringCommented | FullCodeCharKind::EndStringCommented => None,
                FullCodeCharKind::InString | FullCodeCharKind::EndString => None,
                _ => prefix_space_width,
            }
        })
        .min()?;

    Some(
        first_line
            + "\n"
            + &trimmed_lines
                .iter()
                .map(
                    |&(trimmed, ref line, prefix_space_width)| match prefix_space_width {
                        _ if !trimmed => line.to_owned(),
                        Some(original_indent_width) => {
                            let new_indent_width = indent.width()
                                + original_indent_width.saturating_sub(min_prefix_space_width);
                            let new_indent = Indent::from_width(config, new_indent_width);
                            format!("{}{}", new_indent.to_string(config), line)
                        }
                        None => String::new(),
                    },
                )
                .collect::<Vec<_>>()
                .join("\n"),
    )
}

pub fn is_empty_line(s: &str) -> bool {
    s.is_empty() || s.chars().all(char::is_whitespace)
}

fn get_prefix_space_width(config: &Config, s: &str) -> usize {
    let mut width = 0;
    for c in s.chars() {
        match c {
            ' ' => width += 1,
            '\t' => width += config.tab_spaces(),
            _ => return width,
        }
    }
    width
}

pub trait NodeIdExt {
    fn root() -> Self;
}

pub fn unicode_str_width(s: &str) -> usize {
    s.width()
}
