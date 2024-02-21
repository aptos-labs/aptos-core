// Copyright (c) The BitsLab.MoveBit Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::core::token_tree::{analyze_token_tree_length, NestKind, NestKind_, Note, TokenTree};
use commentfmt::Config;
use move_compiler::parser::lexer::Tok;

pub enum TokType {
    /// abc like token,
    Alphabet,
    /// + - ...
    MathSign,
    ///
    Sign,
    // specials no need space at all.
    NoNeedSpace,
    /// numbers 0x1 ...
    Number,
    /// b"hello world"
    String,
    /// &
    Amp,
    /// *
    Star,
    /// &mut
    AmpMut,
    ///
    Semicolon,
    ///:
    Colon,
    /// @
    AtSign,
    /// <
    Less,
}

impl From<Tok> for TokType {
    fn from(value: Tok) -> Self {
        match value {
            Tok::EOF => unreachable!(), // EOF not in `TokenTree`.
            Tok::NumValue => TokType::Number,
            Tok::NumTypedValue => TokType::Number,
            Tok::ByteStringValue => TokType::String,
            Tok::Exclaim => TokType::Sign,
            Tok::ExclaimEqual => TokType::MathSign,
            Tok::Percent => TokType::MathSign,
            Tok::Amp => TokType::Amp,
            Tok::AmpAmp => TokType::MathSign,
            Tok::LParen => TokType::Sign,
            Tok::RParen => TokType::Sign,
            Tok::LBracket => TokType::Sign,
            Tok::RBracket => TokType::Sign,
            Tok::Star => TokType::Star,
            Tok::Plus => TokType::MathSign,
            Tok::Comma => TokType::Sign,
            Tok::Minus => TokType::MathSign,
            Tok::Period => TokType::NoNeedSpace,
            Tok::PeriodPeriod => TokType::NoNeedSpace,
            Tok::Slash => TokType::Sign,
            Tok::Colon => TokType::Colon,
            Tok::ColonColon => TokType::NoNeedSpace,
            Tok::Semicolon => TokType::Semicolon,
            Tok::Less => TokType::Less,
            Tok::LessEqual => TokType::MathSign,
            Tok::LessLess => TokType::MathSign,
            Tok::Equal => TokType::MathSign,
            Tok::EqualEqual => TokType::MathSign,
            Tok::EqualEqualGreater => TokType::MathSign,
            Tok::LessEqualEqualGreater => TokType::MathSign,
            Tok::Greater => TokType::MathSign,
            Tok::GreaterEqual => TokType::MathSign,
            Tok::GreaterGreater => TokType::MathSign,
            Tok::LBrace => TokType::Sign,
            Tok::Pipe => TokType::Sign,
            Tok::PipePipe => TokType::MathSign,
            Tok::RBrace => TokType::Sign,
            Tok::NumSign => TokType::Sign,
            Tok::AtSign => TokType::AtSign,
            Tok::AmpMut => TokType::Amp,
            _ => TokType::Alphabet,
        }
    }
}

fn get_start_tok(t: &TokenTree) -> Tok {
    match t {
        TokenTree::SimpleToken {
            content: _,
            pos: _,
            tok,
            note: _,
        } => *tok,
        TokenTree::Nested {
            elements: _,
            kind,
            note: _,
        } => kind.kind.start_tok(),
    }
}

fn get_end_tok(t: &TokenTree) -> Tok {
    match t {
        TokenTree::SimpleToken {
            content: _,
            pos: _,
            tok,
            note: _,
        } => *tok,
        TokenTree::Nested {
            elements: _,
            kind,
            note: _,
        } => kind.kind.end_tok(),
    }
}

fn is_to_or_except(token: &Option<&TokenTree>) -> bool {
    match token {
        None => false,
        Some(TokenTree::SimpleToken { content: con, .. }) => {
            con.as_str() == "to" || con.as_str() == "except"
        }
        _ => false,
    }
}

fn get_nested_and_comma_num(elements: &Vec<TokenTree>) -> (usize, usize) {
    let mut result = (0, 0);
    for ele in elements {
        if let TokenTree::Nested {
            elements: recursive_elements,
            kind: _,
            note: _,
        } = ele
        {
            let recursive_result = get_nested_and_comma_num(recursive_elements);
            result.0 += recursive_result.0 + 1;
            result.1 += recursive_result.1;
        }
        if let TokenTree::SimpleToken {
            content: _,
            pos: _,
            tok,
            note: _,
        } = ele
        {
            if Tok::Comma == *tok {
                result.1 += 1;
            }
        }
    }

    result
}

