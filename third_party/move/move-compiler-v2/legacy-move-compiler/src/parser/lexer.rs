// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    diag, diagnostics::Diagnostic, parser::syntax::make_loc, shared::CompilationEnv,
    FileCommentMap, MatchedFileCommentMap,
};
use move_command_line_common::files::FileHash;
use move_ir_types::location::Loc;
use std::fmt;

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Tok {
    EOF,
    NumValue,
    NumTypedValue,
    ByteStringValue,
    Identifier,
    Exclaim,
    ExclaimEqual,
    Percent,
    Amp,
    AmpAmp,
    AmpMut,
    LParen,
    RParen,
    LBracket,
    RBracket,
    Star,
    Plus,
    Comma,
    Minus,
    Period,
    PeriodPeriod,
    Slash,
    Colon,
    ColonColon,
    Semicolon,
    Less,
    LessEqual,
    LessLess,
    Equal,
    PlusEqual,
    SubEqual,
    MulEqual,
    ModEqual,
    DivEqual,
    BitOrEqual,
    BitAndEqual,
    XorEqual,
    ShlEqual,
    ShrEqual,
    EqualEqual,
    EqualGreater,
    EqualEqualGreater,
    LessEqualEqualGreater,
    Greater,
    GreaterEqual,
    GreaterGreater,
    Caret,
    Abort,
    Acquires,
    As,
    Break,
    Continue,
    Copy,
    Else,
    False,
    If,
    Invariant,
    Let,
    Loop,
    Module,
    Move,
    Native,
    Public,
    Return,
    Spec,
    Struct,
    True,
    Use,
    While,
    LBrace,
    Pipe,
    PipePipe,
    RBrace,
    Fun,
    Script,
    Const,
    Friend,
    NumSign,
    AtSign,
    Inline,
    Label,
}

impl fmt::Display for Tok {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        use Tok::*;
        let s = match *self {
            EOF => "[end-of-file]",
            NumValue => "[Num]",
            NumTypedValue => "[NumTyped]",
            ByteStringValue => "[ByteString]",
            Identifier => "[Identifier]",
            Exclaim => "!",
            ExclaimEqual => "!=",
            Percent => "%",
            Amp => "&",
            AmpAmp => "&&",
            AmpMut => "&mut",
            LParen => "(",
            RParen => ")",
            LBracket => "[",
            RBracket => "]",
            Star => "*",
            Plus => "+",
            PlusEqual => "+=",
            SubEqual => "-=",
            MulEqual => "*=",
            ModEqual => "%=",
            DivEqual => "/=",
            BitOrEqual => "|=",
            BitAndEqual => "&=",
            XorEqual => "^=",
            ShlEqual => "<<=",
            ShrEqual => ">>=",
            Comma => ",",
            Minus => "-",
            Period => ".",
            PeriodPeriod => "..",
            Slash => "/",
            Colon => ":",
            ColonColon => "::",
            Semicolon => ";",
            Less => "<",
            LessEqual => "<=",
            LessLess => "<<",
            Equal => "=",
            EqualEqual => "==",
            EqualGreater => "=>",
            EqualEqualGreater => "==>",
            LessEqualEqualGreater => "<==>",
            Greater => ">",
            GreaterEqual => ">=",
            GreaterGreater => ">>",
            Caret => "^",
            Abort => "abort",
            Acquires => "acquires",
            As => "as",
            Break => "break",
            Continue => "continue",
            Copy => "copy",
            Else => "else",
            False => "false",
            If => "if",
            Invariant => "invariant",
            Let => "let",
            Loop => "loop",
            Inline => "inline",
            Module => "module",
            Move => "move",
            Native => "native",
            Public => "public",
            Return => "return",
            Spec => "spec",
            Struct => "struct",
            True => "true",
            Use => "use",
            While => "while",
            LBrace => "{",
            Pipe => "|",
            PipePipe => "||",
            RBrace => "}",
            Fun => "fun",
            Script => "script",
            Const => "const",
            Friend => "friend",
            NumSign => "#",
            AtSign => "@",
            Label => "[Label]",
        };
        fmt::Display::fmt(s, formatter)
    }
}

pub struct Lexer<'input> {
    text: &'input str,
    file_hash: FileHash,
    doc_comments: FileCommentMap,
    matched_doc_comments: MatchedFileCommentMap,
    prev_end: usize,
    cur_start: usize,
    cur_end: usize,
    token: Tok,
}

