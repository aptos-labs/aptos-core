// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

// In the informal grammar comments in this file, Comma<T> is shorthand for:
//      (<T> ",")* <T>?
// Note that this allows an optional trailing comma.

use crate::{
    diag,
    diagnostics::{Diagnostic, Diagnostics},
    parser::{ast::*, lexer::*},
    shared::*,
    MatchedFileCommentMap,
};
use move_command_line_common::files::FileHash;
use move_ir_types::location::*;
use move_symbol_pool::Symbol;

struct Context<'env, 'lexer, 'input> {
    env: &'env mut CompilationEnv,
    tokens: &'lexer mut Lexer<'input>,
}

impl<'env, 'lexer, 'input> Context<'env, 'lexer, 'input> {
    fn new(env: &'env mut CompilationEnv, tokens: &'lexer mut Lexer<'input>) -> Self {
        Self { env, tokens }
    }
}

// Internal complier variables used to represent `for` loops.
pub const FOR_LOOP_UPDATE_ITER_FLAG: &str = "__update_iter_flag";
const FOR_LOOP_UPPER_BOUND_VALUE: &str = "__upper_bound_value";

//**************************************************************************************************
// Error Handling
//**************************************************************************************************

fn current_token_error_string(tokens: &Lexer) -> String {
    if tokens.peek() == Tok::EOF {
        "end-of-file".to_string()
    } else {
        format!("'{}'", tokens.content())
    }
}

fn unexpected_token_error(tokens: &Lexer, expected: &str) -> Box<Diagnostic> {
    unexpected_token_error_(tokens, tokens.start_loc(), expected)
}

fn unexpected_token_error_(
    tokens: &Lexer,
    expected_start_loc: usize,
    expected: &str,
) -> Box<Diagnostic> {
    let unexpected_loc = current_token_loc(tokens);
    let unexpected = current_token_error_string(tokens);
    let expected_loc = if expected_start_loc < tokens.start_loc() {
        make_loc(
            tokens.file_hash(),
            expected_start_loc,
            tokens.previous_end_loc(),
        )
    } else {
        unexpected_loc
    };
    Box::new(diag!(
        Syntax::UnexpectedToken,
        (unexpected_loc, format!("Unexpected {}", unexpected)),
        (expected_loc, format!("Expected {}", expected)),
    ))
}

fn add_type_args_ambiguity_label(loc: Loc, mut diag: Box<Diagnostic>) -> Box<Diagnostic> {
    const MSG: &str = "Perhaps you need a blank space before this '<' operator?";
    diag.add_secondary_label((loc, MSG));
    diag
}

//**************************************************************************************************
// Miscellaneous Utilities
//**************************************************************************************************

fn require_move_2(context: &mut Context, loc: Loc, description: &str) -> bool {
    require_language_version_msg(
        context,
        loc,
        LanguageVersion::V2_0,
        &format!("Move 2 language construct is not enabled: {}", description),
    )
}

fn require_move_version(
    min_language_version: LanguageVersion,
    context: &mut Context,
    loc: Loc,
    description: &str,
) -> bool {
    require_language_version_msg(
        context,
        loc,
        min_language_version,
        &format!(
            "Move {} language construct is not enabled: {}",
            min_language_version, description
        ),
    )
}

fn require_language_version(
    context: &mut Context,
    loc: Loc,
    min_language_version: LanguageVersion,
    description: &str,
) -> bool {
    require_language_version_msg(
        context,
        loc,
        min_language_version,
        &format!(
            "{} not enabled before version {}",
            description, min_language_version
        ),
    )
}

fn require_language_version_msg(
    context: &mut Context,
    loc: Loc,
    min_language_version: LanguageVersion,
    msg: &str,
) -> bool {
    if context.env.flags().language_version() < min_language_version {
        context
            .env
            .add_diag(diag!(Syntax::UnsupportedLanguageItem, (loc, msg)));
        false
    } else {
        true
    }
}

fn require_move_2_and_advance(
    context: &mut Context,
    description: &str,
) -> Result<bool, Box<Diagnostic>> {
    let loc = current_token_loc(context.tokens);
    context.tokens.advance()?;
    Ok(require_move_2(context, loc, description))
}

fn require_move_version_and_advance(
    min_language_version: LanguageVersion,
    context: &mut Context,
    description: &str,
) -> Result<bool, Box<Diagnostic>> {
    let loc = current_token_loc(context.tokens);
    context.tokens.advance()?;
    Ok(require_move_version(
        min_language_version,
        context,
        loc,
        description,
    ))
}

pub fn make_loc(file_hash: FileHash, start: usize, end: usize) -> Loc {
    Loc::new(file_hash, start as u32, end as u32)
}

fn current_token_loc(tokens: &Lexer) -> Loc {
    let start_loc = tokens.start_loc();
    make_loc(
        tokens.file_hash(),
        start_loc,
        start_loc + tokens.content().len(),
    )
}

fn spanned<T>(file_hash: FileHash, start: usize, end: usize, value: T) -> Spanned<T> {
    Spanned {
        loc: make_loc(file_hash, start, end),
        value,
    }
}

// Check for the specified token and consume it if it matches.
// Returns true if the token matches.
fn match_token(tokens: &mut Lexer, tok: Tok) -> Result<bool, Box<Diagnostic>> {
    if tokens.peek() == tok {
        tokens.advance()?;
        Ok(true)
    } else {
        Ok(false)
    }
}

// Check for the specified token and return an error if it does not match.
fn consume_token(tokens: &mut Lexer, tok: Tok) -> Result<(), Box<Diagnostic>> {
    consume_token_(tokens, tok, tokens.start_loc(), "")
}

fn consume_token_(
    tokens: &mut Lexer,
    tok: Tok,
    expected_start_loc: usize,
    expected_case: &str,
) -> Result<(), Box<Diagnostic>> {
    if tokens.peek() == tok {
        tokens.advance()?;
        Ok(())
    } else {
        let expected = format!("'{}'{}", tok, expected_case);
        Err(unexpected_token_error_(
            tokens,
            expected_start_loc,
            &expected,
        ))
    }
}

// Check for the identifier token with specified value and return an error if it does not match.
fn consume_identifier(tokens: &mut Lexer, value: &str) -> Result<(), Box<Diagnostic>> {
    if tokens.peek() == Tok::Identifier && tokens.content() == value {
        tokens.advance()
    } else {
        let expected = format!("'{}'", value);
        Err(unexpected_token_error(tokens, &expected))
    }
}

// If the next token is the specified kind, consume it and return
// its source location.
fn consume_optional_token_with_loc(
    tokens: &mut Lexer,
    tok: Tok,
) -> Result<Option<Loc>, Box<Diagnostic>> {
    if tokens.peek() == tok {
        let start_loc = tokens.start_loc();
        tokens.advance()?;
        let end_loc = tokens.previous_end_loc();
        Ok(Some(make_loc(tokens.file_hash(), start_loc, end_loc)))
    } else {
        Ok(None)
    }
}

// While parsing a list and expecting a ">" token to mark the end, replace
// a ">>" token with the expected ">". This handles the situation where there
// are nested type parameters that result in two adjacent ">" tokens, e.g.,
// "A<B<C>>".
fn adjust_token(tokens: &mut Lexer, end_token: Tok) {
    if tokens.peek() == Tok::GreaterGreater && end_token == Tok::Greater {
        tokens.replace_token(Tok::Greater, 1);
    }
}

// Parse a comma-separated list of items, including the specified starting and
// ending tokens.
fn parse_comma_list<F, R>(
    context: &mut Context,
    start_token: Tok,
    end_token: Tok,
    parse_list_item: F,
    item_description: &str,
) -> Result<Vec<R>, Box<Diagnostic>>
where
    F: Fn(&mut Context) -> Result<R, Box<Diagnostic>>,
{
    let start_loc = context.tokens.start_loc();
    consume_token(context.tokens, start_token)?;
    parse_comma_list_after_start(
        context,
        start_loc,
        start_token,
        end_token,
        parse_list_item,
        item_description,
    )
}

// Parse a comma-separated list of items, including the specified ending token, but
// assuming that the starting token has already been consumed.
fn parse_comma_list_after_start<F, R>(
    context: &mut Context,
    start_loc: usize,
    start_token: Tok,
    end_token: Tok,
    parse_list_item: F,
    item_description: &str,
) -> Result<Vec<R>, Box<Diagnostic>>
where
    F: Fn(&mut Context) -> Result<R, Box<Diagnostic>>,
{
    adjust_token(context.tokens, end_token);
    if match_token(context.tokens, end_token)? {
        return Ok(vec![]);
    }
    let mut v = vec![];
    loop {
        if context.tokens.peek() == Tok::Comma {
            let current_loc = context.tokens.start_loc();
            let loc = make_loc(context.tokens.file_hash(), current_loc, current_loc);
            return Err(Box::new(diag!(
                Syntax::UnexpectedToken,
                (loc, format!("Expected {}", item_description))
            )));
        }
        v.push(parse_list_item(context)?);
        adjust_token(context.tokens, end_token);
        if match_token(context.tokens, end_token)? {
            break Ok(v);
        }
        if !match_token(context.tokens, Tok::Comma)? {
            let current_loc = context.tokens.start_loc();
            let loc = make_loc(context.tokens.file_hash(), current_loc, current_loc);
            let loc2 = make_loc(context.tokens.file_hash(), start_loc, start_loc);
            return Err(Box::new(diag!(
                Syntax::UnexpectedToken,
                (loc, format!("Expected '{}'", end_token)),
                (loc2, format!("To match this '{}'", start_token)),
            )));
        }
        adjust_token(context.tokens, end_token);
        if match_token(context.tokens, end_token)? {
            break Ok(v);
        }
    }
}

// Parse a list of items, without specified start and end tokens, and the separator determined by
// the passed function `parse_list_continue`.
fn parse_list<C, F, R>(
    context: &mut Context,
    mut parse_list_continue: C,
    parse_list_item: F,
) -> Result<Vec<R>, Box<Diagnostic>>
where
    C: FnMut(&mut Context) -> Result<bool, Box<Diagnostic>>,
    F: Fn(&mut Context) -> Result<R, Box<Diagnostic>>,
{
    let mut v = vec![];
    loop {
        v.push(parse_list_item(context)?);
        if !parse_list_continue(context)? {
            break Ok(v);
        }
    }
}

//**************************************************************************************************
// Identifiers, Addresses, and Names
//**************************************************************************************************

// Parse an identifier:
//      Identifier = <IdentifierValue>
fn parse_identifier(context: &mut Context) -> Result<Name, Box<Diagnostic>> {
    if context.tokens.peek() != Tok::Identifier {
        return Err(unexpected_token_error(context.tokens, "an identifier"));
    }
    let start_loc = context.tokens.start_loc();
    let id = context.tokens.content().into();
    context.tokens.advance()?;
    let end_loc = context.tokens.previous_end_loc();
    Ok(spanned(context.tokens.file_hash(), start_loc, end_loc, id))
}

// Parse an identifier or an positional field
//     <Identifier> | [0-9]+
fn parse_identifier_or_positional_field(context: &mut Context) -> Result<Name, Box<Diagnostic>> {
    let start_loc = context.tokens.start_loc();
    let id: Symbol = context.tokens.content().into();
    if !(context.tokens.peek() == Tok::Identifier || next_token_is_positional_field(context)) {
        return Err(unexpected_token_error(
            context.tokens,
            &format!(
                "an identifier {}",
                if context.env.flags().lang_v2() {
                    "or a positional field `0`, `1`, ..."
                } else {
                    ""
                }
            ),
        ));
    }
    let is_positional_field = context.tokens.peek() == Tok::NumValue;
    context.tokens.advance()?;
    let end_loc = context.tokens.previous_end_loc();
    let loc = make_loc(context.tokens.file_hash(), start_loc, end_loc);
    if is_positional_field {
        require_move_2(context, loc, "positional field");
    }
    Ok(Spanned::new(loc, id))
}

fn next_token_is_positional_field(context: &mut Context) -> bool {
    if context.tokens.peek() == Tok::NumValue {
        let id: Symbol = context.tokens.content().into();
        id.as_str().chars().all(|c| c.is_ascii_digit())
    } else {
        false
    }
}

// Parse a numerical address value
//     NumericalAddress = <Number>
fn parse_address_bytes(
    context: &mut Context,
) -> Result<Spanned<NumericalAddress>, Box<Diagnostic>> {
    let loc = current_token_loc(context.tokens);
    let addr_res = NumericalAddress::parse_str(context.tokens.content());
    consume_token(context.tokens, Tok::NumValue)?;
    let addr_ = match addr_res {
        Ok(addr_) => addr_,
        Err(msg) => {
            context
                .env
                .add_diag(diag!(Syntax::InvalidAddress, (loc, msg)));
            NumericalAddress::DEFAULT_ERROR_ADDRESS
        },
    };
    Ok(sp(loc, addr_))
}

// Parse the beginning of an access, either an address or an identifier:
//      LeadingNameAccess = <NumericalAddress> | <Identifier>
fn parse_leading_name_access(
    context: &mut Context,
    allow_wildcard: bool,
) -> Result<LeadingNameAccess, Box<Diagnostic>> {
    parse_leading_name_access_(context, allow_wildcard, || "an address or an identifier")
}

// Parse the beginning of an access, either an address or an identifier with a specific description
fn parse_leading_name_access_<'a, F: FnOnce() -> &'a str>(
    context: &mut Context,
    allow_wildcard: bool,
    item_description: F,
) -> Result<LeadingNameAccess, Box<Diagnostic>> {
    match context.tokens.peek() {
        Tok::Identifier => {
            let loc = current_token_loc(context.tokens);
            let n = parse_identifier(context)?;
            Ok(sp(loc, LeadingNameAccess_::Name(n)))
        },
        Tok::Star if allow_wildcard => {
            let name = advance_wildcard_name(context)?;
            Ok(sp(name.loc, LeadingNameAccess_::Name(name)))
        },
        Tok::NumValue => {
            let sp!(loc, addr) = parse_address_bytes(context)?;
            Ok(sp(loc, LeadingNameAccess_::AnonymousAddress(addr)))
        },
        _ => Err(unexpected_token_error(context.tokens, item_description())),
    }
}

fn advance_wildcard_name(context: &mut Context) -> Result<Name, Box<Diagnostic>> {
    let loc = current_token_loc(context.tokens);
    context.tokens.advance()?;
    Ok(Name::new(loc, Symbol::from("*")))
}

// Parse a variable name:
//      Var = <Identifier>
fn parse_var(context: &mut Context) -> Result<Var, Box<Diagnostic>> {
    Ok(Var(parse_identifier(context)?))
}

// Parse a field name:
//      Field = <Identifier>
fn parse_field(context: &mut Context) -> Result<Field, Box<Diagnostic>> {
    Ok(Field(parse_identifier(context)?))
}

// Parse a module name:
//      ModuleName = <Identifier>
fn parse_module_name(context: &mut Context) -> Result<ModuleName, Box<Diagnostic>> {
    Ok(ModuleName(parse_identifier(context)?))
}

// Parse a module identifier:
//      ModuleIdent = <LeadingNameAccess> "::" <ModuleName>
fn parse_module_ident(context: &mut Context) -> Result<ModuleIdent, Box<Diagnostic>> {
    let start_loc = context.tokens.start_loc();
    let address = parse_leading_name_access(context, false)?;

    consume_token_(
        context.tokens,
        Tok::ColonColon,
        start_loc,
        " after an address in a module identifier",
    )?;
    let module = parse_module_name(context)?;
    let end_loc = context.tokens.previous_end_loc();
    let loc = make_loc(context.tokens.file_hash(), start_loc, end_loc);
    Ok(sp(loc, ModuleIdent_ { address, module }))
}

// Parse a module access (a variable, struct type, or function):
//      NameAccessChain = <LeadingNameAccess> ( "::" <Identifier> ( "::" <Identifier> )? )?
// If `allow_wildcard` is true, `*` will be accepted as an identifier.
fn parse_name_access_chain<'a, F: FnOnce() -> &'a str>(
    context: &mut Context,
    allow_wildcard: bool,
    item_description: F,
) -> Result<NameAccessChain, Box<Diagnostic>> {
    let start_loc = context.tokens.start_loc();
    let access = parse_name_access_chain_(context, allow_wildcard, item_description)?;
    let end_loc = context.tokens.previous_end_loc();
    Ok(spanned(
        context.tokens.file_hash(),
        start_loc,
        end_loc,
        access,
    ))
}

// Parse a module access with a specific description. If `allow_wildcard` is true, allows
// wildcards (`*`) for identifiers.
fn parse_name_access_chain_<'a, F: FnOnce() -> &'a str>(
    context: &mut Context,
    allow_wildcard: bool,
    item_description: F,
) -> Result<NameAccessChain_, Box<Diagnostic>> {
    let start_loc = context.tokens.start_loc();
    let ln = parse_leading_name_access_(context, allow_wildcard, item_description)?;
    let ln = match ln {
        // A name by itself is a valid access chain
        sp!(_, LeadingNameAccess_::Name(n1)) if context.tokens.peek() != Tok::ColonColon => {
            return Ok(NameAccessChain_::One(n1))
        },
        ln => ln,
    };

    consume_token_(
        context.tokens,
        Tok::ColonColon,
        start_loc,
        " after an address in a module access chain",
    )?;
    let n2 = parse_identifier_or_possibly_wildcard(context, allow_wildcard)?;
    if context.tokens.peek() != Tok::ColonColon {
        return Ok(NameAccessChain_::Two(ln, n2));
    }
    let ln_n2_loc = make_loc(
        context.tokens.file_hash(),
        start_loc,
        context.tokens.previous_end_loc(),
    );
    consume_token(context.tokens, Tok::ColonColon)?;
    let n3 = parse_identifier_or_possibly_wildcard(context, allow_wildcard)?;
    if context.tokens.peek() != Tok::ColonColon {
        return Ok(NameAccessChain_::Three(sp(ln_n2_loc, (ln, n2)), n3));
    }
    consume_token(context.tokens, Tok::ColonColon)?;
    let n4 = parse_identifier_or_possibly_wildcard(context, allow_wildcard)?;
    require_move_2(context, n4.loc, "fully qualified variant name");
    Ok(NameAccessChain_::Four(sp(ln_n2_loc, (ln, n2)), n3, n4))
}

fn parse_identifier_or_possibly_wildcard(
    context: &mut Context,
    allow_wildcard: bool,
) -> Result<Name, Box<Diagnostic>> {
    match context.tokens.peek() {
        Tok::Identifier => parse_identifier(context),
        Tok::Star if allow_wildcard => advance_wildcard_name(context),
        _ => Err(unexpected_token_error(
            context.tokens,
            if allow_wildcard {
                "an identifier or wildcard"
            } else {
                "an identifier"
            },
        )),
    }
}