pub(crate) fn need_space(current: &TokenTree, next: Option<&TokenTree>) -> bool {
    if next.is_none() {
        return false;
    }

    let _is_bin_current = current
        .get_note()
        .map(|x| x == Note::BinaryOP)
        .unwrap_or_default();

    let is_bin_next = match next {
        None => false,
        Some(next_) => next_
            .get_note()
            .map(|x| x == Note::BinaryOP)
            .unwrap_or_default(),
    };
    let is_apply_current = current
        .get_note()
        .map(|x| x == Note::ApplyName)
        .unwrap_or_default();

    let is_apply_next = match next {
        None => false,
        Some(next_) => next_
            .get_note()
            .map(|x| x == Note::ApplyName)
            .unwrap_or_default(),
    };

    let is_to_execpt = is_to_or_except(&Some(current)) || is_to_or_except(&next);

    if let Tok::Greater = get_end_tok(current) {
        if let TokType::Alphabet = TokType::from(next.map(get_start_tok).unwrap()) {
            return true
        }
    }

    match (
        TokType::from(get_start_tok(current)),
        TokType::from(next.map(get_start_tok).unwrap()),
    ) {
        (TokType::Alphabet, TokType::Alphabet) => true,
        (TokType::MathSign, _) => true,
        (TokType::Sign, TokType::Alphabet) => Tok::Exclaim != get_end_tok(current),
        (TokType::Sign, TokType::Number) => true,
        (TokType::Sign, TokType::String | TokType::AtSign | TokType::Amp | TokType::AmpMut) => {
            let mut result = false;
            let mut next_tok = Tok::EOF;
            if let Some(x) = next {
                match x {
                    TokenTree::Nested {
                        elements: _,
                        kind,
                        note: _,
                    } => {
                        next_tok = kind.kind.start_tok();
                        // if kind.kind.start_tok() == Tok::LBrace {
                        //     result = true;
                        // }
                    }
                    TokenTree::SimpleToken {
                        content: _,
                        pos: _,
                        tok,
                        note: _,
                    } => {
                        next_tok = *tok;
                        // tracing::debug!("content = {:?}", content);
                        if Tok::ByteStringValue == *tok {
                            result = true;
                        }
                    }
                }
            }

            if Tok::Comma == get_start_tok(current) && 
                (Tok::AtSign == next_tok || Tok::Amp == next_tok || Tok::AmpMut == next_tok) {
                result = true;
                tracing::debug!("after Comma, result = {}, next_tok = {:?}", result, next_tok);
            }
            result
        }
        (TokType::Alphabet, TokType::String) => true,
        (TokType::Number, TokType::Alphabet) => true,
        (_, TokType::AmpMut) => true,
        (TokType::Colon, _) => true,
        (TokType::Alphabet, TokType::Number) => true,

        (_, TokType::Less) => is_bin_next,
        (TokType::Alphabet, TokType::MathSign) => {
            if let Some(next_tk) = next {
                if let Some(content) = next_tk.simple_str() {
                    if content.contains('>') {
                        return is_bin_next
                    }
                }
            }
            true
        },
        (_, TokType::MathSign) => true,
        (TokType::Less, TokType::Alphabet) => _is_bin_current,
        (TokType::Less, _) => false,

        (_, TokType::Amp) => is_bin_next,
        (_, TokType::Star) => {
            let result = if is_bin_next || is_apply_next {
                is_to_execpt
            } else {
                false
            };
            result
                || Tok::NumValue == get_start_tok(current)
                || Tok::NumTypedValue == get_start_tok(current)
                || Tok::Acquires == get_start_tok(current)
                || Tok::Identifier == get_start_tok(current)
                || Tok::RParen == get_end_tok(current)
                || Tok::Comma == get_end_tok(current)
        }

        (TokType::Star, _) => {
            if is_bin_next {
                return true;
            }
            match next {
                None => {}
                Some(next_) => {
                    if let TokenTree::Nested { elements, .. } = next_ {
                        // if elements.len() > 0 {
                        //     elements[0]
                        //     .get_note()
                        //     .map(|x| x == Note::BinaryOP)
                        //     .unwrap_or_default()
                        // } else {
                        //     false
                        // }
                        if Tok::LParen == get_start_tok(next_) {
                            return elements.len() > 2;
                        }
                    }
                }
            }

            if is_apply_current {
                is_to_execpt
            } else {
                let mut result = false;
                if let Some(x) = next {
                    match x {
                        TokenTree::Nested {
                            elements: _,
                            kind,
                            note: _,
                        } => {
                            if kind.kind == NestKind_::Brace {
                                result = true;
                            }
                        }
                        TokenTree::SimpleToken {
                            content,
                            pos: _,
                            tok,
                            note: _,
                        } => {
                            // tracing::debug!("content = {:?}", content);
                            if Tok::NumValue == *tok
                                || Tok::NumTypedValue == *tok
                                || Tok::LParen == *tok
                            {
                                result = true;
                            }
                            if Tok::Identifier == *tok {
                                if content.contains("vector") {
                                    result = false;
                                } else if _is_bin_current {
                                    result = true;
                                }
                            }
                        }
                    }
                }
                result
            }
        }

        (TokType::AtSign, TokType::Alphabet) => false,
        (TokType::Alphabet | TokType::Number | TokType::Sign, TokType::Sign) => {
            let mut result = false;
            let mut next_tok = Tok::EOF;
            if let Some(x) = next {
                match x {
                    TokenTree::Nested {
                        elements: _,
                        kind,
                        note: _,
                    } => {
                        next_tok = kind.kind.start_tok();
                        if kind.kind.start_tok() == Tok::LBrace {
                            result = true;
                        }
                    }
                    TokenTree::SimpleToken {
                        content: _,
                        pos: _,
                        tok,
                        note: _,
                    } => {
                        next_tok = *tok;
                        // tracing::debug!("content = {:?}", content);
                        if Tok::Slash == *tok || Tok::LBrace == *tok {
                            result = true;
                        }
                    }
                }
            }
            if Tok::Let == get_start_tok(current)
                || Tok::Slash == get_start_tok(current)
                || Tok::If == get_start_tok(current)
                || Tok::Else == get_start_tok(current)
                || Tok::While == get_start_tok(current)
            {
                result = true;
            }

            if let Some(content) = current.simple_str() {
                if content.contains("aborts_if") || content.contains("ensures") {
                    result = true;
                }
            }

            if next_tok == Tok::Exclaim {
                result = matches!(TokType::from(get_start_tok(current)), TokType::Alphabet)
                    || Tok::RParen == get_end_tok(current);
            }

            if Tok::RParen == get_end_tok(current) && next_tok == Tok::LParen {
                result = true;
            }

            // tracing::debug!("result = {}, next_tok = {:?}", result, next_tok);
            result
        }
        _ => false,
    }
}