impl<'input> Lexer<'input> {
    pub fn new(text: &'input str, file_hash: FileHash) -> Lexer<'input> {
        Lexer {
            text,
            file_hash,
            doc_comments: FileCommentMap::new(),
            matched_doc_comments: MatchedFileCommentMap::new(),
            prev_end: 0,
            cur_start: 0,
            cur_end: 0,
            token: Tok::EOF,
        }
    }

    pub fn peek(&self) -> Tok {
        self.token
    }

    pub fn content(&self) -> &'input str {
        &self.text[self.cur_start..self.cur_end]
    }

    pub fn file_hash(&self) -> FileHash {
        self.file_hash
    }

    pub fn start_loc(&self) -> usize {
        self.cur_start
    }

    pub fn previous_end_loc(&self) -> usize {
        self.prev_end
    }

    /// Strips line and block comments from input source, and collects documentation comments,
    /// putting them into a map indexed by the span of the comment region. Comments in the original
    /// source will be replaced by spaces, such that positions of source items stay unchanged.
    /// Block comments can be nested.
    ///
    /// Documentation comments are comments which start with
    /// `///` or `/**`, but not `////` or `/***`. The actual comment delimiters
    /// (`/// .. <newline>` and `/** .. */`) will be not included in extracted comment string. The
    /// span in the returned map, however, covers the whole region of the comment, including the
    /// delimiters.
    fn trim_whitespace_and_comments(
        &mut self,
        offset: usize,
    ) -> Result<&'input str, Box<Diagnostic>> {
        let mut text = &self.text[offset..];

        // A helper function to compute the index of the start of the given substring.
        let len = text.len();
        let get_offset = |substring: &str| offset + len - substring.len();