//**************************************************************************************************
// Modifiers
//**************************************************************************************************

struct Modifiers {
    visibility: Option<Visibility>,
    entry: Option<Loc>,
    native: Option<Loc>,
}

impl Modifiers {
    fn empty() -> Self {
        Self {
            visibility: None,
            entry: None,
            native: None,
        }
    }
}

// Parse module member modifiers: visiblility and native.
// The modifiers are also used for script-functions
//      ModuleMemberModifiers = <ModuleMemberModifier>*
//      ModuleMemberModifier = <Visibility> | "native" | "entry"
// ModuleMemberModifiers checks for uniqueness, meaning each individual ModuleMemberModifier can
// appear only once
fn parse_module_member_modifiers(context: &mut Context) -> Result<Modifiers, Box<Diagnostic>> {
    let check_previous_vis = |context: &mut Context, mods: &mut Modifiers, vis: &Visibility| {
        if let Some(prev_vis) = &mods.visibility {
            let msg = "Duplicate visibility modifier".to_string();
            let prev_msg = "Visibility modifier previously given here".to_string();
            context.env.add_diag(diag!(
                Declarations::DuplicateItem,
                (vis.loc().unwrap(), msg),
                (prev_vis.loc().unwrap(), prev_msg),
            ));
        }
    };
    let mut mods = Modifiers::empty();
    loop {
        match context.tokens.peek() {
            Tok::Public => {
                let vis = parse_visibility(context)?;
                check_previous_vis(context, &mut mods, &vis);
                mods.visibility = Some(vis)
            },
            Tok::Friend => {
                let loc = current_token_loc(context.tokens);
                context.tokens.advance()?;
                require_move_2(context, loc, "direct `friend` declaration");
                let vis = Visibility::Friend(loc);
                check_previous_vis(context, &mut mods, &vis);
                mods.visibility = Some(vis)
            },
            Tok::Identifier if context.tokens.content() == "package" => {
                let loc = current_token_loc(context.tokens);
                context.tokens.advance()?;
                require_move_2(context, loc, "direct `package` declaration");
                let vis = Visibility::Package(loc);
                check_previous_vis(context, &mut mods, &vis);
                mods.visibility = Some(vis)
            },
            Tok::Native => {
                let loc = current_token_loc(context.tokens);
                context.tokens.advance()?;
                if let Some(prev_loc) = mods.native {
                    let msg = "Duplicate 'native' modifier".to_string();
                    let prev_msg = "'native' modifier previously given here".to_string();
                    context.env.add_diag(diag!(
                        Declarations::DuplicateItem,
                        (loc, msg),
                        (prev_loc, prev_msg)
                    ))
                }
                mods.native = Some(loc)
            },
            Tok::Identifier if context.tokens.content() == ENTRY_MODIFIER => {
                let loc = current_token_loc(context.tokens);
                context.tokens.advance()?;
                if let Some(prev_loc) = mods.entry {
                    let msg = format!("Duplicate '{}' modifier", ENTRY_MODIFIER);
                    let prev_msg = format!("'{}' modifier previously given here", ENTRY_MODIFIER);
                    context.env.add_diag(diag!(
                        Declarations::DuplicateItem,
                        (loc, msg),
                        (prev_loc, prev_msg)
                    ))
                }
                mods.entry = Some(loc)
            },
            _ => break,
        }
    }
    Ok(mods)
}

// Parse a function visibility modifier:
//      Visibility = "public" ( "(" "script" | "friend" | "package" ")" )?
// Notice that "package" and "friend" visibility can also directly be provided
// without "public" in declarations, but this is not handled by this function
fn parse_visibility(context: &mut Context) -> Result<Visibility, Box<Diagnostic>> {
    let start_loc = context.tokens.start_loc();
    consume_token(context.tokens, Tok::Public)?;
    let sub_public_vis = if match_token(context.tokens, Tok::LParen)? {
        let sub_token = context.tokens.peek();
        let sub_token_content = context.tokens.content();
        context.tokens.advance()?;
        if sub_token != Tok::RParen {
            consume_token(context.tokens, Tok::RParen)?;
        }
        Some((sub_token, sub_token_content))
    } else {
        None
    };
    let end_loc = context.tokens.previous_end_loc();
    // this loc will cover the span of 'public' or 'public(...)' in entirety
    let loc = make_loc(context.tokens.file_hash(), start_loc, end_loc);
    Ok(match sub_public_vis {
        None => Visibility::Public(loc),
        Some((Tok::Script, _)) => Visibility::Script(loc),
        Some((Tok::Friend, _)) => Visibility::Friend(loc),
        Some((Tok::Identifier, "package")) => {
            require_move_2(context, loc, "public(package) visibility");
            Visibility::Package(loc)
        },
        _ => {
            let msg = format!(
                "Invalid visibility modifier. Consider removing it or using '{}', '{}', or '{}'",
                Visibility::PUBLIC,
                Visibility::FRIEND,
                Visibility::PACKAGE,
            );
            return Err(Box::new(diag!(Syntax::UnexpectedToken, (loc, msg))));
        },
    })
}

// Parse an attribute value. Either a value literal or a module access
//      AttributeValue =
//          <Value>
//          | <NameAccessChain>
fn parse_attribute_value(context: &mut Context) -> Result<AttributeValue, Box<Diagnostic>> {
    if let Some(v) = maybe_parse_value(context)? {
        return Ok(sp(v.loc, AttributeValue_::Value(v)));
    }

    let ma = parse_name_access_chain(context, false, || "attribute name value")?;
    Ok(sp(ma.loc, AttributeValue_::ModuleAccess(ma)))
}

// Parse a single attribute
//      Attribute =
//          <AttributeName>
//          | <AttributeName> "=" <AttributeValue>
//          | <AttributeName> "(" Comma<Attribute> ")"
//      AttributeName = <Identifier> ( "::" Identifier )* // merged into one identifier
fn parse_attribute(context: &mut Context) -> Result<Attribute, Box<Diagnostic>> {
    let start_loc = context.tokens.start_loc();
    let mut n = parse_identifier(context)?;
    while match_token(context.tokens, Tok::ColonColon)? {
        let n1 = parse_identifier(context)?;
        let id = Symbol::from(format!("{}::{}", n.value.as_str(), n1.value.as_str()));
        let end_loc = context.tokens.previous_end_loc();
        n = spanned(context.tokens.file_hash(), start_loc, end_loc, id);
    }
    let attr_ = match context.tokens.peek() {
        Tok::Equal => {
            context.tokens.advance()?;
            Attribute_::Assigned(n, Box::new(parse_attribute_value(context)?))
        },
        Tok::LParen => {
            let args_ = parse_comma_list(
                context,
                Tok::LParen,
                Tok::RParen,
                parse_attribute,
                "attribute",
            )?;
            let end_loc = context.tokens.previous_end_loc();
            Attribute_::Parameterized(
                n,
                spanned(context.tokens.file_hash(), start_loc, end_loc, args_),
            )
        },
        _ => Attribute_::Name(n),
    };
    let end_loc = context.tokens.previous_end_loc();
    Ok(spanned(
        context.tokens.file_hash(),
        start_loc,
        end_loc,
        attr_,
    ))
}

// Parse attributes. Used to annotate a variety of AST nodes
//      Attributes = ("#" "[" Comma<Attribute> "]")*
fn parse_attributes(context: &mut Context) -> Result<Vec<Attributes>, Box<Diagnostic>> {
    let mut attributes_vec = vec![];
    while let Tok::NumSign = context.tokens.peek() {
        let start_loc = context.tokens.start_loc();
        context.tokens.advance()?;
        let attributes_ = parse_comma_list(
            context,
            Tok::LBracket,
            Tok::RBracket,
            parse_attribute,
            "attribute",
        )?;
        let end_loc = context.tokens.previous_end_loc();
        attributes_vec.push(spanned(
            context.tokens.file_hash(),
            start_loc,
            end_loc,
            attributes_,
        ))
    }
    Ok(attributes_vec)
}

//**************************************************************************************************
// Fields and Bindings
//**************************************************************************************************

// Parse a field name optionally followed by a colon and an expression argument:
//      ExpField = <Field> <":" <Exp>>?
fn parse_exp_field(context: &mut Context) -> Result<(Field, Exp), Box<Diagnostic>> {
    let f = parse_field(context)?;
    let arg = if match_token(context.tokens, Tok::Colon)? {
        parse_exp(context)?
    } else {
        sp(
            f.loc(),
            Exp_::Name(sp(f.loc(), NameAccessChain_::One(f.0)), None),
        )
    };
    Ok((f, arg))
}

// Parse a field name optionally followed by a colon and a binding:
//      BindField = <Field> <":" <Bind>>?
//
// If the binding is not specified, the default is to use a variable
// with the same name as the field.
fn parse_bind_field(context: &mut Context) -> Result<(Field, Bind), Box<Diagnostic>> {
    let f = parse_field(context)?;
    let arg = if match_token(context.tokens, Tok::Colon)? {
        parse_bind(context)?
    } else {
        let v = Var(f.0);
        sp(v.loc(), Bind_::Var(v))
    };
    Ok((f, arg))
}

// Parse an optionally typed binding:
//     TypedBind = Bind ( ":" <Type> )?
fn parse_typed_bind(context: &mut Context) -> Result<TypedBind, Box<Diagnostic>> {
    let start_loc = context.tokens.start_loc();
    let bind = parse_bind(context)?;
    let ty_opt = if match_token(context.tokens, Tok::Colon)? {
        Some(parse_type(context)?)
    } else {
        None
    };
    let end_loc = context.tokens.previous_end_loc();
    let typed_bind_ = TypedBind_(bind, ty_opt);
    Ok(spanned(
        context.tokens.file_hash(),
        start_loc,
        end_loc,
        typed_bind_,
    ))
}

// Parse a binding:
//      Bind =
//          <Var>
//          | <NameAccessChain> <OptionalTypeArgs> "{" Comma<BindFieldOrDotDot> "}"
//          | <NameAccessChain> <OptionalTypeArgs> "(" Comma<BindOrDotDot> "," ")"
//          | <NameAccessChain> <OptionalTypeArgs>
fn parse_bind(context: &mut Context) -> Result<Bind, Box<Diagnostic>> {
    let start_loc = context.tokens.start_loc();
    if context.tokens.peek() == Tok::Identifier {
        let next_tok = context.tokens.lookahead()?;
        if next_tok != Tok::LBrace
            && next_tok != Tok::LParen
            && next_tok != Tok::Less
            && next_tok != Tok::ColonColon
        {
            let v = Bind_::Var(parse_var(context)?);
            let end_loc = context.tokens.previous_end_loc();
            return Ok(spanned(context.tokens.file_hash(), start_loc, end_loc, v));
        }
    }
    // The item description specified here should include the special case above for
    // variable names, because if the current context cannot be parsed as a struct name
    // it is possible that the user intention was to use a variable name.
    let ty = parse_name_access_chain(context, false, || "a variable or struct or variant name")?;
    let ty_args = parse_optional_type_args(context)?;

    let unpack = if !context.env.flags().lang_v2() || context.tokens.peek() == Tok::LBrace {
        let args = parse_comma_list(
            context,
            Tok::LBrace,
            Tok::RBrace,
            parse_bind_field_or_dotdot,
            "a field binding",
        )?;
        Bind_::Unpack(Box::new(ty), ty_args, args)
    } else if context.tokens.peek() == Tok::LParen {
        let start_loc = context.tokens.start_loc();
        let args = parse_comma_list(
            context,
            Tok::LParen,
            Tok::RParen,
            parse_bind_or_dotdot,
            "a positional field binding",
        )?;
        let end_loc = context.tokens.previous_end_loc();
        let loc = make_loc(context.tokens.file_hash(), start_loc, end_loc);
        require_move_2(context, loc, "positional field");
        Bind_::PositionalUnpack(Box::new(ty), ty_args, args)
    } else {
        Bind_::Unpack(Box::new(ty), ty_args, vec![])
    };
    let end_loc = context.tokens.previous_end_loc();
    Ok(spanned(
        context.tokens.file_hash(),
        start_loc,
        end_loc,
        unpack,
    ))
}

// Parse a list of bindings, which can be zero, one, or more bindings:
//      BindList =
//          <Bind>
//          | "(" Comma<Bind> ")"
//
// The list is enclosed in parenthesis, except that the parenthesis are
// optional if there is a single Bind.
fn parse_bind_list(context: &mut Context) -> Result<BindList, Box<Diagnostic>> {
    let start_loc = context.tokens.start_loc();
    let b = if context.tokens.peek() != Tok::LParen {
        vec![parse_bind(context)?]
    } else {
        parse_comma_list(
            context,
            Tok::LParen,
            Tok::RParen,
            parse_bind,
            "a variable or structure binding",
        )?
    };
    let end_loc = context.tokens.previous_end_loc();
    Ok(spanned(context.tokens.file_hash(), start_loc, end_loc, b))
}

/// Parse a <BindField> or a ".."
/// <BindFieldOrDotDot> = <BindField> | ".."
fn parse_bind_field_or_dotdot(context: &mut Context) -> Result<BindFieldOrDotDot, Box<Diagnostic>> {
    if context.tokens.peek() == Tok::PeriodPeriod {
        let loc = current_token_loc(context.tokens);
        require_move_2(context, loc, "`..` patterns");
        context.tokens.advance()?;
        Ok(sp(loc, BindFieldOrDotDot_::DotDot))
    } else {
        let (f, b) = parse_bind_field(context)?;
        Ok(sp(f.loc(), BindFieldOrDotDot_::FieldBind(f, b)))
    }
}

/// Parse a <Bind> or a ".."
/// <BindOrDotDot> = <Bind> | ".."
fn parse_bind_or_dotdot(context: &mut Context) -> Result<BindOrDotDot, Box<Diagnostic>> {
    if context.tokens.peek() == Tok::PeriodPeriod {
        let loc = current_token_loc(context.tokens);
        require_move_2(context, loc, "`..` patterns");
        context.tokens.advance()?;
        Ok(sp(loc, BindOrDotDot_::DotDot))
    } else {
        let b = parse_bind(context)?;
        Ok(sp(b.loc, BindOrDotDot_::Bind(b)))
    }
}

// Parse a list of bindings for lambda.
//      LambdaBindList =
//          "|" Comma<TypedBind> "|"
fn parse_lambda_bind_list(context: &mut Context) -> Result<TypedBindList, Box<Diagnostic>> {
    let start_loc = context.tokens.start_loc();
    let b = parse_comma_list(
        context,
        Tok::Pipe,
        Tok::Pipe,
        parse_typed_bind,
        "a variable or structure binding",
    )?;
    let end_loc = context.tokens.previous_end_loc();
    Ok(spanned(context.tokens.file_hash(), start_loc, end_loc, b))
}

//**************************************************************************************************
// Values
//**************************************************************************************************

// Parse a byte string:
//      ByteString = <ByteStringValue>
fn parse_byte_string(context: &mut Context) -> Result<Value_, Box<Diagnostic>> {
    if context.tokens.peek() != Tok::ByteStringValue {
        return Err(unexpected_token_error(
            context.tokens,
            "a byte string value",
        ));
    }
    let s = context.tokens.content();
    let text = Symbol::from(&s[2..s.len() - 1]);
    let value_ = if s.starts_with("x\"") {
        Value_::HexString(text)
    } else {
        assert!(s.starts_with("b\""));
        Value_::ByteString(text)
    };
    context.tokens.advance()?;
    Ok(value_)
}

// Parse a value:
//      Value =
//          "@" <LeadingAccessName>
//          | "true"
//          | "false"
//          | <Number>
//          | <NumberTyped>
//          | <ByteString>
fn maybe_parse_value(context: &mut Context) -> Result<Option<Value>, Box<Diagnostic>> {
    let start_loc = context.tokens.start_loc();
    let val = match context.tokens.peek() {
        Tok::AtSign => {
            context.tokens.advance()?;
            let addr = parse_leading_name_access(context, false)?;
            Value_::Address(addr)
        },
        Tok::True => {
            context.tokens.advance()?;
            Value_::Bool(true)
        },
        Tok::False => {
            context.tokens.advance()?;
            Value_::Bool(false)
        },
        Tok::NumValue => {
            //  If the number is followed by "::", parse it as the beginning of an address access
            if let Ok(Tok::ColonColon) = context.tokens.lookahead() {
                return Ok(None);
            }
            let num = context.tokens.content().into();
            context.tokens.advance()?;
            Value_::Num(num)
        },
        Tok::NumTypedValue => {
            let num = context.tokens.content().into();
            context.tokens.advance()?;
            Value_::Num(num)
        },

        Tok::ByteStringValue => parse_byte_string(context)?,
        _ => return Ok(None),
    };
    let end_loc = context.tokens.previous_end_loc();
    Ok(Some(spanned(
        context.tokens.file_hash(),
        start_loc,
        end_loc,
        val,
    )))
}

fn parse_value(context: &mut Context) -> Result<Value, Box<Diagnostic>> {
    Ok(maybe_parse_value(context)?.expect("parse_value called with invalid token"))
}

//**************************************************************************************************
// Sequences
//**************************************************************************************************

// Parse a sequence item:
//      SequenceItem =
//          <Exp>
//          | "let" <BindList> (":" <Type>)? ("=" <Exp>)?
fn parse_sequence_item(context: &mut Context) -> Result<SequenceItem, Box<Diagnostic>> {
    let start_loc = context.tokens.start_loc();
    let item = if match_token(context.tokens, Tok::Let)? {
        let b = parse_bind_list(context)?;
        let ty_opt = if match_token(context.tokens, Tok::Colon)? {
            Some(parse_type(context)?)
        } else {
            None
        };
        if match_token(context.tokens, Tok::Equal)? {
            let e = parse_exp(context)?;
            SequenceItem_::Bind(b, ty_opt, Box::new(e))
        } else {
            SequenceItem_::Declare(b, ty_opt)
        }
    } else {
        let e = parse_exp(context)?;
        SequenceItem_::Seq(Box::new(e))
    };
    let end_loc = context.tokens.previous_end_loc();
    Ok(spanned(
        context.tokens.file_hash(),
        start_loc,
        end_loc,
        item,
    ))
}