pub(crate) fn judge_simple_paren_expr(
    kind: &NestKind,
    elements: &Vec<TokenTree>,
    config: Config,
) -> bool {
    if elements.is_empty() {
        return true;
    };
    if NestKind_::ParentTheses == kind.kind {
        let paren_num = get_nested_and_comma_num(elements);
        tracing::debug!(
            "elements[0] = {:?}, paren_num = {:?}",
            elements[0].simple_str(),
            paren_num
        );
        if paren_num.0 > 2 || paren_num.1 > 4 {
            return false;
        }
        if paren_num.0 >= 1 && paren_num.1 >= 4 {
            return false;
        }
        if analyze_token_tree_length(elements, config.max_width() / 2) >= config.max_width() - 55 {
            return false;
        }
    }
    true
}

pub(crate) fn process_link_access(elements: &[TokenTree], idx: usize) -> (usize, usize) {
    tracing::debug!("process_link_access >>");
    if idx >= elements.len() - 1 {
        return (0, 0);
    }
    let mut continue_dot_cnt = 0;
    let mut index = idx;
    while index <= elements.len() - 2 {
        let t = elements.get(index).unwrap();
        if !t.simple_str().unwrap_or_default().contains('.') {
            break;
        }
        continue_dot_cnt += 1;
        index += 2;
    }
    tracing::debug!(
        "process_link_access << (continue_dot_cnt, index) = ({}, {})",
        continue_dot_cnt,
        index
    );
    (continue_dot_cnt, index - 2)
}