        // Loop until we find text that isn't whitespace, and that isn't part of
        // a multi-line or single-line comment.
        loop {
            // Trim the start whitespace characters.
            text = trim_start_whitespace(text);

            if text.starts_with("/*") {
                // Strip multi-line comments like '/* ... */' or '/** ... */'.
                // These can be nested, as in '/* /* ... */ */', so record the
                // start locations of each nested comment as a stack. The
                // boolean indicates whether it's a documentation comment.
                let mut locs: Vec<(usize, bool)> = vec![];
                loop {
                    text = text.trim_start_matches(|c: char| c != '/' && c != '*');
                    if text.is_empty() {
                        // We've reached the end of string while searching for a
                        // terminating '*/'.
                        let loc = *locs.last().unwrap();
                        // Highlight the '/**' if it's a documentation comment, or the '/*'
                        // otherwise.
                        let location =
                            make_loc(self.file_hash, loc.0, loc.0 + if loc.1 { 3 } else { 2 });
                        return Err(Box::new(diag!(
                            Syntax::InvalidDocComment,
                            (location, "Unclosed block comment"),
                        )));
                    } else if text.starts_with("/*") {
                        // We've found a (perhaps nested) multi-line comment.
                        let start = get_offset(text);
                        text = &text[2..];

                        // Check if this is a documentation comment: '/**', but neither '/***' nor '/**/'.
                        // A documentation comment cannot be nested within another comment.
                        let is_doc = text.starts_with('*')
                            && !text.starts_with("**")
                            && !text.starts_with("*/")
                            && locs.is_empty();

                        locs.push((start, is_doc));
                    } else if text.starts_with("*/") {
                        // We've found a multi-line comment terminator that ends
                        // our innermost nested comment.
                        let loc = locs.pop().unwrap();
                        text = &text[2..];

                        // If this was a documentation comment, record it in our map.
                        if loc.1 {
                            let end = get_offset(text);
                            self.doc_comments.insert(
                                (loc.0 as u32, end as u32),
                                self.text[(loc.0 + 3)..(end - 2)].to_string(),
                            );
                        }

                        // If this terminated our last comment, exit the loop.
                        if locs.is_empty() {
                            break;
                        }
                    } else {
                        // This is a solitary '/' or '*' that isn't part of any comment delimiter.
                        // Skip over it.
                        text = &text[1..];
                    }
                }

                // Continue the loop immediately after the multi-line comment.
                // There may be whitespace or another comment following this one.
                continue;
            } else if text.starts_with("//") {
                let start = get_offset(text);
                let is_doc = text.starts_with("///") && !text.starts_with("////");
                text = text.trim_start_matches(|c: char| c != '\n');

                // If this was a documentation comment, record it in our map.
                if is_doc {
                    let end = get_offset(text);
                    let mut comment = &self.text[(start + 3)..end];
                    comment = comment.trim_end_matches(|c: char| c == '\r');

                    self.doc_comments
                        .insert((start as u32, end as u32), comment.to_string());
                }

                // Continue the loop on the following line, which may contain leading
                // whitespace or comments of its own.
                continue;
            }
            break;
        }
        Ok(text)
    }

    // Look ahead to the next token after the current one and return it, and its starting offset,
    // without advancing the state of the lexer.
    pub fn lookahead_with_start_loc(&mut self) -> Result<(Tok, usize), Box<Diagnostic>> {
        let text = self.trim_whitespace_and_comments(self.cur_end)?;
        let next_start = self.text.len() - text.len();
        let (tok, _) = find_token(self.file_hash, text, next_start)?;
        Ok((tok, next_start))
    }

    // Look ahead to the next token after the current one and return it without advancing
    // the state of the lexer.
    pub fn lookahead(&mut self) -> Result<Tok, Box<Diagnostic>> {
        Ok(self.lookahead_with_start_loc()?.0)
    }

    // Look ahead to the next two tokens after the current one and return them without advancing
    // the state of the lexer.
    pub fn lookahead2(&mut self) -> Result<(Tok, Tok), Box<Diagnostic>> {
        let text = self.trim_whitespace_and_comments(self.cur_end)?;
        let offset = self.text.len() - text.len();
        let (first, length) = find_token(self.file_hash, text, offset)?;
        let text2 = self.trim_whitespace_and_comments(offset + length)?;
        let offset2 = self.text.len() - text2.len();
        let (second, _) = find_token(self.file_hash, text2, offset2)?;
        Ok((first, second))
    }

    // Look ahead to the nth token after the current one and return it without advancing
    // the state of the lexer.
    pub fn lookahead_nth(&mut self, n: usize) -> Result<Tok, Box<Diagnostic>> {
        let mut current_offset = self.cur_end;
        let mut token = Tok::EOF;

        for _ in 0..=n {
            let text = self.trim_whitespace_and_comments(current_offset)?;
            let offset = self.text.len() - text.len();
            let (found_token, length) = find_token(self.file_hash, text, offset)?;
            token = found_token;
            current_offset = offset + length;
        }
        Ok(token)
    }

    // Matches the doc comments after the last token (or the beginning of the file) to the position
    // of the current token. This moves the comments out of `doc_comments` and
    // into `matched_doc_comments`. At the end of parsing, if `doc_comments` is not empty, errors
    // for stale doc comments will be produced.
    //
    // Calling this function during parsing effectively marks a valid point for documentation
    // comments. The documentation comments are not stored in the AST, but can be retrieved by
    // using the start position of an item as an index into `matched_doc_comments`.
    pub fn match_doc_comments(&mut self) {
        let start = self.previous_end_loc() as u32;
        let end = self.cur_start as u32;
        let mut matched = vec![];
        let merged = self
            .doc_comments
            .range((start, start)..(end, end))
            .map(|(span, s)| {
                matched.push(*span);
                s.clone()
            })
            .collect::<Vec<String>>()
            .join("\n");
        for span in matched {
            self.doc_comments.remove(&span);
        }
        self.matched_doc_comments.insert(end, merged);
    }

    // At the end of parsing, checks whether there are any unmatched documentation comments,
    // producing errors if so. Otherwise returns a map from file position to associated
    // documentation.
    pub fn check_and_get_doc_comments(
        &mut self,
        env: &mut CompilationEnv,
    ) -> MatchedFileCommentMap {
        let msg = "Documentation comment cannot be matched to a language item";
        let diags = self
            .doc_comments
            .iter()
            .map(|((start, end), _)| {
                let loc = Loc::new(self.file_hash, *start, *end);
                diag!(Syntax::InvalidDocComment, (loc, msg))
            })
            .collect();
        env.add_diags(diags);
        std::mem::take(&mut self.matched_doc_comments)
    }

    pub fn advance(&mut self) -> Result<(), Box<Diagnostic>> {
        self.prev_end = self.cur_end;
        let text = self.trim_whitespace_and_comments(self.cur_end)?;
        self.cur_start = self.text.len() - text.len();
        let (token, len) = find_token(self.file_hash, text, self.cur_start)?;
        self.cur_end = self.cur_start + len;
        self.token = token;
        Ok(())
    }

    pub fn advance_with_loc(&mut self) -> Result<Loc, Box<Diagnostic>> {
        let start_loc = self.start_loc();
        self.advance()?;
        let end_loc = self.previous_end_loc();
        Ok(make_loc(self.file_hash, start_loc, end_loc))
    }

    // Replace the current token. The lexer will always match the longest token,
    // but sometimes the parser will prefer to replace it with a shorter one,
    // e.g., ">" instead of ">>".
    pub fn replace_token(&mut self, token: Tok, len: usize) {
        self.token = token;
        self.cur_end = self.cur_start + len
    }
}