// Parse a sequence:
//      Sequence = <UseDecl>* (<SequenceItem> ";")* <Exp>? "}"
//
// Note that this does not include the opening brace of a block but it
// does consume the closing right brace.
fn parse_sequence(context: &mut Context) -> Result<Sequence, Box<Diagnostic>> {
    let mut uses = vec![];
    while context.tokens.peek() == Tok::Use {
        uses.push(parse_use_decl(vec![], context)?);
    }

    let mut seq: Vec<SequenceItem> = vec![];
    let mut last_semicolon_loc = None;
    let mut eopt = None;
    while context.tokens.peek() != Tok::RBrace {
        let item = parse_sequence_item(context)?;
        if context.tokens.peek() == Tok::RBrace {
            // If the sequence ends with an expression that is not
            // followed by a semicolon, split out that expression
            // from the rest of the SequenceItems.
            match item.value {
                SequenceItem_::Seq(e) => {
                    eopt = Some(Spanned {
                        loc: item.loc,
                        value: e.value,
                    });
                },
                _ => return Err(unexpected_token_error(context.tokens, "';'")),
            }
            break;
        }
        seq.push(item);
        last_semicolon_loc = Some(current_token_loc(context.tokens));
        consume_token(context.tokens, Tok::Semicolon)?;
    }
    context.tokens.advance()?; // consume the RBrace
    Ok((uses, seq, last_semicolon_loc, Box::new(eopt)))
}

//**************************************************************************************************
// Expressions
//**************************************************************************************************

// Parse an expression term:
//      Term =
//          "break"
//          | "continue"
//          | "vector" ('<' Comma<Type> ">")? "[" Comma<Exp> "]"
//          | <Value>
//          | "(" Comma<Exp> ")"
//          | "(" <Exp> ":" <Type> ")"
//          | "(" <Exp> "as" <Type> ")"
//          | "{" <Sequence>
//          | "if" "(" <Exp> ")" <Exp> "else" "{" <Exp> "}"
//          | "if" "(" <Exp> ")" "{" <Exp> "}"
//          | "if" "(" <Exp> ")" <Exp> ("else" <Exp>)?
//          | "while" "(" <Exp> ")" "{" <Exp> "}"
//          | "while" "(" <Exp> ")" <Exp> (SpecBlock)?
//          | "loop" <Exp>
//          | "loop" "{" <Exp> "}"
//          | <Match>
//          | "return" "{" <Exp> "}"
//          | "return" <Exp>?
//          | "abort" "{" <Exp> "}"
//          | "abort" <Exp>
//          | "for" "(" <Exp> "in" <Exp> ".." <Exp> ")" "{" <Exp> "}"
//          | <NameExp>
//          | <Term> <CallArgs>              // --> ExpCall
fn parse_term(context: &mut Context) -> Result<Exp, Box<Diagnostic>> {
    const VECTOR_IDENT: &str = "vector";
    const FOR_IDENT: &str = "for";

    let start_loc = context.tokens.start_loc();
    let term = match context.tokens.peek() {
        tok if is_control_exp(tok) => {
            let (control_exp, ends_in_block) = parse_control_exp(context)?;
            if !ends_in_block || at_end_of_exp(context) {
                return Ok(control_exp);
            }
            return parse_binop_exp(context, control_exp, /* min_prec */ 1);
        },
        Tok::Identifier
            if context.tokens.content() == "match"
                && context.tokens.lookahead()? == Tok::LParen =>
        {
            // Match always ends in block (see above case for comparison)
            let match_exp = parse_match_exp(context)?;
            if at_end_of_exp(context) {
                return Ok(match_exp);
            }
            return parse_binop_exp(context, match_exp, 1);
        },
        Tok::Identifier
            if context.tokens.content() == FOR_IDENT
                && matches!(context.tokens.lookahead_nth(0), Ok(Tok::LParen))
                && matches!(context.tokens.lookahead_nth(2), Ok(Tok::Identifier)) =>
        {
            let (control_exp, _) = parse_for_loop(context)?;
            // for loop isn't useful in an expression, so we ignore second result from
            // `parse_for_loop` and never call `parse_binop_exp` (as might be done for some other
            return Ok(control_exp);
        },
        Tok::Break => {
            context.tokens.advance()?;
            let label = parse_optional_label(context)?;

            if at_start_of_exp(context) {
                let mut diag = unexpected_token_error(context.tokens, "the end of an expression");
                diag.add_note("'break' with a value is not yet supported");
                return Err(diag);
            }
            Exp_::Break(label)
        },

        Tok::Continue => {
            context.tokens.advance()?;
            let label = parse_optional_label(context)?;
            Exp_::Continue(label)
        },

        Tok::Identifier
            if context.tokens.content() == VECTOR_IDENT
                && matches!(context.tokens.lookahead(), Ok(Tok::Less | Tok::LBracket)) =>
        {
            consume_identifier(context.tokens, VECTOR_IDENT)?;
            let vec_end_loc = context.tokens.previous_end_loc();
            let vec_loc = make_loc(context.tokens.file_hash(), start_loc, vec_end_loc);
            let targs_start_loc = context.tokens.start_loc();
            let tys_opt = parse_optional_type_args(context).map_err(|diag| {
                let targ_loc =
                    make_loc(context.tokens.file_hash(), targs_start_loc, targs_start_loc);
                add_type_args_ambiguity_label(targ_loc, diag)
            })?;
            let args_start_loc = context.tokens.start_loc();
            let args_ = parse_comma_list(
                context,
                Tok::LBracket,
                Tok::RBracket,
                parse_exp,
                "a vector argument expression",
            )?;
            let args_end_loc = context.tokens.previous_end_loc();
            let args = spanned(
                context.tokens.file_hash(),
                args_start_loc,
                args_end_loc,
                args_,
            );
            Exp_::Vector(vec_loc, tys_opt, args)
        },

        Tok::Identifier => parse_name_exp(context)?,

        Tok::NumValue => {
            // Check if this is a ModuleIdent (in a ModuleAccess).
            if context.tokens.lookahead()? == Tok::ColonColon {
                parse_name_exp(context)?
            } else {
                Exp_::Value(parse_value(context)?)
            }
        },

        Tok::AtSign | Tok::True | Tok::False | Tok::NumTypedValue | Tok::ByteStringValue => {
            Exp_::Value(parse_value(context)?)
        },

        // "(" Comma<Exp> ")"
        // "(" <Exp> ":" <Type> ")"
        // "(" <Exp> "as" <Type> ")"
        // "(" <Exp> "is" <Type> ( "|" <Type> )* ")"
        Tok::LParen => {
            let list_loc = context.tokens.start_loc();
            context.tokens.advance()?; // consume the LParen
            if match_token(context.tokens, Tok::RParen)? {
                Exp_::Unit
            } else {
                // If there is a single expression inside the parens,
                // then it may be followed by a colon and a type annotation, an 'as' and a type,
                // or an 'is' and a list of variants.
                let e = parse_exp(context)?;
                if let Some(exp) =
                    parse_cast_or_test_exp(context, &e, /*allow_colon_exp*/ true)?
                {
                    consume_token(context.tokens, Tok::RParen)?;
                    exp
                } else {
                    if context.tokens.peek() != Tok::RParen {
                        consume_token(context.tokens, Tok::Comma)?;
                    }
                    let mut es = parse_comma_list_after_start(
                        context,
                        list_loc,
                        Tok::LParen,
                        Tok::RParen,
                        parse_exp,
                        "an expression",
                    )?;
                    if es.is_empty() {
                        e.value
                    } else {
                        es.insert(0, e);
                        Exp_::ExpList(es)
                    }
                }
            }
        },

        // "{" <Sequence>
        Tok::LBrace => {
            context.tokens.advance()?; // consume the LBrace
            Exp_::Block(parse_sequence(context)?)
        },

        Tok::Spec => {
            let spec_block = parse_spec_block(vec![], context)?;
            Exp_::Spec(spec_block)
        },

        _ => {
            return Err(unexpected_token_error(context.tokens, "an expression term"));
        },
    };
    let end_loc = context.tokens.previous_end_loc();
    let mut result_term = spanned(context.tokens.file_hash(), start_loc, end_loc, term);
    while matches!(context.tokens.peek(), Tok::LParen) {
        let args = parse_call_args(context)?;
        let term = Exp_::ExpCall(Box::new(result_term), args);
        let end_loc = context.tokens.previous_end_loc();
        result_term = spanned(context.tokens.file_hash(), start_loc, end_loc, term);
    }
    Ok(result_term)
}

fn parse_cast_or_test_exp(
    context: &mut Context,
    e: &Exp,
    allow_colon_exp: bool,
) -> Result<Option<Exp_>, Box<Diagnostic>> {
    if allow_colon_exp && match_token(context.tokens, Tok::Colon)? {
        let ty = parse_type(context)?;
        Ok(Some(Exp_::Annotate(Box::new(e.clone()), ty)))
    } else if match_token(context.tokens, Tok::As)? {
        let ty = parse_type(context)?;
        Ok(Some(Exp_::Cast(Box::new(e.clone()), ty)))
    } else if context.tokens.peek() == Tok::Identifier && context.tokens.content() == "is" {
        require_move_2(
            context,
            current_token_loc(context.tokens),
            "`is` expression",
        );
        context.tokens.advance()?;
        let types = parse_list(
            context,
            |ctx| match_token(ctx.tokens, Tok::Pipe),
            parse_type,
        )?;
        Ok(Some(Exp_::Test(Box::new(e.clone()), types)))
    } else {
        Ok(None)
    }
}

fn is_control_exp(tok: Tok) -> bool {
    matches!(
        tok,
        Tok::If | Tok::While | Tok::Loop | Tok::Return | Tok::Abort | Tok::Label
    )
}

fn parse_exp_or_control_sequence(context: &mut Context) -> Result<(Exp, bool), Box<Diagnostic>> {
    if let Tok::LBrace = context.tokens.peek() {
        let start_loc = context.tokens.start_loc();
        consume_token(context.tokens, Tok::LBrace)?;

        let block_ = Exp_::Block(parse_sequence(context)?);
        let end_loc = context.tokens.previous_end_loc();

        let exp = spanned(context.tokens.file_hash(), start_loc, end_loc, block_);

        Ok((exp, true))
    } else {
        Ok((parse_exp(context)?, false))
    }
}

fn parse_spec_while_loop(
    context: &mut Context,
    condition: Exp,
    ends_in_block: bool,
) -> Result<(Exp, bool), Box<Diagnostic>> {
    if context.tokens.peek() == Tok::Spec {
        let spec_seq = parse_spec_loop_invariant(context)?;
        let loc = condition.loc;
        let spec_block = Exp_::Block((vec![], vec![spec_seq], None, Box::new(Some(condition))));
        Ok((sp(loc, spec_block), true))
    } else {
        Ok((condition, ends_in_block))
    }
}

fn parse_spec_loop_invariant(context: &mut Context) -> Result<SequenceItem, Box<Diagnostic>> {
    // Parse a loop invariant. Also validate that only `invariant`
    // properties are contained in the spec block. This is
    // transformed into `while ({spec { .. }; cond) body`.
    let spec = parse_spec_block(vec![], context)?;
    for member in &spec.value.members {
        match member.value {
            // Ok
            SpecBlockMember_::Condition {
                kind: sp!(_, SpecConditionKind_::Invariant(..)),
                ..
            } => (),
            _ => {
                return Err(Box::new(diag!(
                    Syntax::InvalidSpecBlockMember,
                    (member.loc, "only 'invariant' allowed here")
                )))
            },
        }
    }
    Ok(sp(
        spec.loc,
        SequenceItem_::Seq(Box::new(sp(spec.loc, Exp_::Spec(spec)))),
    ))
}

// if there is a block, only parse the block, not any subsequent tokens
// e.g.           if (cond) e1 else { e2 } + 1
// should be,    (if (cond) e1 else { e2 }) + 1
// AND NOT,       if (cond) e1 else ({ e2 } + 1)
// But otherwise, if (cond) e1 else e2 + 1
// should be,     if (cond) e1 else (e2 + 1)
fn parse_control_exp(context: &mut Context) -> Result<(Exp, bool), Box<Diagnostic>> {
    let start_loc = context.tokens.start_loc();
    let label = parse_optional_label(context)?;
    if label.is_some() {
        consume_token(context.tokens, Tok::Colon)?;
        // Check here whether a label is actually allowed
        if !matches!(context.tokens.peek(), Tok::While | Tok::Loop) {
            return Err(unexpected_token_error(
                context.tokens,
                "one of: `while` or `loop`",
            ));
        }
    };
    let (exp_, ends_in_block) = match context.tokens.peek() {
        Tok::If => {
            context.tokens.advance()?;
            consume_token(context.tokens, Tok::LParen)?;
            let eb = Box::new(parse_exp(context)?);
            consume_token(context.tokens, Tok::RParen)?;
            let (et, ends_in_block) = parse_exp_or_control_sequence(context)?;
            let (ef, ends_in_block) = if match_token(context.tokens, Tok::Else)? {
                let (ef, ends_in_block) = parse_exp_or_control_sequence(context)?;
                (Some(Box::new(ef)), ends_in_block)
            } else {
                (None, ends_in_block)
            };
            (Exp_::IfElse(eb, Box::new(et), ef), ends_in_block)
        },
        Tok::While => {
            context.tokens.advance()?;
            consume_token(context.tokens, Tok::LParen)?;
            let econd = parse_exp(context)?;
            consume_token(context.tokens, Tok::RParen)?;
            let (eloop, ends_in_block) = parse_exp_or_control_sequence(context)?;
            let (econd, ends_in_block) = parse_spec_while_loop(context, econd, ends_in_block)?;
            (
                Exp_::While(label, Box::new(econd), Box::new(eloop)),
                ends_in_block,
            )
        },
        Tok::Loop => {
            context.tokens.advance()?;
            let (eloop, ends_in_block) = parse_exp_or_control_sequence(context)?;
            (Exp_::Loop(label, Box::new(eloop)), ends_in_block)
        },
        Tok::Return => {
            context.tokens.advance()?;
            let (e, ends_in_block) = if !at_start_of_exp(context) {
                (None, false)
            } else {
                let (e, ends_in_block) = parse_exp_or_control_sequence(context)?;
                (Some(Box::new(e)), ends_in_block)
            };
            (Exp_::Return(e), ends_in_block)
        },
        Tok::Abort => {
            context.tokens.advance()?;
            let (e, ends_in_block) = parse_exp_or_control_sequence(context)?;
            (Exp_::Abort(Box::new(e)), ends_in_block)
        },
        _ => unreachable!(),
    };
    let end_loc = context.tokens.previous_end_loc();
    let exp = spanned(context.tokens.file_hash(), start_loc, end_loc, exp_);
    Ok((exp, ends_in_block))
}

fn parse_optional_label(context: &mut Context) -> Result<Option<Label>, Box<Diagnostic>> {
    if context.tokens.peek() == Tok::Label {
        require_language_version(
            context,
            current_token_loc(context.tokens),
            LanguageVersion::V2_1,
            "loop labels are",
        );
        let label = Label(Name::new(
            current_token_loc(context.tokens),
            Symbol::from(context.tokens.content()),
        ));
        context.tokens.advance()?;
        Ok(Some(label))
    } else {
        Ok(None)
    }
}

// "for (iter in lower_bound..upper_bound) loop_body" transforms into
// let iter = lower_bound;
// let flag = false;
// while (true) {
//     if flag {
//         iter = iter + 1;
//     } else {
//         flag = true;
//     }
//     if (i < upper_bound) {
//         loop_body;
//     } else {
//         break
//     }
// }
fn parse_for_loop(context: &mut Context) -> Result<(Exp, bool), Box<Diagnostic>> {
    const FOR_IDENT: &str = "for";
    let start_loc = context.tokens.start_loc();

    assert!(
        matches!(context.tokens.peek(), Tok::Identifier)
            && context.tokens.content() == FOR_IDENT
            && matches!(context.tokens.lookahead(), Ok(Tok::LParen)),
        "Syntax Error. Use syntax: for (iter in lower_bound..upper_bound) loop_body."
    );
    context.tokens.advance()?;

    consume_token(context.tokens, Tok::LParen)?;
    let iter = parse_identifier(context)?;
    consume_identifier(context.tokens, "in")?;

    let lb = parse_unary_exp(context)?;
    consume_token(context.tokens, Tok::PeriodPeriod)?;
    let ub = parse_unary_exp(context)?;

    let spec_seq = if context.tokens.peek() == Tok::Spec {
        Some(parse_spec_loop_invariant(context)?)
    } else {
        None
    };

    consume_token(context.tokens, Tok::RParen)?;
    let (for_body, ends_in_block) = parse_exp_or_control_sequence(context)?;

    let end_loc = context.tokens.previous_end_loc();

    // Build corresponding expression:
    // let iter = lower_bound;
    // let ub_value = upper_bound_value;
    // flag = false;
    // loop {
    //     if (flag) {
    //         iter = iter + 1;
    //     } else {
    //         flag = true;
    //     };
    //     if (i < ub_value) {
    //         loop_body;
    //     } else break;
    // }
    let for_loc = make_loc(context.tokens.file_hash(), start_loc, end_loc);

    // Create assignment "let iter = lower_bound"
    let iter_bind = sp(for_loc, Bind_::Var(Var(iter)));
    let iter_bindlist = sp(for_loc, vec![iter_bind]);
    let iter_init = sp(
        iter.loc,
        SequenceItem_::Bind(iter_bindlist, None, Box::new(lb)),
    );

    // To create the declaration "let flag = false", first create the variable flag, and then assign it to false
    let flag_symb = Symbol::from(FOR_LOOP_UPDATE_ITER_FLAG);
    let flag = sp(for_loc, vec![sp(
        for_loc,
        Bind_::Var(Var(sp(for_loc, flag_symb))),
    )]);
    let false_exp = sp(for_loc, Exp_::Value(sp(for_loc, Value_::Bool(false))));
    let decl_flag = sp(
        for_loc,
        SequenceItem_::Bind(flag, None, Box::new(false_exp)),
    );

    // To create the declaration "let ub_value = upper_bound", first create the variable flag, and
    // then assign it to upper_bound
    let ub_value_symbol = Symbol::from(FOR_LOOP_UPPER_BOUND_VALUE);
    let ub_value_bindlist = sp(for_loc, vec![sp(
        for_loc,
        Bind_::Var(Var(sp(for_loc, ub_value_symbol))),
    )]);
    let ub_value_assignment = sp(
        for_loc,
        SequenceItem_::Bind(ub_value_bindlist, None, Box::new(ub)),
    );

    // Construct the increment "iter = iter + 1"
    let one_exp = sp(
        for_loc,
        Exp_::Value(sp(for_loc, Value_::Num(Symbol::from("1")))),
    );
    let op_add = sp(for_loc, BinOp_::Add);
    let iter_exp = sp(
        for_loc,
        Exp_::Name(sp(for_loc, NameAccessChain_::One(iter)), None),
    );
    let updated_exp = sp(
        for_loc,
        Exp_::BinopExp(Box::new(iter_exp.clone()), op_add, Box::new(one_exp)),
    );
    let update = sp(
        for_loc,
        Exp_::Assign(Box::new(iter_exp.clone()), None, Box::new(updated_exp)),
    );

    // Create the assignment "flag = true;"
    let flag_exp = sp(
        for_loc,
        Exp_::Name(
            sp(for_loc, NameAccessChain_::One(sp(for_loc, flag_symb))),
            None,
        ),
    );
    let true_exp = sp(for_loc, Exp_::Value(sp(for_loc, Value_::Bool(true))));
    let assign_iter = sp(
        for_loc,
        Exp_::Assign(Box::new(flag_exp.clone()), None, Box::new(true_exp.clone())),
    );

    // construct flag conditional "if (flag) { update; } else { flag = true; }"
    // let update = sp(
    //     for_loc,
    //     Exp_::Block((vec![], vec![update], None, Box::new(None))),
    // );
    let flag_conditional = sp(
        for_loc,
        Exp_::IfElse(
            Box::new(flag_exp.clone()),
            Box::new(update),
            Some(Box::new(assign_iter)),
        ),
    );
    let flag_conditional = sp(for_loc, SequenceItem_::Seq(Box::new(flag_conditional)));

    // Create the condition "iter < upper_bound"
    let op_le = sp(iter.loc, BinOp_::Lt);
    let ub_value_exp = sp(
        for_loc,
        Exp_::Name(
            sp(for_loc, NameAccessChain_::One(sp(for_loc, ub_value_symbol))),
            None,
        ),
    );
    let e = Exp_::BinopExp(Box::new(iter_exp), op_le, Box::new(ub_value_exp));
    let loop_condition = sp(iter.loc, e);

    // Create the "if (loop_condition) { loop_body; } else break"
    let loop_conditional = Exp_::IfElse(
        Box::new(loop_condition),
        Box::new(for_body),
        Some(Box::new(sp(for_loc, Exp_::Break(None)))),
    );
    let loop_conditional = sp(
        for_loc,
        SequenceItem_::Seq(Box::new(sp(for_loc, loop_conditional))),
    );

    let while_condition = match spec_seq {
        None => true_exp.clone(),
        Some(spec) => sp(
            for_loc,
            Exp_::Block((vec![], vec![spec], None, Box::new(Some(true_exp.clone())))),
        ),
    };
    let body = sp(
        for_loc,
        Exp_::Block((
            vec![],
            vec![flag_conditional, loop_conditional],
            None,
            Box::new(None),
        )),
    );
    let loop_body = sp(
        for_loc,
        Exp_::While(None, Box::new(while_condition), Box::new(body)),
    );
    let loop_body = sp(for_loc, SequenceItem_::Seq(Box::new(loop_body)));

    // construct the parsed for loop
    let parsed_for_loop = sp(
        for_loc,
        Exp_::Block((
            vec![],
            vec![iter_init, decl_flag, ub_value_assignment, loop_body],
            Some(for_loc),
            Box::new(None),
        )),
    );
    Ok((parsed_for_loop, ends_in_block))
}

// Match = "match" "(" <Exp> ")" "{" ( <MatchArm> ","? )* "}"
// MatchArm = <BindList> ( "if" <Exp> )? "=>" <Exp>
// If called, we know we are looking at `match (`.
fn parse_match_exp(context: &mut Context) -> Result<Exp, Box<Diagnostic>> {
    // We cannot uniquely determine this is actually a match expression
    // until we have seen the `match (exp) {` prefix. We parse the parts and
    // decide on the go whether to interpret this as a call to a function `match`.
    let start_loc = context.tokens.start_loc();
    let match_ident = parse_identifier(context)?;
    debug_assert!(match_ident.value.as_str() == "match");
    let start_lparen_loc = context.tokens.start_loc();
    assert!(consume_token(context.tokens, Tok::LParen).is_ok());
    if match_token(context.tokens, Tok::RParen)? {
        // Interpret as function call `match()`
        let end_loc = context.tokens.previous_end_loc();
        Ok(spanned(
            context.tokens.file_hash(),
            start_loc,
            end_loc,
            Exp_::Call(
                sp(match_ident.loc, NameAccessChain_::One(match_ident)),
                CallKind::Regular,
                None,
                spanned(
                    match_ident.loc.file_hash(),
                    start_lparen_loc,
                    end_loc,
                    vec![],
                ),
            ),
        ))
    } else {
        let exp = parse_exp(context)?;
        // As we have seen `match (exp`, now check whether we are looking at `match (exp) {`
        // to confirm match expression
        if (context.tokens.peek(), context.tokens.lookahead()?) == (Tok::RParen, Tok::LBrace) {
            require_move_2(context, match_ident.loc, "match expression");
            consume_token(context.tokens, Tok::RParen)?;
            consume_token(context.tokens, Tok::LBrace)?;
            let arms = parse_match_arms(context)?;
            consume_token(context.tokens, Tok::RBrace)?;
            Ok(spanned(
                context.tokens.file_hash(),
                start_loc,
                context.tokens.previous_end_loc(),
                Exp_::Match(Box::new(exp), arms),
            ))
        } else {
            // Interpret as a function call `match(arg1, .., argn)`
            let mut args = vec![exp];
            if context.tokens.peek() == Tok::Comma {
                // parse remaining `, arg2, arg3, ..)`
                args.append(&mut parse_comma_list(
                    context,
                    Tok::Comma,
                    Tok::RParen,
                    parse_exp,
                    "a call argument expression",
                )?);
            } else {
                consume_token(context.tokens, Tok::RParen)?;
            }
            let end_loc = context.tokens.previous_end_loc();
            Ok(spanned(
                match_ident.loc.file_hash(),
                start_loc,
                end_loc,
                Exp_::Call(
                    sp(match_ident.loc, NameAccessChain_::One(match_ident)),
                    CallKind::Regular,
                    None,
                    spanned(match_ident.loc.file_hash(), start_lparen_loc, end_loc, args),
                ),
            ))
        }
    }
}

fn parse_match_arms(
    context: &mut Context,
) -> Result<Vec<Spanned<(BindList, Option<Exp>, Exp)>>, Box<Diagnostic>> {
    let mut arms = vec![];
    while context.tokens.peek() != Tok::RBrace {
        let start_loc = context.tokens.start_loc();
        let bind_list = parse_bind_list(context)?;
        let cond = if match_token(context.tokens, Tok::If)? {
            Some(parse_exp(context)?)
        } else {
            None
        };
        consume_token(context.tokens, Tok::EqualGreater)?;
        let (body, body_is_block) = parse_exp_or_control_sequence(context)?;
        let next = context.tokens.peek();
        // Block based arms are optionally separated by comma, otherwise
        // a comma is required if not at end of the list.
        if (!body_is_block && next != Tok::RBrace) || next == Tok::Comma {
            consume_token(context.tokens, Tok::Comma)?
        }
        arms.push(spanned(
            context.tokens.file_hash(),
            start_loc,
            context.tokens.previous_end_loc(),
            (bind_list, cond, body),
        ))
    }
    Ok(arms)
}

// Parse a pack, call, or other reference to a name:
//      NameExp =
//          <NameAccessChain> <OptionalTypeArgs> "{" Comma<ExpField> "}"
//          | <NameAccessChain> <OptionalTypeArgs> <CallArgs>
//          | <NameAccessChain> "!" <CallArgs>
//          | <NameAccessChain> <OptionalTypeArgs>
fn parse_name_exp(context: &mut Context) -> Result<Exp_, Box<Diagnostic>> {
    let n = parse_name_access_chain(context, false, || {
        panic!("parse_name_exp with something other than a ModuleAccess")
    })?;

    // There's an ambiguity if the name is followed by a '<'. If there is no whitespace
    // after the name, treat it as the start of a list of type arguments. Otherwise
    // assume that the '<' is a boolean operator.
    let mut tys = None;
    let start_loc = context.tokens.start_loc();
    if context.tokens.peek() == Tok::Exclaim {
        context.tokens.advance()?;
        let rhs = parse_call_args(context)?;
        return Ok(Exp_::Call(n, CallKind::Macro, tys, rhs));
    }

    if context.tokens.peek() == Tok::Less && n.loc.end() as usize == start_loc {
        let loc = make_loc(context.tokens.file_hash(), start_loc, start_loc);
        tys = parse_optional_type_args(context)
            .map_err(|diag| add_type_args_ambiguity_label(loc, diag))?;
    }

    match context.tokens.peek() {
        // Pack: "{" Comma<ExpField> "}"
        Tok::LBrace => {
            let fs = parse_comma_list(
                context,
                Tok::LBrace,
                Tok::RBrace,
                parse_exp_field,
                "a field expression",
            )?;
            Ok(Exp_::Pack(n, tys, fs))
        },

        // Call: <CallArgs>
        Tok::Exclaim | Tok::LParen => {
            let rhs = parse_call_args(context)?;
            Ok(Exp_::Call(n, CallKind::Regular, tys, rhs))
        },

        // Other name reference...
        _ => Ok(Exp_::Name(n, tys)),
    }
}

// Parse the arguments to a call:
//      CallArgs =
//          "(" Comma<Exp> ")"
// Result is a list <args>
fn parse_call_args(context: &mut Context) -> Result<Spanned<Vec<Exp>>, Box<Diagnostic>> {
    let start_loc = context.tokens.start_loc();
    let args = parse_comma_list(
        context,
        Tok::LParen,
        Tok::RParen,
        parse_exp,
        "a call argument expression",
    )?;
    let end_loc = context.tokens.previous_end_loc();
    Ok(spanned(
        context.tokens.file_hash(),
        start_loc,
        end_loc,
        args,
    ))
}

// Return true if the current token is one that might occur after an Exp.
// This is needed, for example, to check for the optional Exp argument to
// a return (where "return" is itself an Exp).
fn at_end_of_exp(context: &mut Context) -> bool {
    matches!(
        context.tokens.peek(),
        // These are the tokens that can occur after an Exp. If the grammar
        // changes, we need to make sure that these are kept up to date and that
        // none of these tokens can occur at the beginning of an Exp.
        Tok::Else | Tok::RBrace | Tok::RParen | Tok::Comma | Tok::Colon | Tok::Semicolon
    )
}

fn at_start_of_exp(context: &mut Context) -> bool {
    matches!(
        context.tokens.peek(),
        Tok::NumValue
            | Tok::NumTypedValue
            | Tok::ByteStringValue
            | Tok::Identifier
            | Tok::AtSign
            | Tok::Copy
            | Tok::Move
            | Tok::False
            | Tok::True
            | Tok::Amp
            | Tok::AmpMut
            | Tok::Star
            | Tok::Exclaim
            | Tok::LParen
            | Tok::LBrace
            | Tok::Abort
            | Tok::Break
            | Tok::Continue
            | Tok::If
            | Tok::Loop
            | Tok::Return
            | Tok::While
    )
}

// Parse the rest of a lambda expression, after already processing any capture designator (move/copy).
//       LambdaRest =
//                      <LambdaBindList> <Exp> <WithAbilities>
fn parse_lambda(
    context: &mut Context,
    start_loc: usize,
    capture_kind: LambdaCaptureKind,
    token: Tok,
) -> Result<Exp_, Box<Diagnostic>> {
    let bindings = if token == Tok::Pipe {
        parse_lambda_bind_list(context)?
    } else {
        // token is Tok::PipePipe, i.e., empty bind list in this context.
        consume_token(context.tokens, Tok::PipePipe)?;
        let end_loc = context.tokens.previous_end_loc();
        spanned(context.tokens.file_hash(), start_loc, end_loc, vec![])
    };
    let body = Box::new(parse_exp(context)?);
    let abilities_start = context.tokens.start_loc();
    let abilities = parse_with_abilities(context)?;
    if !abilities.is_empty() {
        let abilities_end = context.tokens.previous_end_loc();
        let loc = make_loc(context.tokens.file_hash(), abilities_start, abilities_end);
        require_move_version(
            LanguageVersion::V2_2,
            context,
            loc,
            "Abilities on function expressions",
        );
    }

    Ok(Exp_::Lambda(bindings, body, capture_kind, abilities))
}

// Parse an expression:
//      Exp =
//            ( "move" | "copy" )? <LambdaBindList> <Exp> <WithAbilities>
//          | <Quantifier>                  spec only
//          | <BinOpExp>
//          | <UnaryExp> "=" <Exp>
//          | <UnaryExp> ("+=" | "-=" | "*=" | "/=" | "%=" | "&=" | "|=" | "^=" | "<<=" | ">>=") <Exp>
//          | <UnaryExp> ("as" | "is") Type
fn parse_exp(context: &mut Context) -> Result<Exp, Box<Diagnostic>> {
    let start_loc = context.tokens.start_loc();
    let token = context.tokens.peek();
    let exp = match token {
        Tok::Move | Tok::Copy
            if matches!(
                context.tokens.lookahead_with_start_loc(),
                Ok((Tok::Pipe | Tok::PipePipe, _))
            ) =>
        {
            let _ = require_move_version_and_advance(
                LanguageVersion::V2_2,
                context,
                "Modifier on lambda expression",
            ); // consume the Move/Copy
            let capture_kind = match token {
                Tok::Move => LambdaCaptureKind::Move,
                Tok::Copy => LambdaCaptureKind::Copy,
                _ => {
                    panic!("can't happen");
                },
            };
            parse_lambda(
                context,
                context.tokens.start_loc(),
                capture_kind,
                context.tokens.peek(),
            )?
        },
        Tok::Pipe | Tok::PipePipe => {
            parse_lambda(context, start_loc, LambdaCaptureKind::default(), token)?
        },
        Tok::Identifier if is_quant(context) => parse_quant(context)?,
        _ => {
            // This could be either an assignment, operator assignment (e.g., +=), or a binary operator
            // expression, or a cast or test
            let lhs = parse_unary_exp(context)?;
            let current_token = context.tokens.peek();
            match current_token {
                Tok::Equal => {
                    context.tokens.advance()?; // consume the "="
                    let rhs = Box::new(parse_exp(context)?);
                    Exp_::Assign(Box::new(lhs), None, rhs)
                },
                Tok::PlusEqual
                | Tok::SubEqual
                | Tok::MulEqual
                | Tok::ModEqual
                | Tok::DivEqual
                | Tok::BitOrEqual
                | Tok::BitAndEqual
                | Tok::XorEqual
                | Tok::ShlEqual
                | Tok::ShrEqual => {
                    require_language_version(
                        context,
                        current_token_loc(context.tokens),
                        LanguageVersion::V2_1,
                        "op-equal operators are",
                    );
                    let op_loc = context.tokens.advance_with_loc()?; // consume the "op="
                    let rhs = Box::new(parse_exp(context)?);
                    Exp_::Assign(
                        Box::new(lhs),
                        Some(sp(
                            op_loc,
                            op_equal_to_binop(&current_token).expect("binop"),
                        )),
                        rhs,
                    )
                },
                _ => {
                    if let Some(exp) =
                        parse_cast_or_test_exp(context, &lhs, /*allow_colon_exp*/ false)?
                    {
                        let loc = make_loc(
                            context.tokens.file_hash(),
                            start_loc,
                            context.tokens.previous_end_loc(),
                        );
                        return Ok(sp(loc, exp));
                    } else {
                        return parse_binop_exp(context, lhs, /* min_prec */ 1);
                    }
                },
            }
        },
    };
    let end_loc = context.tokens.previous_end_loc();
    Ok(spanned(context.tokens.file_hash(), start_loc, end_loc, exp))
}

fn op_equal_to_binop(token: &Tok) -> Option<BinOp_> {
    match token {
        Tok::PlusEqual => Some(BinOp_::Add),
        Tok::SubEqual => Some(BinOp_::Sub),
        Tok::MulEqual => Some(BinOp_::Mul),
        Tok::ModEqual => Some(BinOp_::Mod),
        Tok::DivEqual => Some(BinOp_::Div),
        Tok::BitOrEqual => Some(BinOp_::BitOr),
        Tok::BitAndEqual => Some(BinOp_::BitAnd),
        Tok::XorEqual => Some(BinOp_::Xor),
        Tok::ShlEqual => Some(BinOp_::Shl),
        Tok::ShrEqual => Some(BinOp_::Shr),
        _ => None,
    }
}

// Get the precedence of a binary operator. The minimum precedence value
// is 1, and larger values have higher precedence. For tokens that are not
// binary operators, this returns a value of zero so that they will be
// below the minimum value and will mark the end of the binary expression
// for the code in parse_binop_exp.
fn get_precedence(token: Tok) -> u32 {
    match token {
        // Reserved minimum precedence value is 1
        Tok::EqualEqualGreater => 2,
        Tok::LessEqualEqualGreater => 2,
        Tok::PipePipe => 3,
        Tok::AmpAmp => 4,
        Tok::EqualEqual => 5,
        Tok::ExclaimEqual => 5,
        Tok::Less => 5,
        Tok::Greater => 5,
        Tok::LessEqual => 5,
        Tok::GreaterEqual => 5,
        Tok::PeriodPeriod => 6,
        Tok::Pipe => 7,
        Tok::Caret => 8,
        Tok::Amp => 9,
        Tok::LessLess => 10,
        Tok::GreaterGreater => 10,
        Tok::Plus => 11,
        Tok::Minus => 11,
        Tok::Star => 12,
        Tok::Slash => 12,
        Tok::Percent => 12,
        _ => 0, // anything else is not a binary operator
    }
}

// Parse a binary operator expression:
//      BinOpExp =
//          <BinOpExp> <BinOp> <BinOpExp>
//          | <UnaryExp>
//      BinOp = (listed from lowest to highest precedence)
//          "==>"                                       spec only
//          | "||"
//          | "&&"
//          | "==" | "!=" | '<' | ">" | "<=" | ">="
//          | ".."                                      spec only
//          | "|"
//          | "^"
//          | "&"
//          | "<<" | ">>"
//          | "+" | "-"
//          | "*" | "/" | "%"
//
// This function takes the LHS of the expression as an argument, and it
// continues parsing binary expressions as long as they have at least the
// specified "min_prec" minimum precedence.
fn parse_binop_exp(context: &mut Context, lhs: Exp, min_prec: u32) -> Result<Exp, Box<Diagnostic>> {
    let mut result = lhs;
    let mut next_tok_prec = get_precedence(context.tokens.peek());

    while next_tok_prec >= min_prec {
        // Parse the operator.
        let op_start_loc = context.tokens.start_loc();
        let op_token = context.tokens.peek();
        context.tokens.advance()?;
        let op_end_loc = context.tokens.previous_end_loc();

        let mut rhs = parse_unary_exp(context)?;

        // If the next token is another binary operator with a higher
        // precedence, then recursively parse that expression as the RHS.
        let this_prec = next_tok_prec;
        next_tok_prec = get_precedence(context.tokens.peek());
        if this_prec < next_tok_prec {
            rhs = parse_binop_exp(context, rhs, this_prec + 1)?;
            next_tok_prec = get_precedence(context.tokens.peek());
        }

        let op = match op_token {
            Tok::EqualEqual => BinOp_::Eq,
            Tok::ExclaimEqual => BinOp_::Neq,
            Tok::Less => BinOp_::Lt,
            Tok::Greater => BinOp_::Gt,
            Tok::LessEqual => BinOp_::Le,
            Tok::GreaterEqual => BinOp_::Ge,
            Tok::PipePipe => BinOp_::Or,
            Tok::AmpAmp => BinOp_::And,
            Tok::Caret => BinOp_::Xor,
            Tok::Pipe => BinOp_::BitOr,
            Tok::Amp => BinOp_::BitAnd,
            Tok::LessLess => BinOp_::Shl,
            Tok::GreaterGreater => BinOp_::Shr,
            Tok::Plus => BinOp_::Add,
            Tok::Minus => BinOp_::Sub,
            Tok::Star => BinOp_::Mul,
            Tok::Slash => BinOp_::Div,
            Tok::Percent => BinOp_::Mod,
            Tok::PeriodPeriod => BinOp_::Range,
            Tok::EqualEqualGreater => BinOp_::Implies,
            Tok::LessEqualEqualGreater => BinOp_::Iff,
            _ => panic!("Unexpected token that is not a binary operator"),
        };
        let sp_op = spanned(context.tokens.file_hash(), op_start_loc, op_end_loc, op);

        let start_loc = result.loc.start() as usize;
        let end_loc = context.tokens.previous_end_loc();
        let e = Exp_::BinopExp(Box::new(result), sp_op, Box::new(rhs));
        result = spanned(context.tokens.file_hash(), start_loc, end_loc, e);
    }

    Ok(result)
}