// Find the next token and its length without changing the state of the lexer.
fn find_token(
    file_hash: FileHash,
    text: &str,
    start_offset: usize,
) -> Result<(Tok, usize), Box<Diagnostic>> {
    let c: char = match text.chars().next() {
        Some(next_char) => next_char,
        None => {
            return Ok((Tok::EOF, 0));
        },
    };
    let (tok, len) = match c {
        '0'..='9' => {
            if text.starts_with("0x") && text.len() > 2 {
                let (tok, hex_len) = get_hex_number(&text[2..]);
                if hex_len == 0 {
                    // Fall back to treating this as a "0" token.
                    (Tok::NumValue, 1)
                } else {
                    (tok, 2 + hex_len)
                }
            } else {
                get_decimal_number(text)
            }
        },
        'A'..='Z' | 'a'..='z' | '_' => {
            let is_hex = text.starts_with("x\"");
            if is_hex || text.starts_with("b\"") {
                let line = &text.lines().next().unwrap()[2..];
                match get_string_len(line) {
                    Some(last_quote) => (Tok::ByteStringValue, 2 + last_quote + 1),
                    None => {
                        let loc = make_loc(file_hash, start_offset, start_offset + line.len() + 2);
                        return Err(Box::new(diag!(
                            if is_hex {
                                Syntax::InvalidHexString
                            } else {
                                Syntax::InvalidByteString
                            },
                            (loc, "Missing closing quote (\") after byte string")
                        )));
                    },
                }
            } else {
                let len = get_name_len(text);
                (get_name_token(&text[..len]), len)
            }
        },
        '\'' => {
            if text.len() <= 1
                || !matches!(text.chars().nth(1).unwrap(), 'A'..='Z' | 'a'..='z' | '_')
            {
                let loc = make_loc(file_hash, start_offset + 1, start_offset + 2);
                return Err(Box::new(diag!(
                    Syntax::InvalidCharacter,
                    (loc, "Label quote must be followed by 'A-Z', `a-z', or '_'")
                )));
            }
            let len = get_name_len(&text[1..]);
            (Tok::Label, 1 + len)
        },
        '"' => {
            let line = &text.lines().next().unwrap()[1..];
            match get_string_len(line) {
                Some(_last_quote) => {
                    let loc = make_loc(file_hash, start_offset, start_offset);
                    return Err(Box::new(diag!(
                        Syntax::InvalidByteString,
                        (loc, "String literal must begin with b\" (for a byte string) or x\" (for a hex string)")
                    )));
                },
                None => {
                    let loc = make_loc(file_hash, start_offset, start_offset);
                    return Err(Box::new(diag!(
                        Syntax::InvalidCharacter,
                        (loc, format!("Invalid character: '{}'; string literal must begin with `b\"` and closing quote `\"` must appear on same line", c))
                    )));
                },
            }
        },
        '&' => {
            if text.starts_with("&mut ") {
                (Tok::AmpMut, 5)
            } else if text.starts_with("&&") {
                (Tok::AmpAmp, 2)
            } else if text.starts_with("&=") {
                (Tok::BitAndEqual, 2)
            } else {
                (Tok::Amp, 1)
            }
        },
        '|' => {
            if text.starts_with("||") {
                (Tok::PipePipe, 2)
            } else if text.starts_with("|=") {
                (Tok::BitOrEqual, 2)
            } else {
                (Tok::Pipe, 1)
            }
        },
        '=' => {
            if text.starts_with("==>") {
                (Tok::EqualEqualGreater, 3)
            } else if text.starts_with("=>") {
                (Tok::EqualGreater, 2)
            } else if text.starts_with("==") {
                (Tok::EqualEqual, 2)
            } else {
                (Tok::Equal, 1)
            }
        },
        '!' => {
            if text.starts_with("!=") {
                (Tok::ExclaimEqual, 2)
            } else {
                (Tok::Exclaim, 1)
            }
        },
        '<' => {
            if text.starts_with("<==>") {
                (Tok::LessEqualEqualGreater, 4)
            } else if text.starts_with("<<=") {
                (Tok::ShlEqual, 3)
            } else if text.starts_with("<=") {
                (Tok::LessEqual, 2)
            } else if text.starts_with("<<") {
                (Tok::LessLess, 2)
            } else {
                (Tok::Less, 1)
            }
        },
        '>' => {
            if text.starts_with(">>=") {
                (Tok::ShrEqual, 3)
            } else if text.starts_with(">=") {
                (Tok::GreaterEqual, 2)
            } else if text.starts_with(">>") {
                (Tok::GreaterGreater, 2)
            } else {
                (Tok::Greater, 1)
            }
        },
        ':' => {
            if text.starts_with("::") {
                (Tok::ColonColon, 2)
            } else {
                (Tok::Colon, 1)
            }
        },
        '%' => {
            if text.starts_with("%=") {
                (Tok::ModEqual, 2)
            } else {
                (Tok::Percent, 1)
            }
        },
        '(' => (Tok::LParen, 1),
        ')' => (Tok::RParen, 1),
        '[' => (Tok::LBracket, 1),
        ']' => (Tok::RBracket, 1),
        '*' => {
            if text.starts_with("*=") {
                (Tok::MulEqual, 2)
            } else {
                (Tok::Star, 1)
            }
        },
        '+' => {
            if text.starts_with("+=") {
                (Tok::PlusEqual, 2)
            } else {
                (Tok::Plus, 1)
            }
        },
        ',' => (Tok::Comma, 1),
        '-' => {
            if text.starts_with("-=") {
                (Tok::SubEqual, 2)
            } else {
                (Tok::Minus, 1)
            }
        },
        '.' => {
            if text.starts_with("..") {
                (Tok::PeriodPeriod, 2)
            } else {
                (Tok::Period, 1)
            }
        },
        '/' => {
            if text.starts_with("/=") {
                (Tok::DivEqual, 2)
            } else {
                (Tok::Slash, 1)
            }
        },
        ';' => (Tok::Semicolon, 1),
        '^' => {
            if text.starts_with("^=") {
                (Tok::XorEqual, 2)
            } else {
                (Tok::Caret, 1)
            }
        },
        '{' => (Tok::LBrace, 1),
        '}' => (Tok::RBrace, 1),
        '#' => (Tok::NumSign, 1),
        '@' => (Tok::AtSign, 1),
        _ => {
            let loc = make_loc(file_hash, start_offset, start_offset);
            return Err(Box::new(diag!(
                Syntax::InvalidCharacter,
                (loc, format!("Invalid character: '{}'", c))
            )));
        },
    };

    Ok((tok, len))
}