// Parse a unary expression:
//      UnaryExp =
//          "!" <UnaryExp>
//          | "&mut" <UnaryExp>
//          | "&" <UnaryExp>
//          | "*" <UnaryExp>
//          | "move" <Var>
//          | "copy" <Var>
//          | <DotOrIndexOrCallChain>
fn parse_unary_exp(context: &mut Context) -> Result<Exp, Box<Diagnostic>> {
    let start_loc = context.tokens.start_loc();
    let exp = match context.tokens.peek() {
        Tok::Exclaim => {
            context.tokens.advance()?;
            let op_end_loc = context.tokens.previous_end_loc();
            let op = spanned(
                context.tokens.file_hash(),
                start_loc,
                op_end_loc,
                UnaryOp_::Not,
            );
            let e = parse_unary_exp(context)?;
            Exp_::UnaryExp(op, Box::new(e))
        },
        Tok::AmpMut => {
            context.tokens.advance()?;
            let e = parse_unary_exp(context)?;
            Exp_::Borrow(true, Box::new(e))
        },
        Tok::Amp => {
            context.tokens.advance()?;
            let e = parse_unary_exp(context)?;
            Exp_::Borrow(false, Box::new(e))
        },
        Tok::Star => {
            context.tokens.advance()?;
            let e = parse_unary_exp(context)?;
            Exp_::Dereference(Box::new(e))
        },
        Tok::Move => {
            context.tokens.advance()?;
            Exp_::Move(parse_var(context)?)
        },
        Tok::Copy => {
            context.tokens.advance()?;
            Exp_::Copy(parse_var(context)?)
        },
        _ => {
            return parse_dot_or_index_chain(context);
        },
    };
    let end_loc = context.tokens.previous_end_loc();
    Ok(spanned(context.tokens.file_hash(), start_loc, end_loc, exp))
}

// Parse an expression term optionally followed by a chain of dot accesses and/or index accesses
// and/or calls of function values.
//      DotOrIndexChain =
//          <DotOrIndexChain> "." <Identifier> [ ["::" "<" Comma<Type> ">"]? <CallArgs> ]?
//          | <DotOrIndexChain> "[" <Exp> "]"
//          | <Term>
fn parse_dot_or_index_chain(context: &mut Context) -> Result<Exp, Box<Diagnostic>> {
    let start_loc = context.tokens.start_loc();
    let mut lhs = parse_term(context)?;
    loop {
        let exp = match context.tokens.peek() {
            Tok::Period => {
                context.tokens.advance()?;
                let n = parse_identifier_or_positional_field(context)?;
                let ahead = context.tokens.peek();
                if matches!(ahead, Tok::LParen | Tok::ColonColon) {
                    let generics = if ahead == Tok::ColonColon {
                        context.tokens.advance()?;
                        Some(parse_comma_list(
                            context,
                            Tok::Less,
                            Tok::Greater,
                            parse_type,
                            "a type",
                        )?)
                    } else {
                        None
                    };
                    let mut args = parse_call_args(context)?;
                    args.loc =
                        Loc::new(context.tokens.file_hash(), lhs.loc.start(), args.loc.end());
                    args.value.insert(0, lhs);
                    let maccess = sp(n.loc, NameAccessChain_::One(n));
                    Exp_::Call(maccess, CallKind::Receiver, generics, args)
                } else {
                    Exp_::Dot(Box::new(lhs), n)
                }
            },
            Tok::LBracket => {
                context.tokens.advance()?;
                let index = parse_exp(context)?;
                let exp = Exp_::Index(Box::new(lhs), Box::new(index));
                consume_token(context.tokens, Tok::RBracket)?;
                exp
            },
            _ => break,
        };
        let end_loc = context.tokens.previous_end_loc();
        lhs = spanned(context.tokens.file_hash(), start_loc, end_loc, exp);
    }
    Ok(lhs)
}

// Lookahead to determine whether this is a quantifier. This matches
//
//      ( "exists" | "forall" | "choose" | "min" )
//          <Identifier> ( ":" | <Identifier> ) ...
//
// as a sequence to identify a quantifier. While the <Identifier> after
// the exists/forall would by syntactically sufficient (Move does not
// have affixed identifiers in expressions), we add another token
// of lookahead to keep the result more precise in the presence of
// syntax errors.
fn is_quant(context: &mut Context) -> bool {
    if !matches!(context.tokens.content(), "exists" | "forall" | "choose") {
        return false;
    }
    match context.tokens.lookahead2() {
        Err(_) => false,
        Ok((tok1, tok2)) => tok1 == Tok::Identifier && matches!(tok2, Tok::Colon | Tok::Identifier),
    }
}

// Parses a quantifier expressions, assuming is_quant(context) is true.
//
//   <Quantifier> =
//       ( "forall" | "exists" ) <QuantifierBindings> ("{" Comma<Exp> "}")* ("where" <Exp>)? ":" <Exp>
//     | ( "choose" [ "min" ] ) <QuantifierBind> "where" <Exp>
//   <QuantifierBindings> = <QuantifierBind> ("," <QuantifierBind>)*
//   <QuantifierBind> = <Identifier> ":" <Type> | <Identifier> "in" <Exp>
//
fn parse_quant(context: &mut Context) -> Result<Exp_, Box<Diagnostic>> {
    let start_loc = context.tokens.start_loc();
    let kind = match context.tokens.content() {
        "exists" => {
            context.tokens.advance()?;
            QuantKind_::Exists
        },
        "forall" => {
            context.tokens.advance()?;
            QuantKind_::Forall
        },
        "choose" => {
            context.tokens.advance()?;
            match context.tokens.peek() {
                Tok::Identifier if context.tokens.content() == "min" => {
                    context.tokens.advance()?;
                    QuantKind_::ChooseMin
                },
                _ => QuantKind_::Choose,
            }
        },
        _ => unreachable!(),
    };
    let spanned_kind = spanned(
        context.tokens.file_hash(),
        start_loc,
        context.tokens.previous_end_loc(),
        kind,
    );

    if matches!(kind, QuantKind_::Choose | QuantKind_::ChooseMin) {
        let binding = parse_quant_binding(context)?;
        consume_identifier(context.tokens, "where")?;
        let body = parse_exp(context)?;
        return Ok(Exp_::Quant(
            spanned_kind,
            Spanned {
                loc: binding.loc,
                value: vec![binding],
            },
            vec![],
            None,
            Box::new(body),
        ));
    }

    let bindings_start_loc = context.tokens.start_loc();
    let binds_with_range_list = parse_list(
        context,
        |context| {
            if context.tokens.peek() == Tok::Comma {
                context.tokens.advance()?;
                Ok(true)
            } else {
                Ok(false)
            }
        },
        parse_quant_binding,
    )?;
    let binds_with_range_list = spanned(
        context.tokens.file_hash(),
        bindings_start_loc,
        context.tokens.previous_end_loc(),
        binds_with_range_list,
    );

    let triggers = if context.tokens.peek() == Tok::LBrace {
        parse_list(
            context,
            |context| {
                if context.tokens.peek() == Tok::LBrace {
                    Ok(true)
                } else {
                    Ok(false)
                }
            },
            |context| {
                parse_comma_list(
                    context,
                    Tok::LBrace,
                    Tok::RBrace,
                    parse_exp,
                    "a trigger expresssion",
                )
            },
        )?
    } else {
        Vec::new()
    };

    let condition = match context.tokens.peek() {
        Tok::Identifier if context.tokens.content() == "where" => {
            context.tokens.advance()?;
            Some(Box::new(parse_exp(context)?))
        },
        _ => None,
    };
    consume_token(context.tokens, Tok::Colon)?;
    let body = parse_exp(context)?;

    Ok(Exp_::Quant(
        spanned_kind,
        binds_with_range_list,
        triggers,
        condition,
        Box::new(body),
    ))
}

// Parses one quantifier binding.
fn parse_quant_binding(context: &mut Context) -> Result<Spanned<(Bind, Exp)>, Box<Diagnostic>> {
    let start_loc = context.tokens.start_loc();
    let ident = parse_identifier(context)?;
    let bind = spanned(
        context.tokens.file_hash(),
        start_loc,
        context.tokens.previous_end_loc(),
        Bind_::Var(Var(ident)),
    );
    let range = if context.tokens.peek() == Tok::Colon {
        // This is a quantifier over the full domain of a type.
        // Built `domain<ty>()` expression.
        context.tokens.advance()?;
        let ty = parse_type(context)?;
        make_builtin_call(ty.loc, Symbol::from("$spec_domain"), Some(vec![ty]), vec![])
    } else {
        // This is a quantifier over a value, like a vector or a range.
        consume_identifier(context.tokens, "in")?;
        parse_exp(context)?
    };
    let end_loc = context.tokens.previous_end_loc();
    Ok(spanned(
        context.tokens.file_hash(),
        start_loc,
        end_loc,
        (bind, range),
    ))
}

fn make_builtin_call(loc: Loc, name: Symbol, type_args: Option<Vec<Type>>, args: Vec<Exp>) -> Exp {
    let maccess = sp(loc, NameAccessChain_::One(sp(loc, name)));
    sp(
        loc,
        Exp_::Call(maccess, CallKind::Regular, type_args, sp(loc, args)),
    )
}

//**************************************************************************************************
// Types
//**************************************************************************************************

// Parse a Type:
//      Type =
//          <NameAccessChain> ("<" Comma<Type> ">")?
//          | "&" <Type>
//          | "&mut" <Type>
//          | "|" Comma<Type> "|" <Type>? <WithAbilities>
//          | "(" Comma<Type> ")"
fn parse_type(context: &mut Context) -> Result<Type, Box<Diagnostic>> {
    let start_loc = context.tokens.start_loc();
    let token = context.tokens.peek();
    let t = match token {
        Tok::LParen => {
            let mut ts = parse_comma_list(context, Tok::LParen, Tok::RParen, parse_type, "a type")?;
            match ts.len() {
                0 => Type_::Unit,
                1 => ts.pop().unwrap().value,
                _ => Type_::Multiple(ts),
            }
        },
        Tok::Amp => {
            context.tokens.advance()?;
            let t = parse_type(context)?;
            Type_::Ref(false, Box::new(t))
        },
        Tok::AmpMut => {
            context.tokens.advance()?;
            let t = parse_type(context)?;
            Type_::Ref(true, Box::new(t))
        },
        Tok::Pipe | Tok::PipePipe => {
            let args = if token == Tok::Pipe {
                parse_comma_list(context, Tok::Pipe, Tok::Pipe, parse_type, "a type")?
            } else {
                // token is Tok::PipePipe, i.e., empty param type list in this context.
                consume_token(context.tokens, Tok::PipePipe)?;
                vec![]
            };
            let result = if is_start_of_type(context) {
                parse_type(context)?
            } else {
                spanned(
                    context.tokens.file_hash(),
                    start_loc,
                    context.tokens.start_loc(),
                    Type_::Unit,
                )
            };
            let abilities_start = context.tokens.start_loc();
            let abilities = parse_with_abilities(context)?;
            if !abilities.is_empty() {
                let abilities_end = context.tokens.previous_end_loc();
                let loc = make_loc(context.tokens.file_hash(), abilities_start, abilities_end);
                require_move_version(
                    LanguageVersion::V2_2,
                    context,
                    loc,
                    "Ability constraints on function types",
                );
            }
            return Ok(spanned(
                context.tokens.file_hash(),
                start_loc,
                context.tokens.previous_end_loc(),
                Type_::Fun(args, Box::new(result), abilities),
            ));
        },
        _ => {
            let tn = parse_name_access_chain(context, false, || "a type name")?;
            let tys = if context.tokens.peek() == Tok::Less {
                parse_comma_list(context, Tok::Less, Tok::Greater, parse_type, "a type")?
            } else {
                vec![]
            };
            Type_::Apply(Box::new(tn), tys)
        },
    };
    let end_loc = context.tokens.previous_end_loc();
    Ok(spanned(context.tokens.file_hash(), start_loc, end_loc, t))
}

/// Checks whether the next tokens looks like the start of a type. NOTE: must be aligned
/// with `parse_type`.
fn is_start_of_type(context: &mut Context) -> bool {
    matches!(
        context.tokens.peek(),
        Tok::LParen | Tok::Amp | Tok::AmpMut | Tok::Pipe | Tok::Identifier
    )
}

// Parse an optional list of type arguments.
//    OptionalTypeArgs = '<' Comma<Type> ">" | <empty>
fn parse_optional_type_args(context: &mut Context) -> Result<Option<Vec<Type>>, Box<Diagnostic>> {
    if context.tokens.peek() == Tok::Less {
        Ok(Some(parse_comma_list(
            context,
            Tok::Less,
            Tok::Greater,
            parse_type,
            "a type",
        )?))
    } else {
        Ok(None)
    }
}

fn token_to_ability(token: Tok, content: &str) -> Option<Ability_> {
    match (token, content) {
        (Tok::Copy, _) => Some(Ability_::Copy),
        (Tok::Identifier, Ability_::DROP) => Some(Ability_::Drop),
        (Tok::Identifier, Ability_::STORE) => Some(Ability_::Store),
        (Tok::Identifier, Ability_::KEY) => Some(Ability_::Key),
        _ => None,
    }
}

// Parse a type ability
//      Ability =
//          <Copy>
//          | "drop"
//          | "store"
//          | "key"
fn parse_ability(context: &mut Context) -> Result<Ability, Box<Diagnostic>> {
    let loc = current_token_loc(context.tokens);
    match token_to_ability(context.tokens.peek(), context.tokens.content()) {
        Some(ability) => {
            context.tokens.advance()?;
            Ok(sp(loc, ability))
        },
        None => {
            let msg = format!(
                "Unexpected {}. Expected a type ability, one of: 'copy', 'drop', 'store', or 'key'",
                current_token_error_string(context.tokens)
            );
            Err(Box::new(diag!(Syntax::UnexpectedToken, (loc, msg))))
        },
    }
}

// Parse an optional "with" type constraint:
//      WithAbilities =
//          ( "with" <Ability> (+ <Ability>)* )?
fn parse_with_abilities(context: &mut Context) -> Result<Vec<Ability>, Box<Diagnostic>> {
    if context.tokens.peek() == Tok::Identifier && context.tokens.content() == "with" {
        context.tokens.advance()?;
        parse_type_constraints_core(context)
    } else {
        Ok(vec![])
    }
}

// Parse an optional type constraint:
//      Constraint =
//          ( ":" <Ability> (+ <Ability>)* )?
fn parse_type_constraints(context: &mut Context) -> Result<Vec<Ability>, Box<Diagnostic>> {
    if match_token(context.tokens, Tok::Colon)? {
        parse_type_constraints_core(context)
    } else {
        Ok(vec![])
    }
}

fn parse_type_constraints_core(context: &mut Context) -> Result<Vec<Ability>, Box<Diagnostic>> {
    parse_list(
        context,
        |context| match context.tokens.peek() {
            Tok::Plus => {
                context.tokens.advance()?;
                Ok(true)
            },
            _ => Ok(false),
        },
        parse_ability,
    )
}

// Parse a type parameter:
//      TypeParameter =
//          <Identifier> <Constraint>?
//      Constraint =
//          ":" <Ability> (+ <Ability>)*
fn parse_type_parameter(context: &mut Context) -> Result<(Name, Vec<Ability>), Box<Diagnostic>> {
    let n = parse_identifier(context)?;

    let ability_constraints = parse_type_constraints(context)?;
    Ok((n, ability_constraints))
}

// Parse type parameter with optional phantom declaration:
//   TypeParameterWithPhantomDecl = "phantom"? <TypeParameter>
fn parse_type_parameter_with_phantom_decl(
    context: &mut Context,
) -> Result<StructTypeParameter, Box<Diagnostic>> {
    let is_phantom =
        if context.tokens.peek() == Tok::Identifier && context.tokens.content() == "phantom" {
            context.tokens.advance()?;
            true
        } else {
            false
        };
    let (name, constraints) = parse_type_parameter(context)?;
    Ok(StructTypeParameter {
        is_phantom,
        name,
        constraints,
    })
}

// Parse optional type parameter list.
//    OptionalTypeParameters = '<' Comma<TypeParameter> ">" | <empty>
fn parse_optional_type_parameters(
    context: &mut Context,
) -> Result<Vec<(Name, Vec<Ability>)>, Box<Diagnostic>> {
    if context.tokens.peek() == Tok::Less {
        parse_comma_list(
            context,
            Tok::Less,
            Tok::Greater,
            parse_type_parameter,
            "a type parameter",
        )
    } else {
        Ok(vec![])
    }
}

// Parse optional struct type parameters:
//    StructTypeParameter = '<' Comma<TypeParameterWithPhantomDecl> ">" | <empty>
fn parse_struct_type_parameters(
    context: &mut Context,
) -> Result<Vec<StructTypeParameter>, Box<Diagnostic>> {
    if context.tokens.peek() == Tok::Less {
        parse_comma_list(
            context,
            Tok::Less,
            Tok::Greater,
            parse_type_parameter_with_phantom_decl,
            "a type parameter",
        )
    } else {
        Ok(vec![])
    }
}

//**************************************************************************************************
// Functions
//**************************************************************************************************

// Parse a function declaration:
//      FunctionDecl =
//          [ "inline" ] "fun"
//          <FunctionDefName> "(" Comma<Parameter> ")"
//          (":" <Type>)?
//          ("acquires" <NameAccessChain> ("," <NameAccessChain>)*)?
//          ("{" <Sequence> "}" | ";")
//      FunctionDefName =
//          <Identifier> <OptionalTypeParameters>
fn parse_function_decl(
    attributes: Vec<Attributes>,
    start_loc: usize,
    modifiers: Modifiers,
    context: &mut Context,
) -> Result<Function, Box<Diagnostic>> {
    let Modifiers {
        visibility,
        mut entry,
        native,
    } = modifiers;

    if let Some(Visibility::Script(vloc)) = visibility {
        let msg = format!(
            "'{script}' is deprecated in favor of the '{entry}' modifier. \
            Replace with '{public} {entry}'",
            script = Visibility::SCRIPT,
            public = Visibility::PUBLIC,
            entry = ENTRY_MODIFIER,
        );
        context
            .env
            .add_diag(diag!(Uncategorized::DeprecatedWillBeRemoved, (vloc, msg,)));
        if entry.is_none() {
            entry = Some(vloc)
        }
    }

    // [ "inline" ] "fun" <FunctionDefName>
    let inline = if context.tokens.peek() == Tok::Inline {
        context.tokens.advance()?;
        true
    } else {
        false
    };
    consume_token(context.tokens, Tok::Fun)?;
    let name = FunctionName(parse_identifier(context)?);
    let type_parameters = parse_optional_type_parameters(context)?;

    // "(" Comma<Parameter> ")"
    let parameters = parse_comma_list(
        context,
        Tok::LParen,
        Tok::RParen,
        parse_parameter,
        "a function parameter",
    )?;

    // (":" <Type>)?
    let return_type = if match_token(context.tokens, Tok::Colon)? {
        parse_type(context)?
    } else {
        sp(name.loc(), Type_::Unit)
    };

    // "pure" | ( ( "!" )? ("acquires" | "reads" | "writes" ) <AccessSpecifierList> )*
    let mut access_specifiers = vec![];
    let mut pure_loc = None;
    loop {
        let negated = if context.tokens.peek() == Tok::Exclaim {
            require_move_2_and_advance(context, "access specifiers")?;
            true
        } else {
            false
        };
        match context.tokens.peek() {
            Tok::Acquires => {
                context.tokens.advance()?;
                access_specifiers.extend(parse_access_specifier_list(
                    context,
                    negated,
                    &AccessSpecifier_::Acquires,
                )?)
            },
            Tok::Identifier if context.tokens.content() == "reads" => {
                require_move_2_and_advance(context, "access specifiers")?;
                access_specifiers.extend(parse_access_specifier_list(
                    context,
                    negated,
                    &AccessSpecifier_::Reads,
                )?)
            },
            Tok::Identifier if context.tokens.content() == "writes" => {
                require_move_2_and_advance(context, "access specifiers")?;
                access_specifiers.extend(parse_access_specifier_list(
                    context,
                    negated,
                    &AccessSpecifier_::Writes,
                )?)
            },
            Tok::Identifier if context.tokens.content() == "pure" => {
                pure_loc = Some(current_token_loc(context.tokens));
                require_move_2_and_advance(context, "access specifiers")?;
                if negated {
                    return Err(Box::new(diag!(
                        Syntax::InvalidAccessSpecifier,
                        (pure_loc.unwrap(), "'pure' cannot be negated")
                    )));
                }
            },
            _ => break,
        }
    }
    let access_specifiers = if let Some(loc) = pure_loc {
        if !access_specifiers.is_empty() {
            return Err(Box::new(diag!(
                Syntax::InvalidAccessSpecifier,
                (
                    loc,
                    "'pure' cannot be mixed with 'acquires'/`reads'/'writes'"
                )
            )));
        }
        // pure is represented by an empty access list
        Some(vec![])
    } else if access_specifiers.is_empty() {
        // no specifiers is represented as None
        None
    } else {
        Some(access_specifiers)
    };

    let body = match native {
        Some(loc) => {
            consume_token(context.tokens, Tok::Semicolon)?;
            sp(loc, FunctionBody_::Native)
        },
        _ => {
            let start_loc = context.tokens.start_loc();
            consume_token(context.tokens, Tok::LBrace)?;
            let seq = parse_sequence(context)?;
            let end_loc = context.tokens.previous_end_loc();
            sp(
                make_loc(context.tokens.file_hash(), start_loc, end_loc),
                FunctionBody_::Defined(seq),
            )
        },
    };

    let signature = FunctionSignature {
        type_parameters,
        parameters,
        return_type,
    };

    let loc = make_loc(
        context.tokens.file_hash(),
        start_loc,
        context.tokens.previous_end_loc(),
    );
    Ok(Function {
        attributes,
        loc,
        visibility: visibility.unwrap_or(Visibility::Internal),
        entry,
        signature,
        access_specifiers,
        inline,
        name,
        body,
    })
}

// Parse a function parameter:
//      Parameter = <Var> ":" <Type>
fn parse_parameter(context: &mut Context) -> Result<(Var, Type), Box<Diagnostic>> {
    let v = parse_var(context)?;
    consume_token(context.tokens, Tok::Colon)?;
    let t = parse_type(context)?;
    Ok((v, t))
}

// Parse an access specifier list:
//      AccessSpecifierList = <AccessSpecifier> ( "," <AccessSpecifier> )* ","?
fn parse_access_specifier_list(
    context: &mut Context,
    negated: bool,
    ctor: &impl Fn(bool, NameAccessChain, Option<Vec<Type>>, AddressSpecifier) -> AccessSpecifier_,
) -> Result<Vec<AccessSpecifier>, Box<Diagnostic>> {
    let mut chain = vec![];
    loop {
        chain.push(parse_access_specifier(context, negated, ctor)?);
        if context.tokens.peek() == Tok::Comma {
            context.tokens.advance()?;
            // Trailing comma allowed, check FIRST(<AccessSpecifier>)
            if matches!(
                context.tokens.peek(),
                Tok::Identifier | Tok::Star | Tok::NumValue
            ) {
                continue;
            } else {
                break;
            }
        } else {
            break;
        }
    }
    Ok(chain)
}

// Parse an access specifier:
//   AccessSpecifier = <NameAccessChainWithWildcard> <OptionalTypeArgs> <AddressSpecifier>
fn parse_access_specifier(
    context: &mut Context,
    negated: bool,
    ctor: &impl Fn(bool, NameAccessChain, Option<Vec<Type>>, AddressSpecifier) -> AccessSpecifier_,
) -> Result<AccessSpecifier, Box<Diagnostic>> {
    let start = context.tokens.start_loc();
    let name_chain = parse_name_access_chain(context, true, || "an access specifier")?;
    let type_args = parse_optional_type_args(context)?;
    let address = parse_address_specifier(context)?;
    let loc = make_loc(
        context.tokens.file_hash(),
        start,
        address.loc.end() as usize,
    );
    Ok(sp(loc, (*ctor)(negated, name_chain, type_args, address)))
}

// Parse an address specifier:
//   AddressSpecifier = <empty> | "(" <AddressSpecifierArg> ")"
//   AddressSpecifierArg = "*" | <AddressBytes> | <NameAccessChain> ( <TypeArgs>? "(" <Identifier> ")" )?
fn parse_address_specifier(context: &mut Context) -> Result<AddressSpecifier, Box<Diagnostic>> {
    let start = context.tokens.start_loc();
    let (spec, end) = if match_token(context.tokens, Tok::LParen)? {
        let spec = match context.tokens.peek() {
            Tok::Star => {
                context.tokens.advance()?;
                AddressSpecifier_::Any
            },
            Tok::NumValue => AddressSpecifier_::Literal(parse_address_bytes(context)?.value),
            _ => {
                let chain = parse_name_access_chain(context, false, || "an address specifier")?;
                let type_args = parse_optional_type_args(context)?;
                if match_token(context.tokens, Tok::LParen)? {
                    let name = parse_identifier(context)?;
                    let call = AddressSpecifier_::Call(chain, type_args, name);
                    consume_token(context.tokens, Tok::RParen)?;
                    call
                } else {
                    if type_args.is_some() {
                        return Err(Box::new(diag!(
                            Syntax::InvalidAccessSpecifier,
                            (chain.loc, "type arguments not allowed")
                        )));
                    }
                    if let NameAccessChain_::One(name) = chain.value {
                        AddressSpecifier_::Name(name)
                    } else {
                        return Err(Box::new(diag!(
                            Syntax::InvalidAccessSpecifier,
                            (chain.loc, "expected a simple name")
                        )));
                    }
                }
            },
        };
        consume_token(context.tokens, Tok::RParen)?;
        (spec, context.tokens.previous_end_loc())
    } else {
        (AddressSpecifier_::Empty, context.tokens.start_loc())
    };
    Ok(sp(make_loc(context.tokens.file_hash(), start, end), spec))
}

//**************************************************************************************************
// Structs
//**************************************************************************************************

// Parse a struct definition:
//      StructDecl =
//          native struct <StructDefName> <Abilities>? ";"
//        | "struct" <StructDefName> <Abilities>? "{" Comma<FieldAnnot> "}" (<Abilities> ";")?
//        | "struct" <StructDefName> ( "(" Comma<Type> ")" )? <Abilities>?";"
//        | "enum" <StructDefName> <Abilities>? "{" Comma<EnumVariant> "}" (<Abilities> ";")?
//      StructDefName =
//          <Identifier> <StructTypeParameter>
//      EnumVariant =
//          <Identifier> "{" Comma<FieldAnnot> "}"
//          <Identifier> "(" Comma<Type> ")"
//          <Identifier>
//      Abilities =
//          "has" <Ability> (, <Ability>)+
fn parse_struct_decl(
    is_enum: bool,
    attributes: Vec<Attributes>,
    start_loc: usize,
    modifiers: Modifiers,
    context: &mut Context,
) -> Result<StructDefinition, Box<Diagnostic>> {
    let Modifiers {
        visibility,
        entry,
        native,
    } = modifiers;
    match visibility {
        Some(vis) if !context.env.flags().lang_v2() => {
            let msg = format!(
                "Invalid struct declaration. Structs cannot have visibility modifiers as they are \
             always '{}'",
                Visibility::PUBLIC
            );
            context
                .env
                .add_diag(diag!(Syntax::InvalidModifier, (vis.loc().unwrap(), msg)));
        },
        _ => {},
    }
    if let Some(loc) = entry {
        let msg = format!(
            "Invalid type declaration. '{}' is used only on functions",
            ENTRY_MODIFIER
        );
        context
            .env
            .add_diag(diag!(Syntax::InvalidModifier, (loc, msg)));
    }

    if is_enum {
        require_move_2_and_advance(context, "enum types")?;
    } else {
        consume_token(context.tokens, Tok::Struct)?;
    }

    // <StructDefName>
    let name = StructName(parse_identifier(context)?);
    let type_parameters = parse_struct_type_parameters(context)?;

    let mut abilities = parse_abilities(context)?;

    let layout = match native {
        Some(loc) => {
            consume_token(context.tokens, Tok::Semicolon)?;
            StructLayout::Native(loc)
        },
        None => {
            if is_enum {
                let mut list = vec![];
                consume_token(context.tokens, Tok::LBrace)?;
                while context.tokens.peek() != Tok::RBrace {
                    // If the variant is based on a block, we allow but do not require
                    // a `,`. Otherwise, a comma is required.
                    let (variant, has_block) = parse_struct_variant(context)?;
                    let next = context.tokens.peek();
                    if (!has_block && next != Tok::RBrace) || next == Tok::Comma {
                        consume_token(context.tokens, Tok::Comma)?;
                    }
                    list.push(variant)
                }
                consume_token(context.tokens, Tok::RBrace)?;
                parse_postfix_abilities(context, &mut abilities)?;
                StructLayout::Variants(list)
            } else {
                let (list, is_positional) = if context.tokens.peek() == Tok::LParen {
                    let loc = current_token_loc(context.tokens);
                    require_move_2(context, loc, "positional fields");
                    let list = parse_anonymous_fields(context)?;
                    abilities = parse_abilities(context)?;
                    consume_token(context.tokens, Tok::Semicolon)?;
                    (list, true)
                } else if context.tokens.peek() == Tok::LBrace {
                    let list = parse_comma_list(
                        context,
                        Tok::LBrace,
                        Tok::RBrace,
                        parse_field_annot,
                        "a field",
                    )?;
                    parse_postfix_abilities(context, &mut abilities)?;
                    (list, false)
                } else {
                    // Assume positional with 0 fields.
                    let loc = current_token_loc(context.tokens);
                    require_move_2(context, loc, "struct declaration without field list");
                    consume_token(context.tokens, Tok::Semicolon)?;
                    (vec![], true)
                };
                StructLayout::Singleton(list, is_positional)
            }
        },
    };

    let loc = make_loc(
        context.tokens.file_hash(),
        start_loc,
        context.tokens.previous_end_loc(),
    );
    Ok(StructDefinition {
        attributes,
        loc,
        abilities,
        name,
        type_parameters,
        layout,
    })
}

/// Parse ability declarations:
///
///    Abilities = "has" <Ability> (, <Ability>)+
fn parse_abilities(context: &mut Context) -> Result<Vec<Ability>, Box<Diagnostic>> {
    if context.tokens.peek() == Tok::Identifier && context.tokens.content() == "has" {
        context.tokens.advance()?;
        parse_list(
            context,
            |context| match context.tokens.peek() {
                Tok::Comma => {
                    context.tokens.advance()?;
                    Ok(true)
                },
                Tok::LBrace | Tok::Semicolon => Ok(false),
                _ => Err(unexpected_token_error(
                    context.tokens,
                    &format!(
                        "one of: '{}', '{}', or '{}'",
                        Tok::Comma,
                        Tok::LBrace,
                        Tok::Semicolon
                    ),
                )),
            },
            parse_ability,
        )
    } else {
        Ok(vec![])
    }
}

/// Parse postfix ability declarations:
///   PostfixAbilities = (<Abilities> ";")?
fn parse_postfix_abilities(
    context: &mut Context,
    prefix_abilities: &mut Vec<Ability>,
) -> Result<(), Box<Diagnostic>> {
    let postfix_abilities = parse_abilities(context)?;
    if !postfix_abilities.is_empty() {
        consume_token(context.tokens, Tok::Semicolon)?;
    }
    if let (Some(sp!(_l1, _)), Some(sp!(l2, _))) =
        (prefix_abilities.first(), postfix_abilities.first())
    {
        let msg =
            "Conflicting ability declarations. Abilities must be declared either before or after \
             the variant list, not both.";
        context
            .env
            .add_diag(diag!(Syntax::InvalidModifier, (*l2, msg)));
    } else if !postfix_abilities.is_empty() {
        *prefix_abilities = postfix_abilities;
    }
    Ok(())
}

// Parse a struct variant, which may have positional fields. The returned boolean indicates whether the variant has a braced (`{..}`)
// field list.
fn parse_struct_variant(context: &mut Context) -> Result<(StructVariant, bool), Box<Diagnostic>> {
    let start_loc = context.tokens.start_loc();
    let attributes = parse_attributes(context)?;
    context.tokens.match_doc_comments();
    let name = VariantName(parse_identifier(context)?);
    let (fields, has_block, is_positional) = if context.tokens.peek() == Tok::LBrace {
        (
            parse_comma_list(
                context,
                Tok::LBrace,
                Tok::RBrace,
                parse_field_annot,
                "a field",
            )?,
            true,
            false,
        )
    } else if context.tokens.peek() == Tok::LParen {
        let loc = current_token_loc(context.tokens);
        require_move_2(context, loc, "positional fields");
        (parse_anonymous_fields(context)?, false, true)
    } else {
        (vec![], false, false)
    };
    let loc = make_loc(
        context.tokens.file_hash(),
        start_loc,
        context.tokens.previous_end_loc(),
    );
    Ok((
        StructVariant {
            attributes,
            loc,
            name,
            fields,
            is_positional,
        },
        has_block,
    ))
}

// Parse a field annotated with a type:
//      FieldAnnot = <DocComments> <Field> ":" <Type>
fn parse_field_annot(context: &mut Context) -> Result<(Field, Type), Box<Diagnostic>> {
    context.tokens.match_doc_comments();
    let f = parse_field(context)?;
    consume_token(context.tokens, Tok::Colon)?;
    let st = parse_type(context)?;
    Ok((f, st))
}

/// Parse a comma list of types surrounded by parenthesis into a vector of `(Field, Type)` pairs
/// where the fields are named "0", "1", ... with location of the type in the second field
/// AnonymousFields = "(" Comma<Type> ")"
fn parse_anonymous_fields(context: &mut Context) -> Result<Vec<(Field, Type)>, Box<Diagnostic>> {
    let field_types = parse_comma_list(
        context,
        Tok::LParen,
        Tok::RParen,
        parse_type,
        "a field type",
    )?;
    Ok(field_types
        .into_iter()
        .enumerate()
        .map(|(field_offset, st)| {
            let field_name_ = Symbol::from(field_offset.to_string());
            let field_name = Spanned::new(st.loc, field_name_);
            let field = Field(field_name);
            (field, st)
        })
        .collect())
}

//**************************************************************************************************
// Constants
//**************************************************************************************************

// Parse a constant:
//      ConstantDecl = "const" <Identifier> ":" <Type> "=" <Exp> ";"
fn parse_constant_decl(
    attributes: Vec<Attributes>,
    start_loc: usize,
    modifiers: Modifiers,
    context: &mut Context,
) -> Result<Constant, Box<Diagnostic>> {
    let Modifiers {
        visibility,
        entry,
        native,
    } = modifiers;
    if let Some(vis) = visibility {
        let msg = "Invalid constant declaration. Constants cannot have visibility modifiers as \
                   they are always internal";
        context
            .env
            .add_diag(diag!(Syntax::InvalidModifier, (vis.loc().unwrap(), msg)));
    }
    if let Some(loc) = entry {
        let msg = format!(
            "Invalid constant declaration. '{}' is used only on functions",
            ENTRY_MODIFIER
        );
        context
            .env
            .add_diag(diag!(Syntax::InvalidModifier, (loc, msg)));
    }
    if let Some(loc) = native {
        let msg = "Invalid constant declaration. 'native' constants are not supported";
        context
            .env
            .add_diag(diag!(Syntax::InvalidModifier, (loc, msg)));
    }
    consume_token(context.tokens, Tok::Const)?;
    let name = ConstantName(parse_identifier(context)?);
    consume_token(context.tokens, Tok::Colon)?;
    let signature = parse_type(context)?;
    consume_token(context.tokens, Tok::Equal)?;
    let value = parse_exp(context)?;
    consume_token(context.tokens, Tok::Semicolon)?;
    let loc = make_loc(
        context.tokens.file_hash(),
        start_loc,
        context.tokens.previous_end_loc(),
    );
    Ok(Constant {
        attributes,
        loc,
        signature,
        name,
        value,
    })
}