// Return the length of the substring matching [a-zA-Z0-9_]. Note that
// this does not do any special check for whether the first character
// starts with a number, so the caller is responsible for any additional
// checks on the first character.
fn get_name_len(text: &str) -> usize {
    text.chars()
        .position(|c| !matches!(c, 'a'..='z' | 'A'..='Z' | '_' | '0'..='9'))
        .unwrap_or(text.len())
}

fn get_decimal_number(text: &str) -> (Tok, usize) {
    let num_text_len = text
        .chars()
        .position(|c| !matches!(c, '0'..='9' | '_'))
        .unwrap_or(text.len());
    get_number_maybe_with_suffix(text, num_text_len)
}

// Return the length of the substring containing characters in [0-9a-fA-F].
fn get_hex_number(text: &str) -> (Tok, usize) {
    let num_text_len = text
        .find(|c| !matches!(c, 'a'..='f' | 'A'..='F' | '0'..='9'| '_'))
        .unwrap_or(text.len());
    get_number_maybe_with_suffix(text, num_text_len)
}

// Given the text for a number literal and the length for the characters that match to the number
// portion, checks for a typed suffix.
fn get_number_maybe_with_suffix(text: &str, num_text_len: usize) -> (Tok, usize) {
    let rest = &text[num_text_len..];
    if rest.starts_with("u8") {
        (Tok::NumTypedValue, num_text_len + 2)
    } else if rest.starts_with("u64") || rest.starts_with("u16") || rest.starts_with("u32") {
        (Tok::NumTypedValue, num_text_len + 3)
    } else if rest.starts_with("u128") || rest.starts_with("u256") {
        (Tok::NumTypedValue, num_text_len + 4)
    } else {
        // No typed suffix
        (Tok::NumValue, num_text_len)
    }
}