//**************************************************************************************************
// AddressBlock
//**************************************************************************************************

// Parse an address block:
//      AddressBlock =
//          "address" <LeadingNameAccess> "{" (<Attributes> <Module>)* "}"
//
// Note that "address" is not a token.
fn parse_address_block(
    attributes: Vec<Attributes>,
    context: &mut Context,
) -> Result<AddressDefinition, Box<Diagnostic>> {
    const UNEXPECTED_TOKEN: &str = "Invalid code unit. Expected 'address', 'module', or 'script'";
    if context.tokens.peek() != Tok::Identifier {
        let start = context.tokens.start_loc();
        let end = start + context.tokens.content().len();
        let loc = make_loc(context.tokens.file_hash(), start, end);
        let msg = format!(
            "{}. Got {}",
            UNEXPECTED_TOKEN,
            current_token_error_string(context.tokens)
        );
        return Err(Box::new(diag!(Syntax::UnexpectedToken, (loc, msg))));
    }
    let addr_name = parse_identifier(context)?;
    if addr_name.value.as_str() != "address" {
        let msg = format!("{}. Got '{}'", UNEXPECTED_TOKEN, addr_name.value);
        return Err(Box::new(diag!(
            Syntax::UnexpectedToken,
            (addr_name.loc, msg)
        )));
    }
    let start_loc = context.tokens.start_loc();
    let addr = parse_leading_name_access(context, false)?;
    let end_loc = context.tokens.previous_end_loc();
    let loc = make_loc(context.tokens.file_hash(), start_loc, end_loc);

    let modules = match context.tokens.peek() {
        Tok::LBrace => {
            context.tokens.advance()?;
            let mut modules = vec![];
            while context.tokens.peek() != Tok::RBrace {
                let attributes = parse_attributes(context)?;
                modules.push(parse_module(attributes, context)?);
            }
            consume_token(context.tokens, Tok::RBrace)?;
            modules
        },
        _ => return Err(unexpected_token_error(context.tokens, "'{'")),
    };

    Ok(AddressDefinition {
        attributes,
        loc,
        addr,
        modules,
    })
}

//**************************************************************************************************
// Friends
//**************************************************************************************************

// Parse a friend declaration:
//      FriendDecl =
//          "friend" <NameAccessChain> ";"
fn parse_friend_decl(
    attributes: Vec<Attributes>,
    context: &mut Context,
) -> Result<FriendDecl, Box<Diagnostic>> {
    let start_loc = context.tokens.start_loc();
    consume_token(context.tokens, Tok::Friend)?;
    let friend = parse_name_access_chain(context, false, || "a friend declaration")?;
    consume_token(context.tokens, Tok::Semicolon)?;
    let loc = make_loc(
        context.tokens.file_hash(),
        start_loc,
        context.tokens.previous_end_loc(),
    );
    Ok(FriendDecl {
        attributes,
        loc,
        friend,
    })
}

//**************************************************************************************************
// Modules
//**************************************************************************************************

// Parse a use declaration:
//      UseDecl =
//          "use" <ModuleIdent> <UseAlias> ";" |
//          "use" <ModuleIdent> :: <UseMember> ";" |
//          "use" <ModuleIdent> :: "{" Comma<UseMember> "}" ";"
fn parse_use_decl(
    attributes: Vec<Attributes>,
    context: &mut Context,
) -> Result<UseDecl, Box<Diagnostic>> {
    consume_token(context.tokens, Tok::Use)?;
    let ident = parse_module_ident(context)?;
    let alias_opt = parse_use_alias(context)?;
    let use_ = match (&alias_opt, context.tokens.peek()) {
        (None, Tok::ColonColon) => {
            consume_token(context.tokens, Tok::ColonColon)?;
            let sub_uses = match context.tokens.peek() {
                Tok::LBrace => parse_comma_list(
                    context,
                    Tok::LBrace,
                    Tok::RBrace,
                    parse_use_member,
                    "a module member alias",
                )?,
                _ => vec![parse_use_member(context)?],
            };
            Use::Members(ident, sub_uses)
        },
        _ => Use::Module(ident, alias_opt.map(ModuleName)),
    };
    consume_token(context.tokens, Tok::Semicolon)?;
    Ok(UseDecl { attributes, use_ })
}

// Parse an alias for a module member:
//      UseMember = <Identifier> <UseAlias>
fn parse_use_member(context: &mut Context) -> Result<(Name, Option<Name>), Box<Diagnostic>> {
    let member = parse_identifier(context)?;
    let alias_opt = parse_use_alias(context)?;
    Ok((member, alias_opt))
}

// Parse an 'as' use alias:
//      UseAlias = ("as" <Identifier>)?
fn parse_use_alias(context: &mut Context) -> Result<Option<Name>, Box<Diagnostic>> {
    Ok(if context.tokens.peek() == Tok::As {
        context.tokens.advance()?;
        Some(parse_identifier(context)?)
    } else {
        None
    })
}

// Parse a module:
//      Module =
//          <DocComments> ( "spec" | "module") (<LeadingNameAccess>::)?<ModuleName> "{"
//              ( <Attributes>
//                  ( <UseDecl> | <FriendDecl> | <SpecBlock> | <Invariant> |
//                    <DocComments> <ModuleMemberModifiers>
//                        (<ConstantDecl> | <StructDecl> | <FunctionDecl>) )
//                  )
//              )*
//          "}"
fn parse_module(
    attributes: Vec<Attributes>,
    context: &mut Context,
) -> Result<ModuleDefinition, Box<Diagnostic>> {
    context.tokens.match_doc_comments();
    let start_loc = context.tokens.start_loc();

    let is_spec_module = if context.tokens.peek() == Tok::Spec {
        context.tokens.advance()?;
        true
    } else {
        consume_token(context.tokens, Tok::Module)?;
        false
    };
    let sp!(n1_loc, n1_) = parse_leading_name_access(context, false)?;
    let (address, name) = match (n1_, context.tokens.peek()) {
        (addr_ @ LeadingNameAccess_::AnonymousAddress(_), _)
        | (addr_ @ LeadingNameAccess_::Name(_), Tok::ColonColon) => {
            let addr = sp(n1_loc, addr_);
            consume_token(context.tokens, Tok::ColonColon)?;
            let name = parse_module_name(context)?;
            (Some(addr), name)
        },
        (LeadingNameAccess_::Name(name), _) => (None, ModuleName(name)),
    };
    consume_token(context.tokens, Tok::LBrace)?;

    let mut members = vec![];
    while context.tokens.peek() != Tok::RBrace {
        members.push({
            let attributes = parse_attributes(context)?;
            match context.tokens.peek() {
                // Top-level specification constructs
                Tok::Invariant => {
                    context.tokens.match_doc_comments();
                    ModuleMember::Spec(singleton_module_spec_block(
                        context,
                        context.tokens.start_loc(),
                        attributes,
                        parse_invariant,
                    )?)
                },
                Tok::Spec => {
                    match context.tokens.lookahead() {
                        Ok(Tok::Fun) | Ok(Tok::Native) => {
                            context.tokens.match_doc_comments();
                            let start_loc = context.tokens.start_loc();
                            context.tokens.advance()?;
                            // Add an extra check for better error message
                            // if old syntax is used
                            if context.tokens.lookahead2() == Ok((Tok::Identifier, Tok::LBrace)) {
                                return Err(unexpected_token_error(
                                    context.tokens,
                                    "only 'spec', drop the 'fun' keyword",
                                ));
                            }
                            ModuleMember::Spec(singleton_module_spec_block(
                                context,
                                start_loc,
                                attributes,
                                parse_spec_function,
                            )?)
                        },
                        _ => {
                            // Regular spec block
                            ModuleMember::Spec(parse_spec_block(attributes, context)?)
                        },
                    }
                },
                // Regular move constructs
                Tok::Use => ModuleMember::Use(parse_use_decl(attributes, context)?),
                Tok::Friend if context.tokens.lookahead()? != Tok::Fun => {
                    // Only interpret as module friend declaration if not directly
                    // followed by fun keyword. This is invalid syntax in v1, so
                    // we can re-interpret it for Move 2.
                    ModuleMember::Friend(parse_friend_decl(attributes, context)?)
                },
                _ => {
                    context.tokens.match_doc_comments();
                    let start_loc = context.tokens.start_loc();
                    let modifiers = parse_module_member_modifiers(context)?;
                    match context.tokens.peek() {
                        Tok::Const => ModuleMember::Constant(parse_constant_decl(
                            attributes, start_loc, modifiers, context,
                        )?),
                        Tok::Fun | Tok::Inline => ModuleMember::Function(parse_function_decl(
                            attributes, start_loc, modifiers, context,
                        )?),
                        Tok::Struct => ModuleMember::Struct(parse_struct_decl(
                            false, attributes, start_loc, modifiers, context,
                        )?),
                        Tok::Identifier if context.tokens.content() == "enum" => {
                            ModuleMember::Struct(parse_struct_decl(
                                true, attributes, start_loc, modifiers, context,
                            )?)
                        },
                        _ => {
                            return Err(unexpected_token_error(
                                context.tokens,
                                &format!(
                                    "a module member: '{}', '{}', '{}', '{}', '{}', '{}', or '{}'",
                                    Tok::Spec,
                                    Tok::Use,
                                    Tok::Friend,
                                    Tok::Const,
                                    Tok::Fun,
                                    Tok::Inline,
                                    Tok::Struct
                                ),
                            ))
                        },
                    }
                },
            }
        })
    }
    consume_token(context.tokens, Tok::RBrace)?;
    let loc = make_loc(
        context.tokens.file_hash(),
        start_loc,
        context.tokens.previous_end_loc(),
    );
    let def = ModuleDefinition {
        attributes,
        loc,
        address,
        name,
        is_spec_module,
        members,
    };

    Ok(def)
}

//**************************************************************************************************
// Scripts
//**************************************************************************************************

// Parse a script:
//      Script =
//          "script" "{"
//              (<Attributes> <UseDecl>)*
//              (<Attributes> <ConstantDecl>)*
//              <Attributes> <DocComments> <ModuleMemberModifiers> <FunctionDecl>
//              (<Attributes> <SpecBlock>)*
//          "}"
fn parse_script(
    script_attributes: Vec<Attributes>,
    context: &mut Context,
) -> Result<Script, Box<Diagnostic>> {
    let start_loc = context.tokens.start_loc();

    consume_token(context.tokens, Tok::Script)?;
    consume_token(context.tokens, Tok::LBrace)?;

    let mut uses = vec![];
    let mut next_item_attributes = parse_attributes(context)?;
    while context.tokens.peek() == Tok::Use {
        uses.push(parse_use_decl(next_item_attributes, context)?);
        next_item_attributes = parse_attributes(context)?;
    }
    let mut constants = vec![];
    while context.tokens.peek() == Tok::Const {
        let start_loc = context.tokens.start_loc();
        constants.push(parse_constant_decl(
            next_item_attributes,
            start_loc,
            Modifiers::empty(),
            context,
        )?);
        next_item_attributes = parse_attributes(context)?;
    }

    context.tokens.match_doc_comments(); // match doc comments to script function
    let function_start_loc = context.tokens.start_loc();
    let modifiers = parse_module_member_modifiers(context)?;
    // don't need to check native modifier, it is checked later
    let function =
        parse_function_decl(next_item_attributes, function_start_loc, modifiers, context)?;

    let mut specs = vec![];
    while context.tokens.peek() == Tok::NumSign || context.tokens.peek() == Tok::Spec {
        let attributes = parse_attributes(context)?;
        specs.push(parse_spec_block(attributes, context)?);
    }

    if context.tokens.peek() != Tok::RBrace {
        let loc = current_token_loc(context.tokens);
        let msg = "Unexpected characters after end of 'script' function";
        return Err(Box::new(diag!(Syntax::UnexpectedToken, (loc, msg))));
    }
    consume_token(context.tokens, Tok::RBrace)?;

    let loc = make_loc(
        context.tokens.file_hash(),
        start_loc,
        context.tokens.previous_end_loc(),
    );
    Ok(Script {
        attributes: script_attributes,
        loc,
        uses,
        constants,
        function,
        specs,
    })
}
//**************************************************************************************************
// Specification Blocks
//**************************************************************************************************

// Parse an optional specification block:
//     SpecBlockTarget =
//          <Identifier> <SpecTargetSignatureOpt>
//        |  "fun" <Identifier>  # deprecated
//        | "struct <Identifier> # deprecated
//        | "module"
//        | "schema" <Identifier> <OptionalTypeParameters>
//        | <empty>
//     SpecBlock =
//        <DocComments> "spec" ( <SpecFunction> | <SpecBlockTarget> "{" SpecBlockMember* "}" )
fn parse_spec_block(
    attributes: Vec<Attributes>,
    context: &mut Context,
) -> Result<SpecBlock, Box<Diagnostic>> {
    context.tokens.match_doc_comments();
    let start_loc = context.tokens.start_loc();
    consume_token(context.tokens, Tok::Spec)?;
    let target_start_loc = context.tokens.start_loc();
    let target_ = match context.tokens.peek() {
        Tok::Fun => {
            return Err(unexpected_token_error(
                context.tokens,
                "only 'spec', drop the 'fun' keyword",
            ));
        },
        Tok::Struct => {
            return Err(unexpected_token_error(
                context.tokens,
                "only 'spec', drop the 'struct' keyword",
            ));
        },
        Tok::Module => {
            context.tokens.advance()?;
            SpecBlockTarget_::Module
        },
        Tok::Identifier if context.tokens.content() == "schema" => {
            context.tokens.advance()?;
            let name = parse_identifier(context)?;
            let type_parameters = parse_optional_type_parameters(context)?;
            SpecBlockTarget_::Schema(name, type_parameters)
        },
        Tok::Identifier => {
            let name = parse_identifier(context)?;
            let signature = parse_spec_target_signature_opt(&name.loc, context)?;
            SpecBlockTarget_::Member(name, signature)
        },
        Tok::LBrace => SpecBlockTarget_::Code,
        _ => {
            return Err(unexpected_token_error(
                context.tokens,
                "one of `module`, `struct`, `fun`, `schema`, or `{`",
            ));
        },
    };
    let target = spanned(
        context.tokens.file_hash(),
        target_start_loc,
        match target_ {
            SpecBlockTarget_::Code => target_start_loc,
            _ => context.tokens.previous_end_loc(),
        },
        target_,
    );

    consume_token(context.tokens, Tok::LBrace)?;
    let mut uses = vec![];
    while context.tokens.peek() == Tok::Use {
        uses.push(parse_use_decl(vec![], context)?);
    }
    let mut members = vec![];
    while context.tokens.peek() != Tok::RBrace {
        members.push(parse_spec_block_member(context)?);
    }
    consume_token(context.tokens, Tok::RBrace)?;
    Ok(spanned(
        context.tokens.file_hash(),
        start_loc,
        context.tokens.previous_end_loc(),
        SpecBlock_ {
            attributes,
            target,
            uses,
            members,
        },
    ))
}

// Parse an optional specification target signature block:
//      SpecTargetSignatureOpt = <TypeParameters>? "(" Comma<Parameter> ")" (":" <Type>)?
//                             | <empty>
fn parse_spec_target_signature_opt(
    loc: &Loc,
    context: &mut Context,
) -> Result<Option<Box<FunctionSignature>>, Box<Diagnostic>> {
    match context.tokens.peek() {
        Tok::Less | Tok::LParen => {
            let type_parameters = parse_optional_type_parameters(context)?;
            // "(" Comma<Parameter> ")"
            let parameters = parse_comma_list(
                context,
                Tok::LParen,
                Tok::RParen,
                parse_parameter,
                "a function parameter",
            )?;
            // (":" <Type>)?
            let return_type = if match_token(context.tokens, Tok::Colon)? {
                parse_type(context)?
            } else {
                sp(*loc, Type_::Unit)
            };
            Ok(Some(Box::new(FunctionSignature {
                type_parameters,
                parameters,
                return_type,
            })))
        },
        _ => Ok(None),
    }
}

// Parse a spec block member:
//    SpecBlockMember = <DocComments> ( <Invariant> | <Condition> | <SpecFunction> | <SpecVariable>
//                                   | <SpecInclude> | <SpecApply> | <SpecPragma> | <SpecLet>
//                                   | <SpecUpdate> | <SpecAxiom> )
fn parse_spec_block_member(context: &mut Context) -> Result<SpecBlockMember, Box<Diagnostic>> {
    context.tokens.match_doc_comments();
    match context.tokens.peek() {
        Tok::Invariant => parse_invariant(context),
        Tok::Let => parse_spec_let(context),
        Tok::Fun | Tok::Native => parse_spec_function(context),
        Tok::Identifier => match context.tokens.content() {
            "assert" | "assume" | "decreases" | "aborts_if" | "aborts_with" | "succeeds_if"
            | "modifies" | "emits" | "ensures" | "requires" => parse_condition(context),
            "axiom" => parse_axiom(context),
            "include" => parse_spec_include(context),
            "apply" => parse_spec_apply(context),
            "pragma" => parse_spec_pragma(context),
            "global" | "local" => parse_spec_variable(context),
            "update" => parse_spec_update(context),
            _ => {
                // local is optional but supported to be able to declare variables which are
                // named like the weak keywords above
                parse_spec_variable(context)
            },
        },
        _ => Err(unexpected_token_error(
            context.tokens,
            "one of `assert`, `assume`, `decreases`, `aborts_if`, `aborts_with`, `succeeds_if`, \
             `modifies`, `emits`, `ensures`, `requires`, `include`, `apply`, `pragma`, `global`, \
             or a name",
        )),
    }
}

// Parse a specification condition:
//    SpecCondition =
//        ("assert" | "assume" | "ensures" | "requires" ) <ConditionProperties> <Exp> ";"
//      | "aborts_if" <ConditionProperties> <Exp> ["with" <Exp>] ";"
//      | ("aborts_with" | "modifies") <ConditionProperties> <Exp> [Comma <Exp>]* ";"
//      | "decreases" <ConditionProperties> <Exp> ";"
//      | "emits" <ConditionProperties> <Exp> "to" <Exp> [If <Exp>] ";"
fn parse_condition(context: &mut Context) -> Result<SpecBlockMember, Box<Diagnostic>> {
    let start_loc = context.tokens.start_loc();
    let kind_ = match context.tokens.content() {
        "assert" => SpecConditionKind_::Assert,
        "assume" => SpecConditionKind_::Assume,
        "decreases" => SpecConditionKind_::Decreases,
        "aborts_if" => SpecConditionKind_::AbortsIf,
        "aborts_with" => SpecConditionKind_::AbortsWith,
        "succeeds_if" => SpecConditionKind_::SucceedsIf,
        "modifies" => SpecConditionKind_::Modifies,
        "emits" => SpecConditionKind_::Emits,
        "ensures" => SpecConditionKind_::Ensures,
        "requires" => SpecConditionKind_::Requires,
        _ => unreachable!(),
    };
    context.tokens.advance()?;
    let kind = spanned(
        context.tokens.file_hash(),
        start_loc,
        context.tokens.previous_end_loc(),
        kind_.clone(),
    );
    let properties = parse_condition_properties(context)?;
    let exp = if kind_ == SpecConditionKind_::AbortsWith || kind_ == SpecConditionKind_::Modifies {
        // Use a dummy expression as a placeholder for this field.
        let loc = make_loc(context.tokens.file_hash(), start_loc, start_loc + 1);
        sp(loc, Exp_::Value(sp(loc, Value_::Bool(false))))
    } else {
        parse_exp(context)?
    };
    let additional_exps = if kind_ == SpecConditionKind_::AbortsIf
        && context.tokens.peek() == Tok::Identifier
        && context.tokens.content() == "with"
    {
        context.tokens.advance()?;
        let codes = vec![parse_exp(context)?];
        consume_token(context.tokens, Tok::Semicolon)?;
        codes
    } else if kind_ == SpecConditionKind_::AbortsWith || kind_ == SpecConditionKind_::Modifies {
        parse_comma_list_after_start(
            context,
            context.tokens.start_loc(),
            context.tokens.peek(),
            Tok::Semicolon,
            parse_exp,
            "an aborts code or modifies target",
        )?
    } else if kind_ == SpecConditionKind_::Emits {
        consume_identifier(context.tokens, "to")?;
        let mut additional_exps = vec![parse_exp(context)?];
        if match_token(context.tokens, Tok::If)? {
            additional_exps.push(parse_exp(context)?);
        }
        consume_token(context.tokens, Tok::Semicolon)?;
        additional_exps
    } else {
        consume_token(context.tokens, Tok::Semicolon)?;
        vec![]
    };
    let end_loc = context.tokens.previous_end_loc();
    Ok(spanned(
        context.tokens.file_hash(),
        start_loc,
        end_loc,
        SpecBlockMember_::Condition {
            kind,
            properties,
            exp,
            additional_exps,
        },
    ))
}

// Parse properties in a condition.
//   ConditionProperties = ( "[" Comma<SpecPragmaProperty> "]" )?
fn parse_condition_properties(
    context: &mut Context,
) -> Result<Vec<PragmaProperty>, Box<Diagnostic>> {
    let properties = if context.tokens.peek() == Tok::LBracket {
        parse_comma_list(
            context,
            Tok::LBracket,
            Tok::RBracket,
            parse_spec_property,
            "a condition property",
        )?
    } else {
        vec![]
    };
    Ok(properties)
}

// Parse an axiom:
//     a = "axiom" <OptionalTypeParameters> <ConditionProperties> <Exp> ";"
fn parse_axiom(context: &mut Context) -> Result<SpecBlockMember, Box<Diagnostic>> {
    let start_loc = context.tokens.start_loc();
    consume_identifier(context.tokens, "axiom")?;
    let type_parameters = parse_optional_type_parameters(context)?;
    let kind = spanned(
        context.tokens.file_hash(),
        start_loc,
        context.tokens.previous_end_loc(),
        SpecConditionKind_::Axiom(type_parameters),
    );
    let properties = parse_condition_properties(context)?;
    let exp = parse_exp(context)?;
    consume_token(context.tokens, Tok::Semicolon)?;
    Ok(spanned(
        context.tokens.file_hash(),
        start_loc,
        context.tokens.previous_end_loc(),
        SpecBlockMember_::Condition {
            kind,
            properties,
            exp,
            additional_exps: vec![],
        },
    ))
}

// Parse an invariant:
//     Invariant = "invariant" <OptionalTypeParameters> [ "update" ] <ConditionProperties> <Exp> ";"
fn parse_invariant(context: &mut Context) -> Result<SpecBlockMember, Box<Diagnostic>> {
    let start_loc = context.tokens.start_loc();
    consume_token(context.tokens, Tok::Invariant)?;
    let type_parameters = parse_optional_type_parameters(context)?;
    let kind_ = match context.tokens.peek() {
        Tok::Identifier if context.tokens.content() == "update" => {
            context.tokens.advance()?;
            SpecConditionKind_::InvariantUpdate(type_parameters)
        },
        _ => SpecConditionKind_::Invariant(type_parameters),
    };
    let kind = spanned(
        context.tokens.file_hash(),
        start_loc,
        context.tokens.previous_end_loc(),
        kind_,
    );
    let properties = parse_condition_properties(context)?;
    let exp = parse_exp(context)?;
    consume_token(context.tokens, Tok::Semicolon)?;
    Ok(spanned(
        context.tokens.file_hash(),
        start_loc,
        context.tokens.previous_end_loc(),
        SpecBlockMember_::Condition {
            kind,
            properties,
            exp,
            additional_exps: vec![],
        },
    ))
}

// Parse a specification function.
//     SpecFunction = "fun" <SpecFunctionSignature> ( "{" <Sequence> "}" | ";" )
//                  | "native" "fun" <SpecFunctionSignature> ";"
//     SpecFunctionSignature =
//         <Identifier> <OptionalTypeParameters> "(" Comma<Parameter> ")" ":" <Type>
fn parse_spec_function(context: &mut Context) -> Result<SpecBlockMember, Box<Diagnostic>> {
    let start_loc = context.tokens.start_loc();
    let native_opt = consume_optional_token_with_loc(context.tokens, Tok::Native)?;
    consume_token(context.tokens, Tok::Fun)?;
    let name = FunctionName(parse_identifier(context)?);
    let type_parameters = parse_optional_type_parameters(context)?;
    // "(" Comma<Parameter> ")"
    let parameters = parse_comma_list(
        context,
        Tok::LParen,
        Tok::RParen,
        parse_parameter,
        "a function parameter",
    )?;

    // ":" <Type>)
    consume_token(context.tokens, Tok::Colon)?;
    let return_type = parse_type(context)?;

    let body_start_loc = context.tokens.start_loc();
    let no_body = context.tokens.peek() != Tok::LBrace;
    let (uninterpreted, body_) = if native_opt.is_some() || no_body {
        consume_token(context.tokens, Tok::Semicolon)?;
        (native_opt.is_none(), FunctionBody_::Native)
    } else {
        consume_token(context.tokens, Tok::LBrace)?;
        let seq = parse_sequence(context)?;
        (false, FunctionBody_::Defined(seq))
    };
    let body = spanned(
        context.tokens.file_hash(),
        body_start_loc,
        context.tokens.previous_end_loc(),
        body_,
    );

    let signature = FunctionSignature {
        type_parameters,
        parameters,
        return_type,
    };

    Ok(spanned(
        context.tokens.file_hash(),
        start_loc,
        context.tokens.previous_end_loc(),
        SpecBlockMember_::Function {
            signature,
            uninterpreted,
            name,
            body,
        },
    ))
}

// Parse a specification variable.
//     SpecVariable = ( "global" | "local" )?
//                    <Identifier> <OptionalTypeParameters>
//                    ":" <Type>
//                    [ "=" Exp ]  // global only
//                    ";"
fn parse_spec_variable(context: &mut Context) -> Result<SpecBlockMember, Box<Diagnostic>> {
    let start_loc = context.tokens.start_loc();
    let is_global = match context.tokens.content() {
        "global" => {
            consume_token(context.tokens, Tok::Identifier)?;
            true
        },
        "local" => {
            consume_token(context.tokens, Tok::Identifier)?;
            false
        },
        _ => false,
    };
    let name = parse_identifier(context)?;
    let type_parameters = parse_optional_type_parameters(context)?;
    consume_token(context.tokens, Tok::Colon)?;
    let type_ = parse_type(context)?;
    let init = if is_global && context.tokens.peek() == Tok::Equal {
        context.tokens.advance()?;
        Some(parse_exp(context)?)
    } else {
        None
    };

    consume_token(context.tokens, Tok::Semicolon)?;
    Ok(spanned(
        context.tokens.file_hash(),
        start_loc,
        context.tokens.previous_end_loc(),
        SpecBlockMember_::Variable {
            is_global,
            name,
            type_parameters,
            type_,
            init,
        },
    ))
}

// Parse a specification update.
//     SpecUpdate = "update" <UnaryExp> "=" <Exp> ";"
fn parse_spec_update(context: &mut Context) -> Result<SpecBlockMember, Box<Diagnostic>> {
    let start_loc = context.tokens.start_loc();
    consume_token(context.tokens, Tok::Identifier)?;
    let lhs = parse_unary_exp(context)?;
    consume_token(context.tokens, Tok::Equal)?;
    let rhs = parse_exp(context)?;
    consume_token(context.tokens, Tok::Semicolon)?;
    Ok(spanned(
        context.tokens.file_hash(),
        start_loc,
        context.tokens.previous_end_loc(),
        SpecBlockMember_::Update { lhs, rhs },
    ))
}

// Parse a specification let.
//     SpecLet =  "let" [ "post" ] <Identifier> "=" <Exp> ";"
fn parse_spec_let(context: &mut Context) -> Result<SpecBlockMember, Box<Diagnostic>> {
    let start_loc = context.tokens.start_loc();
    context.tokens.advance()?;
    let post_state =
        if context.tokens.peek() == Tok::Identifier && context.tokens.content() == "post" {
            context.tokens.advance()?;
            true
        } else {
            false
        };
    let name = parse_identifier(context)?;
    consume_token(context.tokens, Tok::Equal)?;
    let def = parse_exp(context)?;
    consume_token(context.tokens, Tok::Semicolon)?;
    Ok(spanned(
        context.tokens.file_hash(),
        start_loc,
        context.tokens.previous_end_loc(),
        SpecBlockMember_::Let {
            name,
            post_state,
            def,
        },
    ))
}

// Parse a specification schema include.
//    SpecInclude = "include" <ConditionProperties> <Exp> ";"
fn parse_spec_include(context: &mut Context) -> Result<SpecBlockMember, Box<Diagnostic>> {
    let start_loc = context.tokens.start_loc();
    consume_identifier(context.tokens, "include")?;
    let properties = parse_condition_properties(context)?;
    let exp = parse_exp(context)?;
    consume_token(context.tokens, Tok::Semicolon)?;
    Ok(spanned(
        context.tokens.file_hash(),
        start_loc,
        context.tokens.previous_end_loc(),
        SpecBlockMember_::Include { properties, exp },
    ))
}

// Parse a specification schema apply.
//    SpecApply = "apply" <Exp> "to" Comma<SpecApplyPattern>
//                                   ( "except" Comma<SpecApplyPattern> )? ";"
fn parse_spec_apply(context: &mut Context) -> Result<SpecBlockMember, Box<Diagnostic>> {
    let start_loc = context.tokens.start_loc();
    consume_identifier(context.tokens, "apply")?;
    let exp = parse_exp(context)?;
    consume_identifier(context.tokens, "to")?;
    let parse_patterns = |context: &mut Context| {
        parse_list(
            context,
            |context| {
                if context.tokens.peek() == Tok::Comma {
                    context.tokens.advance()?;
                    Ok(true)
                } else {
                    Ok(false)
                }
            },
            parse_spec_apply_pattern,
        )
    };
    let patterns = parse_patterns(context)?;
    let exclusion_patterns =
        if context.tokens.peek() == Tok::Identifier && context.tokens.content() == "except" {
            context.tokens.advance()?;
            parse_patterns(context)?
        } else {
            vec![]
        };
    consume_token(context.tokens, Tok::Semicolon)?;
    Ok(spanned(
        context.tokens.file_hash(),
        start_loc,
        context.tokens.previous_end_loc(),
        SpecBlockMember_::Apply {
            exp,
            patterns,
            exclusion_patterns,
        },
    ))
}

// Parse a function pattern:
//     SpecApplyPattern = ( "public" | "internal" )? <SpecApplyFragment>+ <OptionalTypeArgs>
fn parse_spec_apply_pattern(context: &mut Context) -> Result<SpecApplyPattern, Box<Diagnostic>> {
    let start_loc = context.tokens.start_loc();
    // TODO: update the visibility parsing in the spec as well
    let public_opt = consume_optional_token_with_loc(context.tokens, Tok::Public)?;
    let visibility = if let Some(loc) = public_opt {
        Some(Visibility::Public(loc))
    } else if context.tokens.peek() == Tok::Identifier && context.tokens.content() == "internal" {
        // Its not ideal right now that we do not have a loc here, but acceptable for what
        // we are doing with this in specs.
        context.tokens.advance()?;
        Some(Visibility::Internal)
    } else {
        None
    };
    let mut last_end = context.tokens.start_loc() + context.tokens.content().len();
    let name_pattern = parse_list(
        context,
        |context| {
            // We need name fragments followed by each other without space. So we do some
            // magic here similar as with `>>` based on token distance.
            let start_loc = context.tokens.start_loc();
            let adjacent = last_end == start_loc;
            last_end = start_loc + context.tokens.content().len();
            Ok(adjacent && [Tok::Identifier, Tok::Star].contains(&context.tokens.peek()))
        },
        parse_spec_apply_fragment,
    )?;
    let type_parameters = parse_optional_type_parameters(context)?;
    Ok(spanned(
        context.tokens.file_hash(),
        start_loc,
        context.tokens.previous_end_loc(),
        SpecApplyPattern_ {
            visibility,
            name_pattern,
            type_parameters,
        },
    ))
}

// Parse a name pattern fragment
//     SpecApplyFragment = <Identifier> | "*"
fn parse_spec_apply_fragment(context: &mut Context) -> Result<SpecApplyFragment, Box<Diagnostic>> {
    let start_loc = context.tokens.start_loc();
    let fragment = match context.tokens.peek() {
        Tok::Identifier => SpecApplyFragment_::NamePart(parse_identifier(context)?),
        Tok::Star => {
            context.tokens.advance()?;
            SpecApplyFragment_::Wildcard
        },
        _ => {
            return Err(unexpected_token_error(
                context.tokens,
                "a name fragment or `*`",
            ))
        },
    };
    Ok(spanned(
        context.tokens.file_hash(),
        start_loc,
        context.tokens.previous_end_loc(),
        fragment,
    ))
}

// Parse a specification pragma:
//    SpecPragma = "pragma" Comma<SpecPragmaProperty> ";"
fn parse_spec_pragma(context: &mut Context) -> Result<SpecBlockMember, Box<Diagnostic>> {
    let start_loc = context.tokens.start_loc();
    consume_identifier(context.tokens, "pragma")?;
    let properties = parse_comma_list_after_start(
        context,
        start_loc,
        Tok::Identifier,
        Tok::Semicolon,
        parse_spec_property,
        "a pragma property",
    )?;
    Ok(spanned(
        context.tokens.file_hash(),
        start_loc,
        context.tokens.previous_end_loc(),
        SpecBlockMember_::Pragma { properties },
    ))
}

// Parse a specification pragma property:
//    SpecPragmaProperty = <Identifier> ( "=" ( <Value> | <NameAccessChain> ) )?
fn parse_spec_property(context: &mut Context) -> Result<PragmaProperty, Box<Diagnostic>> {
    let start_loc = context.tokens.start_loc();
    let name = match consume_optional_token_with_loc(context.tokens, Tok::Friend)? {
        // special treatment for `pragma friend = ...` as friend is a keyword
        // TODO: this might violate the assumption that a keyword can never be a name.
        Some(loc) => Name::new(loc, Symbol::from("friend")),
        None => parse_identifier(context)?,
    };
    let value = if context.tokens.peek() == Tok::Equal {
        context.tokens.advance()?;
        match context.tokens.peek() {
            Tok::AtSign | Tok::True | Tok::False | Tok::NumTypedValue | Tok::ByteStringValue => {
                Some(PragmaValue::Literal(parse_value(context)?))
            },
            Tok::NumValue
                if !context
                    .tokens
                    .lookahead()
                    .map(|tok| tok == Tok::ColonColon)
                    .unwrap_or(false) =>
            {
                Some(PragmaValue::Literal(parse_value(context)?))
            },
            _ => {
                // Parse as a module access for a possibly qualified identifier
                Some(PragmaValue::Ident(parse_name_access_chain(
                    context,
                    false,
                    || "an identifier as pragma value",
                )?))
            },
        }
    } else {
        None
    };
    Ok(spanned(
        context.tokens.file_hash(),
        start_loc,
        context.tokens.previous_end_loc(),
        PragmaProperty_ { name, value },
    ))
}

/// Creates a module spec block for a single member.
fn singleton_module_spec_block(
    context: &mut Context,
    start_loc: usize,
    attributes: Vec<Attributes>,
    member_parser: impl Fn(&mut Context) -> Result<SpecBlockMember, Box<Diagnostic>>,
) -> Result<SpecBlock, Box<Diagnostic>> {
    let member = member_parser(context)?;
    let end_loc = context.tokens.previous_end_loc();
    Ok(spanned(
        context.tokens.file_hash(),
        start_loc,
        end_loc,
        SpecBlock_ {
            attributes,
            target: spanned(
                context.tokens.file_hash(),
                start_loc,
                start_loc,
                SpecBlockTarget_::Module,
            ),
            uses: vec![],
            members: vec![member],
        },
    ))
}

//**************************************************************************************************
// File
//**************************************************************************************************

// Parse a file:
//      File =
//          (<Attributes> (<AddressBlock> | <Module> | <Script>))*
fn parse_file(context: &mut Context) -> Result<Vec<Definition>, Box<Diagnostic>> {
    let mut defs = vec![];
    while context.tokens.peek() != Tok::EOF {
        let attributes = parse_attributes(context)?;
        defs.push(match context.tokens.peek() {
            Tok::Spec | Tok::Module => Definition::Module(parse_module(attributes, context)?),
            Tok::Script => Definition::Script(parse_script(attributes, context)?),
            _ => Definition::Address(parse_address_block(attributes, context)?),
        })
    }
    Ok(defs)
}

/// Parse the `input` string as a file of Move source code and return the
/// result as either a pair of FileDefinition and doc comments or some Diagnostics. The `file` name
/// is used to identify source locations in error messages.
pub fn parse_file_string(
    env: &mut CompilationEnv,
    file_hash: FileHash,
    input: &str,
) -> Result<(Vec<Definition>, MatchedFileCommentMap), Diagnostics> {
    let mut tokens = Lexer::new(input, file_hash);
    match tokens.advance() {
        Err(err) => Err(Diagnostics::from(vec![*err])),
        Ok(..) => Ok(()),
    }?;
    match parse_file(&mut Context::new(env, &mut tokens)) {
        Err(err) => Err(Diagnostics::from(vec![*err])),
        Ok(def) => Ok((def, tokens.check_and_get_doc_comments(env))),
    }
}