// Return the length of the quoted string, or None if there is no closing quote.
fn get_string_len(text: &str) -> Option<usize> {
    let mut pos = 0;
    let mut iter = text.chars();
    while let Some(chr) = iter.next() {
        if chr == '\\' {
            // Skip over the escaped character (e.g., a quote or another backslash)
            if iter.next().is_some() {
                pos += 1;
            }
        } else if chr == '"' {
            return Some(pos);
        }
        pos += 1;
    }
    None
}

fn get_name_token(name: &str) -> Tok {
    match name {
        "abort" => Tok::Abort,
        "acquires" => Tok::Acquires,
        "as" => Tok::As,
        "break" => Tok::Break,
        "const" => Tok::Const,
        "continue" => Tok::Continue,
        "copy" => Tok::Copy,
        "else" => Tok::Else,
        "false" => Tok::False,
        "fun" => Tok::Fun,
        "friend" => Tok::Friend,
        "if" => Tok::If,
        "invariant" => Tok::Invariant,
        "let" => Tok::Let,
        "loop" => Tok::Loop,
        "inline" => Tok::Inline,
        "module" => Tok::Module,
        "move" => Tok::Move,
        "native" => Tok::Native,
        "public" => Tok::Public,
        "return" => Tok::Return,
        "script" => Tok::Script,
        "spec" => Tok::Spec,
        "struct" => Tok::Struct,
        "true" => Tok::True,
        "use" => Tok::Use,
        "while" => Tok::While,
        _ => Tok::Identifier,
    }
}

// Trim the start whitespace characters, include: space, tab, lf(\n) and crlf(\r\n).
fn trim_start_whitespace(text: &str) -> &str {
    let mut pos = 0;
    let mut iter = text.chars();

    while let Some(chr) = iter.next() {
        match chr {
            ' ' | '\t' | '\n' => pos += 1,
            '\r' if matches!(iter.next(), Some('\n')) => pos += 2,
            _ => break,
        };
    }

    &text[pos..]
}

#[cfg(test)]
mod tests {
    use super::trim_start_whitespace;

    #[test]
    fn test_trim_start_whitespace() {
        assert_eq!(trim_start_whitespace("\r"), "\r");
        assert_eq!(trim_start_whitespace("\rxxx"), "\rxxx");
        assert_eq!(trim_start_whitespace("\t\rxxx"), "\rxxx");
        assert_eq!(trim_start_whitespace("\r\n\rxxx"), "\rxxx");

        assert_eq!(trim_start_whitespace("\n"), "");
        assert_eq!(trim_start_whitespace("\r\n"), "");
        assert_eq!(trim_start_whitespace("\t"), "");
        assert_eq!(trim_start_whitespace(" "), "");

        assert_eq!(trim_start_whitespace("\nxxx"), "xxx");
        assert_eq!(trim_start_whitespace("\r\nxxx"), "xxx");
        assert_eq!(trim_start_whitespace("\txxx"), "xxx");
        assert_eq!(trim_start_whitespace(" xxx"), "xxx");

        assert_eq!(trim_start_whitespace(" \r\n"), "");
        assert_eq!(trim_start_whitespace("\t\r\n"), "");
        assert_eq!(trim_start_whitespace("\n\r\n"), "");
        assert_eq!(trim_start_whitespace("\r\n "), "");
        assert_eq!(trim_start_whitespace("\r\n\t"), "");
        assert_eq!(trim_start_whitespace("\r\n\n"), "");

        assert_eq!(trim_start_whitespace(" \r\nxxx"), "xxx");
        assert_eq!(trim_start_whitespace("\t\r\nxxx"), "xxx");
        assert_eq!(trim_start_whitespace("\n\r\nxxx"), "xxx");
        assert_eq!(trim_start_whitespace("\r\n xxx"), "xxx");
        assert_eq!(trim_start_whitespace("\r\n\txxx"), "xxx");
        assert_eq!(trim_start_whitespace("\r\n\nxxx"), "xxx");

        assert_eq!(trim_start_whitespace(" \r\n\r\n"), "");
        assert_eq!(trim_start_whitespace("\r\n \t\n"), "");

        assert_eq!(trim_start_whitespace(" \r\n\r\nxxx"), "xxx");
        assert_eq!(trim_start_whitespace("\r\n \t\nxxx"), "xxx");

        assert_eq!(trim_start_whitespace(" \r\n\r\nxxx\n"), "xxx\n");
        assert_eq!(trim_start_whitespace("\r\n \t\nxxx\r\n"), "xxx\r\n");
    }
}
