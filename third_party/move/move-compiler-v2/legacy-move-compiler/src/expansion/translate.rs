// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use super::aliases::{AliasMapBuilder, OldAliasMap};
use crate::{
    command_line::SKIP_ATTRIBUTE_CHECKS,
    diag,
    diagnostics::{codes::DeprecatedItem, Diagnostic},
    expansion::{
        aliases::{AliasMap, AliasSet},
        ast::{
            self as E, AccessSpecifierKind, Address, Fields, LValueOrDotDot_, LValue_,
            ModuleAccess_, ModuleIdent, ModuleIdent_, SequenceItem_, SpecId,
        },
        byte_string, hex_string,
    },
    parser::ast::{
        self as P, Ability, AccessSpecifier_, AddressSpecifier_, CallKind, ConstantName, Field,
        FunctionName, LeadingNameAccess_, ModuleMember, ModuleName, NameAccessChain,
        NameAccessChain_, StructName, Var,
    },
    shared::{
        builtins,
        known_attributes::{AttributeKind, AttributePosition, KnownAttribute},
        parse_u128, parse_u64, parse_u8,
        unique_map::UniqueMap,
        CompilationEnv, Identifier, Name, NamedAddressMap, NamedAddressMaps, NumericalAddress,
    },
    FullyCompiledProgram,
};
use move_command_line_common::parser::{parse_u16, parse_u256, parse_u32};
use move_ir_types::location::*;
use move_symbol_pool::Symbol;
use once_cell::sync::Lazy;
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    iter::IntoIterator,
};
use str;

//**************************************************************************************************
// Context
//**************************************************************************************************

type ModuleMembers = BTreeMap<Name, ModuleMemberInfo>;
struct Context<'env, 'map> {
    module_members: UniqueMap<ModuleIdent, ModuleMembers>,
    module_deprecation_attribute_locs: BTreeMap<ModuleIdent, Loc>, // if any
    named_address_mapping: Option<&'map NamedAddressMap>,
    address: Option<Address>,
    current_module: Option<ModuleIdent>,
    aliases: AliasMap,
    is_source_definition: bool,
    in_spec_context: bool,
    in_deprecated_code: bool,
    in_aptos_libs: bool,
    exp_specs: BTreeMap<SpecId, E::SpecBlock>,
    env: &'env mut CompilationEnv,
}
impl<'env, 'map> Context<'env, 'map> {
    fn new(
        compilation_env: &'env mut CompilationEnv,
        module_members: UniqueMap<ModuleIdent, ModuleMembers>,
        module_deprecation_attribute_locs: BTreeMap<ModuleIdent, Loc>,
    ) -> Self {
        Self {
            module_members,
            module_deprecation_attribute_locs,
            env: compilation_env,
            named_address_mapping: None,
            address: None,
            current_module: None,
            aliases: AliasMap::new(),
            is_source_definition: false,
            in_spec_context: false,
            in_deprecated_code: false,
            in_aptos_libs: false,
            exp_specs: BTreeMap::new(),
        }
    }

    fn cur_address(&self) -> &Address {
        self.address.as_ref().unwrap()
    }

    fn set_current_module(&mut self, module: Option<ModuleIdent>) {
        self.in_deprecated_code = match &module {
            Some(m) => self.module_deprecation_attribute_locs.contains_key(m),
            None => false,
        };
        self.current_module = module;
    }

    fn current_module(&self) -> Option<&ModuleIdent> {
        self.current_module.as_ref()
    }

    /// Returns previous state: whether we were already in deprecated code
    fn enter_possibly_deprecated_member(&mut self, name: &Name) -> bool {
        let was_in_deprecated_code = self.in_deprecated_code;
        if let Some(moduleid) = self.current_module() {
            if let Some(member_info_map) = self.module_members.get(moduleid) {
                if let Some(member_info) = member_info_map.get(name) {
                    if member_info.deprecation.is_some() {
                        self.in_deprecated_code = true;
                    }
                }
            }
        };
        was_in_deprecated_code
    }

    fn set_in_deprecated_code(&mut self, was_deprecated: bool) {
        self.in_deprecated_code = was_deprecated;
    }

    /// Resets the alias map and reports errors for aliases that were unused
    pub fn set_to_outer_scope(&mut self, outer_scope: OldAliasMap) {
        let AliasSet { modules, members } = self.aliases.set_to_outer_scope(outer_scope);
        for alias in modules {
            unused_alias(self, alias)
        }
        for alias in members {
            unused_alias(self, alias)
        }
    }

    pub fn bind_exp_spec(&mut self, spec_block: P::SpecBlock) -> (SpecId, UnboundNames) {
        let espec_block = spec(self, spec_block);
        let mut unbound_names = UnboundNames::default();
        unbound_names_spec_block(&mut unbound_names, &espec_block);

        let id = SpecId::new(self.exp_specs.len());
        self.exp_specs.insert(id, espec_block);

        (id, unbound_names)
    }

    pub fn extract_exp_specs(&mut self) -> BTreeMap<SpecId, E::SpecBlock> {
        std::mem::take(&mut self.exp_specs)
    }
}

//**************************************************************************************************
// Entry
//**************************************************************************************************

pub fn program(
    compilation_env: &mut CompilationEnv,
    pre_compiled_lib: Option<&FullyCompiledProgram>,
    prog: P::Program,
) -> E::Program {
    let mut module_deprecation_attribute_locs = BTreeMap::new();

    // Process all members from program source, lib, and pre-compiled libs,
    // recording just module->SpannedSymbol->ModuleMemberInfo for each,
    // plus per-module deprecation info in module_deprecation_attribute_locs.
    let module_members = {
        let mut members = UniqueMap::new();
        all_module_members(
            compilation_env,
            &prog.named_address_maps,
            &mut members,
            &mut module_deprecation_attribute_locs,
            true,
            &prog.source_definitions,
        );
        all_module_members(
            compilation_env,
            &prog.named_address_maps,
            &mut members,
            &mut module_deprecation_attribute_locs,
            true,
            &prog.lib_definitions,
        );
        if let Some(pre_compiled) = pre_compiled_lib {
            assert!(pre_compiled.parser.lib_definitions.is_empty());
            all_module_members(
                compilation_env,
                &pre_compiled.parser.named_address_maps,
                &mut members,
                &mut module_deprecation_attribute_locs,
                false,
                &pre_compiled.parser.source_definitions,
            );
        }
        members
    };

    let mut context = Context::new(
        compilation_env,
        module_members,
        module_deprecation_attribute_locs,
    );

    let mut source_module_map = UniqueMap::new();
    let mut lib_module_map = UniqueMap::new();
    let mut scripts = vec![];
    let P::Program {
        named_address_maps,
        source_definitions,
        lib_definitions,
    } = prog;

    context.is_source_definition = true;
    for P::PackageDefinition {
        package,
        named_address_map,
        def,
    } in source_definitions
    {
        context.named_address_mapping = Some(named_address_maps.get(named_address_map));
        definition(
            &mut context,
            &mut source_module_map,
            &mut scripts,
            package,
            def,
        )
    }

    context.is_source_definition = false;
    for P::PackageDefinition {
        package,
        named_address_map,
        def,
    } in lib_definitions
    {
        context.named_address_mapping = Some(named_address_maps.get(named_address_map));
        definition(
            &mut context,
            &mut lib_module_map,
            &mut scripts,
            package,
            def,
        )
    }

    for (mident, module) in lib_module_map {
        if let Err((mident, old_loc)) = source_module_map.add(mident, module) {
            if !context.env.flags().sources_shadow_deps() {
                duplicate_module(&mut context, &source_module_map, mident, old_loc)
            }
        }
    }
    let mut module_map = source_module_map;

    let mut scripts = {
        let mut collected: BTreeMap<Symbol, Vec<E::Script>> = BTreeMap::new();
        for s in scripts {
            collected
                .entry(s.function_name.value())
                .or_default()
                .push(s)
        }
        let mut keyed: BTreeMap<Symbol, E::Script> = BTreeMap::new();
        for (n, mut ss) in collected {
            match ss.len() {
                0 => unreachable!(),
                1 => assert!(
                    keyed.insert(n, ss.pop().unwrap()).is_none(),
                    "ICE duplicate script key"
                ),
                _ => {
                    for (i, s) in ss.into_iter().enumerate() {
                        let k = format!("{}_{}", n, i);
                        assert!(
                            keyed.insert(k.into(), s).is_none(),
                            "ICE duplicate script key"
                        )
                    }
                },
            }
        }
        keyed
    };

    super::dependency_ordering::verify(context.env, &mut module_map, &mut scripts);
    E::Program {
        modules: module_map,
        scripts,
    }
}

fn definition(
    context: &mut Context,
    module_map: &mut UniqueMap<ModuleIdent, E::ModuleDefinition>,
    scripts: &mut Vec<E::Script>,
    package_name: Option<Symbol>,
    def: P::Definition,
) {
    match def {
        P::Definition::Module(mut m) => {
            let module_paddr = std::mem::take(&mut m.address);
            let module_addr = module_paddr
                .map(|a| sp(a.loc, address(context, /* suggest_declaration */ true, a)));
            module(context, module_map, package_name, module_addr, m)
        },
        P::Definition::Address(a) => {
            let addr = address(context, /* suggest_declaration */ false, a.addr);
            for mut m in a.modules {
                let module_addr = check_module_address(context, a.loc, addr, &mut m);
                module(context, module_map, package_name, Some(module_addr), m)
            }
        },

        P::Definition::Script(_) if !context.is_source_definition => (),
        P::Definition::Script(s) => script(context, scripts, package_name, s),
    }
}

fn address_without_value_error(suggest_declaration: bool, loc: Loc, n: &Name) -> Diagnostic {
    let mut msg = format!("address '{}' is not assigned a value", n);
    if suggest_declaration {
        msg = format!(
            "{}. Try assigning it a value when calling the compiler",
            msg,
        )
    }
    diag!(NameResolution::AddressWithoutValue, (loc, msg))
}

// Access a top level address as declared, not affected by any aliasing/shadowing
fn address(context: &mut Context, suggest_declaration: bool, ln: P::LeadingNameAccess) -> Address {
    address_(
        context.env,
        context.named_address_mapping.as_ref().unwrap(),
        suggest_declaration,
        ln,
    )
}

fn address_(
    compilation_env: &mut CompilationEnv,
    named_address_mapping: &NamedAddressMap,
    suggest_declaration: bool,
    ln: P::LeadingNameAccess,
) -> Address {
    let name_res = check_valid_address_name_(compilation_env, &ln);
    let sp!(loc, ln_) = ln;
    match ln_ {
        P::LeadingNameAccess_::AnonymousAddress(bytes) => {
            debug_assert!(name_res.is_ok()); //
            Address::Numerical(None, sp(loc, bytes))
        },
        P::LeadingNameAccess_::Name(n) => match named_address_mapping.get(&n.value).copied() {
            Some(addr) => Address::Numerical(Some(n), sp(loc, addr)),
            None => {
                if name_res.is_ok() {
                    compilation_env.add_diag(address_without_value_error(
                        suggest_declaration,
                        loc,
                        &n,
                    ));
                }
                Address::NamedUnassigned(n)
            },
        },
    }
}

fn module_ident(context: &mut Context, sp!(loc, mident_): P::ModuleIdent) -> ModuleIdent {
    let P::ModuleIdent_ {
        address: ln,
        module,
    } = mident_;
    let addr = address(context, /* suggest_declaration */ false, ln);
    sp(loc, ModuleIdent_::new(addr, module))
}

fn check_module_address(
    context: &mut Context,
    loc: Loc,
    addr: Address,
    m: &mut P::ModuleDefinition,
) -> Spanned<Address> {
    let module_address = std::mem::take(&mut m.address);
    match module_address {
        Some(other_paddr) => {
            let other_loc = other_paddr.loc;
            let other_addr = address(context, /* suggest_declaration */ true, other_paddr);
            let msg = if addr == other_addr {
                "Redundant address specification"
            } else {
                "Multiple addresses specified for module"
            };
            context.env.add_diag(diag!(
                Declarations::DuplicateItem,
                (other_loc, msg),
                (loc, "Address previously specified here")
            ));
            sp(other_loc, other_addr)
        },
        None => sp(loc, addr),
    }
}

fn duplicate_module(
    context: &mut Context,
    module_map: &UniqueMap<ModuleIdent, E::ModuleDefinition>,
    mident: ModuleIdent,
    old_loc: Loc,
) {
    let old_mident = module_map.get_key(&mident).unwrap();
    let dup_msg = format!("Duplicate definition for module '{}'", mident);
    let prev_msg = format!("Module previously defined here, with '{}'", old_mident);
    context.env.add_diag(diag!(
        Declarations::DuplicateItem,
        (mident.loc, dup_msg),
        (old_loc, prev_msg),
    ))
}

fn module(
    context: &mut Context,
    module_map: &mut UniqueMap<ModuleIdent, E::ModuleDefinition>,
    package_name: Option<Symbol>,
    module_address: Option<Spanned<Address>>,
    module_def: P::ModuleDefinition,
) {
    assert!(context.address.is_none());
    let (mident, mod_) = module_(context, package_name, module_address, module_def);
    if let Err((mident, old_loc)) = module_map.add(mident, mod_) {
        duplicate_module(context, module_map, mident, old_loc)
    }
    context.address = None;
    context.current_module = None;
    context.in_deprecated_code = false;
}

fn set_sender_address(
    context: &mut Context,
    module_name: &ModuleName,
    sender: Option<Spanned<Address>>,
) {
    context.address = Some(match sender {
        Some(sp!(_, addr)) => addr,
        None => {
            let loc = module_name.loc();
            let msg = format!(
                "Invalid module declaration. The module does not have a specified address. Either \
                 declare it inside of an 'address <address> {{' block or declare it with an \
                 address 'module <address>::{}''",
                module_name
            );
            context
                .env
                .add_diag(diag!(Declarations::InvalidModule, (loc, msg)));
            Address::Numerical(None, sp(loc, NumericalAddress::DEFAULT_ERROR_ADDRESS))
        },
    })
}

// This is a hack to recognize APTOS StdLib, Framework, and Token libs to avoid warnings on some old errors.
// This will be removed after library attributes are cleaned up.
// (See https://github.com/aptos-labs/aptos-core/issues/9410)
fn module_is_in_aptos_libs(module_address: Option<Spanned<Address>>) -> bool {
    const APTOS_STDLIB_NAME: &str = "aptos_std";
    static APTOS_STDLIB_NUMERICAL_ADDRESS: Lazy<NumericalAddress> =
        Lazy::new(|| NumericalAddress::parse_str("0x1").unwrap());
    const APTOS_FRAMEWORK_NAME: &str = "aptos_framework";
    static APTOS_FRAMEWORK_NUMERICAL_ADDRESS: Lazy<NumericalAddress> =
        Lazy::new(|| NumericalAddress::parse_str("0x1").unwrap());
    const APTOS_TOKEN_NAME: &str = "aptos_token";
    static APTOS_TOKEN_NUMERICAL_ADDRESS: Lazy<NumericalAddress> =
        Lazy::new(|| NumericalAddress::parse_str("0x3").unwrap());
    const APTOS_TOKEN_OBJECTS_NAME: &str = "aptos_token_objects";
    static APTOS_TOKEN_OBJECTS_NUMERICAL_ADDRESS: Lazy<NumericalAddress> =
        Lazy::new(|| NumericalAddress::parse_str("0x4").unwrap());
    match &module_address {
        Some(spanned_address) => {
            let address = spanned_address.value;
            match address {
                Address::Numerical(optional_name, spanned_numerical_address) => match optional_name
                {
                    Some(spanned_symbol) => {
                        ((&spanned_symbol.value as &str) == APTOS_STDLIB_NAME
                            && (spanned_numerical_address.value == *APTOS_STDLIB_NUMERICAL_ADDRESS))
                            || ((&spanned_symbol.value as &str) == APTOS_FRAMEWORK_NAME
                                && (spanned_numerical_address.value
                                    == *APTOS_FRAMEWORK_NUMERICAL_ADDRESS))
                            || ((&spanned_symbol.value as &str) == APTOS_TOKEN_NAME
                                && (spanned_numerical_address.value
                                    == *APTOS_TOKEN_NUMERICAL_ADDRESS))
                            || ((&spanned_symbol.value as &str) == APTOS_TOKEN_OBJECTS_NAME
                                && (spanned_numerical_address.value
                                    == *APTOS_TOKEN_OBJECTS_NUMERICAL_ADDRESS))
                    },
                    None => false,
                },
                Address::NamedUnassigned(_) => false,
            }
        },
        None => false,
    }
}

fn module_(
    context: &mut Context,
    package_name: Option<Symbol>,
    module_address: Option<Spanned<Address>>,
    mdef: P::ModuleDefinition,
) -> (ModuleIdent, E::ModuleDefinition) {
    let P::ModuleDefinition {
        attributes,
        loc,
        address,
        is_spec_module: _,
        name,
        members,
    } = mdef;
    let attributes = flatten_attributes(context, AttributePosition::Module, attributes);

    assert!(context.address.is_none());
    assert!(address.is_none());
    set_sender_address(context, &name, module_address);
    let _ = check_restricted_name_all_cases(context, NameCase::Module, &name.0);
    if name.value().starts_with(|c| c == '_') {
        let msg = format!(
            "Invalid module name '{}'. Module names cannot start with '_'",
            name,
        );
        context
            .env
            .add_diag(diag!(Declarations::InvalidName, (name.loc(), msg)));
    }

    let name_loc = name.0.loc;
    let current_module = sp(name_loc, ModuleIdent_::new(*context.cur_address(), name));
    if context
        .module_deprecation_attribute_locs
        .contains_key(&current_module)
    {
        context.in_deprecated_code = true;
    }
    if context.env.flags().warn_of_deprecation_use_in_aptos_libs() {
        context.in_aptos_libs = false;
    } else {
        context.in_aptos_libs = module_is_in_aptos_libs(module_address);
    }

    let mut new_scope = AliasMapBuilder::new();
    module_self_aliases(&mut new_scope, &current_module);

    // Make a copy of the original UseDecls, to be passed on to the expansion AST before they are
    // processed here.
    let use_decls = members
        .iter()
        .filter_map(|member| {
            if let ModuleMember::Use(decl) = member {
                Some(decl.clone())
            } else {
                None
            }
        })
        .collect();

    let members = members
        .into_iter()
        .filter_map(|member| aliases_from_member(context, &mut new_scope, &current_module, member))
        .collect::<Vec<_>>();
    let old_aliases = context.aliases.add_and_shadow_all(new_scope);
    assert!(
        old_aliases.is_empty(),
        "ICE there should be no aliases entering a module"
    );
    context.set_current_module(Some(current_module));
    let mut friends = UniqueMap::new();
    let mut functions = UniqueMap::new();
    let mut constants = UniqueMap::new();
    let mut structs = UniqueMap::new();
    let mut specs = vec![];
    for member in members {
        match member {
            P::ModuleMember::Use(_) => unreachable!(),
            P::ModuleMember::Friend(f) => friend(context, &mut friends, f),
            P::ModuleMember::Function(mut f) => {
                if !context.is_source_definition && !f.inline {
                    f.body.value = P::FunctionBody_::Native
                }
                function(context, &mut functions, f)
            },
            P::ModuleMember::Constant(c) => constant(context, &mut constants, c),
            P::ModuleMember::Struct(s) => struct_def(context, &mut structs, s),
            P::ModuleMember::Spec(s) => specs.push(spec(context, s)),
        }
    }
    context.set_to_outer_scope(old_aliases);

    let def = E::ModuleDefinition {
        package_name,
        attributes,
        loc,
        is_source_module: context.is_source_definition,
        dependency_order: 0,
        immediate_neighbors: UniqueMap::new(),
        used_addresses: BTreeSet::new(),
        friends,
        structs,
        constants,
        functions,
        specs,
        use_decls,
    };
    (current_module, def)
}

fn script(
    context: &mut Context,
    scripts: &mut Vec<E::Script>,
    package_name: Option<Symbol>,
    pscript: P::Script,
) {
    scripts.push(script_(context, package_name, pscript))
}

fn script_(context: &mut Context, package_name: Option<Symbol>, pscript: P::Script) -> E::Script {
    assert!(context.address.is_none());
    assert!(context.is_source_definition);
    let P::Script {
        attributes,
        loc,
        uses: puses,
        constants: pconstants,
        function: pfunction,
        specs: pspecs,
    } = pscript;

    let attributes = flatten_attributes(context, AttributePosition::Script, attributes);
    let new_scope = uses(context, puses.clone());
    let old_aliases = context.aliases.add_and_shadow_all(new_scope);
    assert!(
        old_aliases.is_empty(),
        "ICE there should be no aliases entering a script"
    );
    context.set_current_module(None);
    context.in_aptos_libs = false;

    let mut constants = UniqueMap::new();
    for c in pconstants {
        // TODO remove after Self rework
        check_valid_module_member_name(context, ModuleMemberKind::Constant, c.name.0);
        constant(context, &mut constants, c);
    }

    // TODO remove after Self rework
    check_valid_module_member_name(context, ModuleMemberKind::Function, pfunction.name.0);
    let (function_name, function) = function_(context, pfunction);
    match &function.visibility {
        E::Visibility::Public(loc) | E::Visibility::Package(loc) | E::Visibility::Friend(loc) => {
            let msg = format!(
                "Invalid '{}' visibility modifier. \
                Script functions are not callable from other Move functions.",
                function.visibility,
            );
            context
                .env
                .add_diag(diag!(Declarations::UnnecessaryItem, (*loc, msg)));
        },
        E::Visibility::Internal => (),
    }
    match &function.body {
        sp!(_, E::FunctionBody_::Defined(_)) => (),
        sp!(loc, E::FunctionBody_::Native) => {
            context.env.add_diag(diag!(
                Declarations::InvalidScript,
                (
                    *loc,
                    "Invalid 'native' function. 'script' functions must have a defined body"
                )
            ));
        },
    }
    let specs = specs(context, pspecs);
    context.set_to_outer_scope(old_aliases);

    E::Script {
        package_name,
        attributes,
        loc,
        immediate_neighbors: UniqueMap::new(),
        used_addresses: BTreeSet::new(),
        constants,
        function_name,
        function,
        specs,
        use_decls: puses,
    }
}

/// If attributes contains a `#[deprecated]` attribute, then returns the location of the attribute.
fn deprecated_attribute_location(attributes: &[P::Attributes]) -> Option<Loc> {
    attributes
        .iter()
        .flat_map(|attrs| &attrs.value)
        .filter_map(|attr| {
            let sp!(nloc, sym) = match &attr.value {
                P::Attribute_::Name(n)
                | P::Attribute_::Assigned(n, _)
                | P::Attribute_::Parameterized(n, _) => *n,
            };
            match KnownAttribute::resolve(sym) {
                Some(KnownAttribute::Deprecation(_dep)) => Some(nloc),
                _ => None,
            }
        })
        .next()
}

fn flatten_attributes(
    context: &mut Context,
    attr_position: AttributePosition,
    attributes: Vec<P::Attributes>,
) -> E::Attributes {
    let all_attrs = attributes
        .into_iter()
        .flat_map(|attrs| attrs.value)
        .flat_map(|attr| attribute(context, attr_position, attr))
        .collect::<Vec<_>>();
    unique_attributes(context, attr_position, false, all_attrs)
}

fn unique_attributes(
    context: &mut Context,
    attr_position: AttributePosition,
    is_nested: bool,
    attributes: impl IntoIterator<Item = E::Attribute>,
) -> E::Attributes {
    let mut attr_map = UniqueMap::new();
    for sp!(loc, attr_) in attributes {
        let sp!(nloc, sym) = match &attr_ {
            E::Attribute_::Name(n)
            | E::Attribute_::Assigned(n, _)
            | E::Attribute_::Parameterized(n, _) => *n,
        };
        let name_ = match KnownAttribute::resolve(sym) {
            None => {
                let flags = &context.env.flags();
                if !flags.skip_attribute_checks() {
                    let known_attributes = &context.env.get_known_attributes();
                    if !is_nested && !known_attributes.contains(sym.as_str()) {
                        let msg = format!("Attribute name '{}' is unknown (use --{} CLI option to ignore); known attributes are '{:?}'.",
					                      sym.as_str(),
					                      SKIP_ATTRIBUTE_CHECKS, known_attributes);
                        context
                            .env
                            .add_diag(diag!(Declarations::UnknownAttribute, (nloc, msg)));
                    } else if is_nested && known_attributes.contains(sym.as_str()) {
                        let msg = format!(
                            "Attribute '{}' is not expected in a nested attribute position.",
                            sym.as_str()
                        );
                        context
                            .env
                            .add_diag(diag!(Declarations::InvalidAttribute, (nloc, msg)));
                    };
                }
                E::AttributeName_::Unknown(sym)
            },
            Some(known) => {
                debug_assert!(known.name() == sym.as_str());
                if is_nested {
                    let msg = format!(
                        "Attribute '{}' is not expected in a nested attribute position",
                        sym.as_str()
                    );
                    context
                        .env
                        .add_diag(diag!(Declarations::InvalidAttribute, (nloc, msg)));
                    continue;
                }

                let expected_positions = known.expected_positions();
                if !expected_positions.contains(&attr_position) {
                    let msg = format!(
                        "Attribute '{}' is not expected with a {}",
                        known.name(),
                        attr_position
                    );
                    let all_expected = expected_positions
                        .iter()
                        .map(|p| format!("{}", p))
                        .collect::<Vec<_>>()
                        .join(", ");
                    let expected_msg = format!(
                        "Expected to be used with one of the following: {}",
                        all_expected
                    );
                    context.env.add_diag(diag!(
                        Declarations::InvalidAttribute,
                        (nloc, msg),
                        (nloc, expected_msg)
                    ));
                    continue;
                }
                E::AttributeName_::Known(known)
            },
        };
        if let Err((_, old_loc)) = attr_map.add(sp(nloc, name_), sp(loc, attr_)) {
            let msg = format!("Duplicate attribute '{}' attached to the same item", name_);
            context.env.add_diag(diag!(
                Declarations::DuplicateItem,
                (loc, msg),
                (old_loc, "Attribute previously given here"),
            ));
        }
    }
    attr_map
}

fn attribute(
    context: &mut Context,
    attr_position: AttributePosition,
    sp!(loc, attribute_): P::Attribute,
) -> Option<E::Attribute> {
    use E::Attribute_ as EA;
    use P::Attribute_ as PA;
    Some(sp(loc, match attribute_ {
        PA::Name(n) => EA::Name(n),
        PA::Assigned(n, v) => EA::Assigned(n, Box::new(attribute_value(context, *v)?)),
        PA::Parameterized(n, sp!(_, pattrs_)) => {
            let attrs = pattrs_
                .into_iter()
                .map(|a| attribute(context, attr_position, a))
                .collect::<Option<Vec<_>>>()?;
            EA::Parameterized(n, unique_attributes(context, attr_position, true, attrs))
        },
    }))
}

fn check_module_name(context: &mut Context, ident_loc: &Loc, mident: &ModuleIdent) {
    match context.module_members.get(mident) {
        None => {
            context.env.add_diag(diag!(
                NameResolution::UnboundModule,
                (*ident_loc, format!("Unbound module '{}'", mident))
            ));
        },
        Some(_module) => {
            check_for_deprecated_module_use(context, mident);
        },
    }
}

/// Can `address_name::module_name` be resolved to a valid module access?
///      ^^^^^^^^^^^^^^^^^^^^^^^^^ is the `full_loc`
///      ^^^^^^^^^^^^              is the `address_loc`
fn can_be_resolved_as_module(
    context: &mut Context,
    full_loc: Loc,
    address_loc: Loc,
    address_name: Name,
    module_name: Name,
) -> Option<ModuleIdent> {
    use P::LeadingNameAccess_ as LN;
    if context
        .named_address_mapping
        .as_ref()
        .map(|m| m.contains_key(&address_name.value))
        .unwrap_or(false)
    {
        // We have `address_name` resolved to `addr`.
        let addr = address(context, false, sp(address_loc, LN::Name(address_name)));
        let mident = sp(full_loc, ModuleIdent_::new(addr, ModuleName(module_name)));
        // Is `addr::module_name` a resolvable module? If so, return it.
        if context.module_members.contains_key(&mident) {
            check_for_deprecated_module_use(context, &mident);
            Some(mident)
        } else {
            None
        }
    } else {
        None
    }
}

fn attribute_value(
    context: &mut Context,
    sp!(loc, avalue_): P::AttributeValue,
) -> Option<E::AttributeValue> {
    use E::AttributeValue_ as EV;
    use P::{AttributeValue_ as PV, LeadingNameAccess_ as LN, NameAccessChain_ as PN};
    Some(sp(loc, match avalue_ {
        PV::Value(v) => EV::Value(value(context, v)?),
        PV::ModuleAccess(sp!(ident_loc, PN::Two(sp!(aloc, LN::AnonymousAddress(a)), n))) => {
            let addr = Address::Numerical(None, sp(aloc, a));
            let mident = sp(ident_loc, ModuleIdent_::new(addr, ModuleName(n)));
            check_module_name(context, &ident_loc, &mident);
            EV::Module(mident)
        },
        // bit wonky, but this is the only spot currently where modules and expressions exist
        // in the same namespace.
        // TODO consider if we want to just force all of these checks into the well-known
        // attribute setup
        PV::ModuleAccess(sp!(ident_loc, PN::One(n)))
            if context.aliases.module_alias_get(&n).is_some() =>
        {
            let sp!(_, mident_) = context.aliases.module_alias_get(&n).unwrap();
            let mident = sp(ident_loc, mident_);
            check_module_name(context, &ident_loc, &mident);
            EV::Module(mident)
        },
        PV::ModuleAccess(ma) => {
            let value = match ma {
                sp!(ident_loc, PN::Two(sp!(aloc, LN::Name(n1)), n2)) => {
                    // Check to see if `n1::n2` can be resolved to be module access.
                    can_be_resolved_as_module(context, ident_loc, aloc, n1, n2)
                },
                _ => None,
            };
            match value {
                Some(mident) => EV::Module(mident),
                None => EV::ModuleAccess(name_access_chain(
                    context,
                    Access::Type,
                    ma,
                    Some(DeprecatedItem::Module),
                )?),
            }
        },
    }))
}

//**************************************************************************************************
// Aliases
//**************************************************************************************************

/// Process the PackageDefinition refs provided by the defs iterator,
/// adding all symbol definitions to members, which records
/// moduleId->SpannedSymbol->ModuleMemberInfo.  Also add a record
/// for each deprecated module to module_deprecation_attribute_locs.
fn all_module_members<'a>(
    compilation_env: &mut CompilationEnv,
    named_addr_maps: &NamedAddressMaps,
    members: &mut UniqueMap<ModuleIdent, ModuleMembers>,
    module_deprecation_attribute_locs: &mut BTreeMap<ModuleIdent, Loc>,
    always_add: bool,
    defs: impl IntoIterator<Item = &'a P::PackageDefinition>,
) {
    for P::PackageDefinition {
        named_address_map,
        def,
        ..
    } in defs
    {
        let named_addr_map = named_addr_maps.get(*named_address_map);
        match def {
            P::Definition::Module(m) => {
                let addr = match &m.address {
                    Some(a) => {
                        address_(
                            compilation_env,
                            named_addr_map,
                            /* suggest_declaration */ true,
                            *a,
                        )
                    },
                    // Error will be handled when the module is compiled
                    None => {
                        Address::Numerical(None, sp(m.loc, NumericalAddress::DEFAULT_ERROR_ADDRESS))
                    },
                };
                let mident = sp(m.name.loc(), ModuleIdent_::new(addr, m.name));
                module_members(members, always_add, m, &mident);
                if let Some(loc) = deprecated_attribute_location(&m.attributes) {
                    module_deprecation_attribute_locs.insert(mident, loc);
                }
            },
            P::Definition::Address(addr_def) => {
                let addr = address_(
                    compilation_env,
                    named_addr_map,
                    /* suggest_declaration */ false,
                    addr_def.addr,
                );
                for m in &addr_def.modules {
                    let mident = sp(m.name.loc(), ModuleIdent_::new(addr, m.name));
                    module_members(members, always_add, m, &mident);
                    if let Some(loc) = deprecated_attribute_location(&addr_def.attributes) {
                        module_deprecation_attribute_locs.insert(mident, loc);
                    } else if let Some(loc) = deprecated_attribute_location(&m.attributes) {
                        module_deprecation_attribute_locs.insert(mident, loc);
                    }
                }
            },
            P::Definition::Script(_) => (),
        }
    }
}

/// Record ModuleMemberInfo about a specified member name, including
/// info about any deprecation found in attributes.
fn record_module_member_info(
    cur_members: &mut BTreeMap<Spanned<Symbol>, ModuleMemberInfo>,
    name: &Spanned<Symbol>,
    attributes: &[P::Attributes],
    member_kind: ModuleMemberKind,
) {
    cur_members.insert(*name, ModuleMemberInfo {
        kind: member_kind,
        deprecation: deprecated_attribute_location(attributes),
    });
}

/// Record ModuleMemberInfo about a specified member name, skipping
/// deprecation info (as for a spec member).
fn record_module_member_info_without_deprecation(
    cur_members: &mut BTreeMap<Spanned<Symbol>, ModuleMemberInfo>,
    name: &Spanned<Symbol>,
    member_kind: ModuleMemberKind,
) {
    cur_members.insert(*name, ModuleMemberInfo {
        kind: member_kind,
        deprecation: None,
    });
}

/// Specified module with identifier mident and definition m,
/// add MemberInfo about each defined member to the members map.
/// This currently includes ModuleMemberKind and deprecation info.
/// If always_add is not false, then a module is processed only if it
/// is already present in the map (as for a module in the stdlibs).
fn module_members(
    members: &mut UniqueMap<ModuleIdent, ModuleMembers>,
    always_add: bool,
    m: &P::ModuleDefinition,
    mident: &ModuleIdent,
) {
    if !always_add && members.contains_key(mident) {
        return;
    }
    let mut cur_members = members.remove(mident).unwrap_or_default();
    for mem in &m.members {
        use P::{SpecBlockMember_ as SBM, SpecBlockTarget_ as SBT, SpecBlock_ as SB};
        match mem {
            P::ModuleMember::Function(f) => {
                record_module_member_info(
                    &mut cur_members,
                    &f.name.0,
                    &f.attributes,
                    ModuleMemberKind::Function,
                );
            },
            P::ModuleMember::Constant(c) => {
                record_module_member_info(
                    &mut cur_members,
                    &c.name.0,
                    &c.attributes,
                    ModuleMemberKind::Constant,
                );
            },
            P::ModuleMember::Struct(s) => {
                record_module_member_info(
                    &mut cur_members,
                    &s.name.0,
                    &s.attributes,
                    ModuleMemberKind::Struct,
                );
            },
            P::ModuleMember::Spec(
                sp!(_, SB {
                    target,
                    members,
                    ..
                }),
            ) => match &target.value {
                SBT::Schema(n, _) => {
                    record_module_member_info_without_deprecation(
                        &mut cur_members,
                        n,
                        ModuleMemberKind::Schema,
                    );
                },
                SBT::Module => {
                    for sp!(_, smember_) in members {
                        if let SBM::Function { name, .. } = smember_ {
                            record_module_member_info_without_deprecation(
                                &mut cur_members,
                                &name.0,
                                ModuleMemberKind::Function,
                            );
                        }
                    }
                },
                _ => (),
            },
            P::ModuleMember::Use(_) | P::ModuleMember::Friend(_) => (),
        };
    }
    members.add(*mident, cur_members).unwrap();
}

fn module_self_aliases(acc: &mut AliasMapBuilder, current_module: &ModuleIdent) {
    let self_name = sp(current_module.loc, ModuleName::SELF_NAME.into());
    acc.add_implicit_module_alias(self_name, *current_module)
        .unwrap()
}

fn aliases_from_member(
    context: &mut Context,
    acc: &mut AliasMapBuilder,
    current_module: &ModuleIdent,
    member: P::ModuleMember,
) -> Option<P::ModuleMember> {
    use P::{SpecBlockMember_ as SBM, SpecBlockTarget_ as SBT, SpecBlock_ as SB};
    macro_rules! check_name_and_add_implicit_alias {
        ($kind:expr, $name:expr) => {{
            if let Some(n) = check_valid_module_member_name(context, $kind, $name) {
                if let Err(loc) =
                    acc.add_implicit_member_alias(n.clone(), current_module.clone(), n.clone())
                {
                    duplicate_module_member(context, loc, n)
                }
            }
        }};
    }

    match member {
        P::ModuleMember::Use(u) => {
            use_(context, acc, u);
            None
        },
        f @ P::ModuleMember::Friend(_) => {
            // friend declarations do not produce implicit aliases
            Some(f)
        },
        P::ModuleMember::Function(f) => {
            let n = f.name.0;
            check_name_and_add_implicit_alias!(ModuleMemberKind::Function, n);
            Some(P::ModuleMember::Function(f))
        },
        P::ModuleMember::Constant(c) => {
            let n = c.name.0;
            check_name_and_add_implicit_alias!(ModuleMemberKind::Constant, n);
            Some(P::ModuleMember::Constant(c))
        },
        P::ModuleMember::Struct(s) => {
            let n = s.name.0;
            check_name_and_add_implicit_alias!(ModuleMemberKind::Struct, n);
            Some(P::ModuleMember::Struct(s))
        },
        P::ModuleMember::Spec(s) => {
            let sp!(_, SB {
                target,
                members,
                ..
            }) = &s;
            match &target.value {
                SBT::Schema(n, _) => {
                    check_name_and_add_implicit_alias!(ModuleMemberKind::Schema, *n);
                },
                SBT::Module => {
                    for sp!(_, smember_) in members {
                        if let SBM::Function { name, .. } = smember_ {
                            let n = name.0;
                            check_name_and_add_implicit_alias!(ModuleMemberKind::Function, n);
                        }
                    }
                },
                _ => (),
            };
            Some(P::ModuleMember::Spec(s))
        },
    }
}

fn uses(context: &mut Context, uses: Vec<P::UseDecl>) -> AliasMapBuilder {
    let mut new_scope = AliasMapBuilder::new();
    for u in uses {
        use_(context, &mut new_scope, u);
    }
    new_scope
}

fn warn_about_unbound_module_use(context: &mut Context, mident: &ModuleIdent) {
    context.env.add_diag(diag!(
        NameResolution::UnboundModule,
        (
            mident.loc,
            format!("Invalid 'use'. Unbound module: '{}'", mident.value),
        )
    ));
}

fn module_has_deprecated_annotation(context: &mut Context, mident: &ModuleIdent) -> Option<Loc> {
    context
        .module_deprecation_attribute_locs
        .get(mident)
        .copied()
}

fn member_has_deprecated_annotation(
    context: &mut Context,
    mident: &ModuleIdent,
    member: &Spanned<Symbol>,
) -> Option<Loc> {
    context
        .module_members
        .get(mident)
        .and_then(|members| members.get(member))
        .and_then(|member_info| member_info.deprecation)
}

fn check_for_deprecated_module_use(context: &mut Context, mident: &ModuleIdent) -> bool {
    let warn_deprecation = &context.env.flags().warn_of_deprecation_use();
    if !warn_deprecation || context.in_deprecated_code || context.in_aptos_libs {
        return false;
    }
    if let Some(loc) = module_has_deprecated_annotation(context, mident) {
        context.env.add_diag(diag!(
            NameResolution::DeprecatedModule,
            (
                mident.loc,
                format!("Use of deprecated module '{}'", mident.value),
            ),
            (loc, format!("Module '{}' deprecated here", mident.value),),
        ));
        true
    } else {
        false
    }
}

fn check_for_deprecated_member_use(
    context: &mut Context,
    mident_in: Option<&ModuleIdent>,
    member: &Spanned<Symbol>,
    deprecated_item: DeprecatedItem,
) {
    let warn_deprecation = &context.env.flags().warn_of_deprecation_use();
    if !warn_deprecation || context.in_deprecated_code || context.in_aptos_libs {
        return;
    }
    let mident = match mident_in {
        None => {
            if let Some(mident) = context.current_module() {
                *mident
            } else {
                // No module, we must be in a script.
                return;
            }
        },
        Some(mident) => {
            check_for_deprecated_module_use(context, mident);
            *mident
        },
    };
    if let Some(loc) = member_has_deprecated_annotation(context, &mident, member) {
        context.env.add_diag(diag!(
            deprecated_item.get_code(),
            (
                member.loc,
                format!(
                    "Use of deprecated {} '{}' from module '{}'",
                    deprecated_item.get_string(),
                    member,
                    mident
                )
            ),
            (
                loc,
                format!(
                    "{} '{}' in module '{}' deprecated here",
                    deprecated_item.get_capitalized_string(),
                    member,
                    mident
                )
            ),
        ));
    }
}

fn use_(context: &mut Context, acc: &mut AliasMapBuilder, u: P::UseDecl) {
    let P::UseDecl {
        use_: u,
        attributes,
    } = u;
    flatten_attributes(context, AttributePosition::Use, attributes);
    macro_rules! add_module_alias {
        ($ident:expr, $alias_opt:expr) => {{
            let alias: Name = $alias_opt.unwrap_or_else(|| $ident.value.module.0.clone());
            if let Err(()) = check_restricted_name_all_cases(context, NameCase::ModuleAlias, &alias)
            {
                return;
            }

            if let Err(old_loc) = acc.add_module_alias(alias.clone(), $ident) {
                duplicate_module_alias(context, old_loc, alias)
            }
        }};
    }
    match u {
        P::Use::Module(pmident, alias_opt) => {
            let mident = module_ident(context, pmident);
            if !context.module_members.contains_key(&mident) {
                warn_about_unbound_module_use(context, &mident);
                return;
            };
            check_for_deprecated_module_use(context, &mident);
            add_module_alias!(mident, alias_opt.map(|m| m.0))
        },
        P::Use::Members(pmident, sub_uses) => {
            let mident = module_ident(context, pmident);
            if !context.module_members.contains_key(&mident) {
                warn_about_unbound_module_use(context, &mident);
                return;
            }
            check_for_deprecated_module_use(context, &mident);
            let members = context.module_members.get(&mident).unwrap();
            let mloc = *context.module_members.get_loc(&mident).unwrap();
            let sub_uses_kinds = sub_uses
                .into_iter()
                .map(|(member, alia_opt)| {
                    let kind = members.get(&member).map(|x| x.kind);
                    (member, alia_opt, kind)
                })
                .collect::<Vec<_>>();

            for (member, alias_opt, member_kind_opt) in sub_uses_kinds {
                if member.value.as_str() == ModuleName::SELF_NAME {
                    add_module_alias!(mident, alias_opt);
                    continue;
                }

                // check is member
                let member_kind = match member_kind_opt {
                    None => {
                        let msg = format!(
                            "Invalid 'use'. Unbound member '{}' in module '{}'",
                            member, mident
                        );
                        context.env.add_diag(diag!(
                            NameResolution::UnboundModuleMember,
                            (member.loc, msg),
                            (mloc, format!("Module '{}' declared here", mident)),
                        ));
                        continue;
                    },
                    Some(m) => m,
                };
                let deprecated_item_kind = match member_kind {
                    ModuleMemberKind::Constant => DeprecatedItem::Constant,
                    ModuleMemberKind::Function => DeprecatedItem::Function,

                    ModuleMemberKind::Struct => DeprecatedItem::Struct,
                    _ => DeprecatedItem::Member,
                };
                check_for_deprecated_member_use(
                    context,
                    Some(&mident),
                    &member,
                    deprecated_item_kind,
                );

                let alias = alias_opt.unwrap_or(member);

                let alias = match check_valid_module_member_alias(context, member_kind, alias) {
                    None => continue,
                    Some(alias) => alias,
                };
                if let Err(old_loc) = acc.add_member_alias(alias, mident, member) {
                    duplicate_module_member(context, old_loc, alias)
                }
            }
        },
    }
}

fn duplicate_module_alias(context: &mut Context, old_loc: Loc, alias: Name) {
    let msg = format!(
        "Duplicate module alias '{}'. Module aliases must be unique within a given namespace",
        alias
    );
    context.env.add_diag(diag!(
        Declarations::DuplicateItem,
        (alias.loc, msg),
        (old_loc, "Alias previously defined here"),
    ));
}

fn duplicate_module_member(context: &mut Context, old_loc: Loc, alias: Name) {
    let msg = format!(
        "Duplicate module member or alias '{}'. Top level names in a namespace must be unique",
        alias
    );
    context.env.add_diag(diag!(
        Declarations::DuplicateItem,
        (alias.loc, msg),
        (old_loc, "Alias previously defined here"),
    ));
}

fn unused_alias(context: &mut Context, alias: Name) {
    if !context.is_source_definition {
        return;
    }

    context.env.add_diag(diag!(
        UnusedItem::Alias,
        (
            alias.loc,
            format!("Unused 'use' of alias '{}'. Consider removing it", alias)
        ),
    ))
}

//**************************************************************************************************
// Structs
//**************************************************************************************************

fn struct_def(
    context: &mut Context,
    structs: &mut UniqueMap<StructName, E::StructDefinition>,
    pstruct: P::StructDefinition,
) {
    let (sname, sdef) = struct_def_(context, pstruct);
    if let Err(_old_loc) = structs.add(sname, sdef) {
        assert!(context.env.has_errors())
    }
}

fn struct_def_(
    context: &mut Context,
    pstruct: P::StructDefinition,
) -> (StructName, E::StructDefinition) {
    let P::StructDefinition {
        attributes,
        loc,
        name,
        abilities: abilities_vec,
        type_parameters: pty_params,
        layout: pfields,
    } = pstruct;
    let was_in_deprecated_code = context.enter_possibly_deprecated_member(&name.0);
    let attributes = flatten_attributes(context, AttributePosition::Struct, attributes);
    let type_parameters = struct_type_parameters(context, pty_params);
    let old_aliases = context
        .aliases
        .shadow_for_type_parameters(type_parameters.iter().map(|tp| &tp.name));
    let abilities = ability_set(context, "modifier", abilities_vec);
    let fields = struct_layout(context, &name, pfields);
    let sdef = E::StructDefinition {
        attributes,
        loc,
        abilities,
        type_parameters,
        layout: fields,
    };
    context.set_to_outer_scope(old_aliases);
    context.set_in_deprecated_code(was_in_deprecated_code);
    (name, sdef)
}

fn struct_layout(
    context: &mut Context,
    _sname: &StructName,
    parsed_layout: P::StructLayout,
) -> E::StructLayout {
    match parsed_layout {
        P::StructLayout::Native(loc) => E::StructLayout::Native(loc),
        P::StructLayout::Singleton(fields, is_positional) => {
            E::StructLayout::Singleton(struct_fields(context, fields), is_positional)
        },
        P::StructLayout::Variants(variants) => {
            let mut previous_variants = BTreeMap::new();
            E::StructLayout::Variants(
                variants
                    .into_iter()
                    .map(|v| {
                        if !is_valid_struct_constant_or_schema_name(v.name.0.value.as_str()) {
                            let msg = format!(
                                "Invalid variant name '{}'. variant names must start with 'A'..'Z'",
                                v.name
                            );
                            context
                                .env
                                .add_diag(diag!(Declarations::InvalidName, (v.loc, msg)));
                        }
                        if let Some(old_loc) = previous_variants.insert(v.name, v.loc) {
                            context.env.add_diag(diag!(
                                Declarations::DuplicateItem,
                                (
                                    v.loc,
                                    format!("Duplicate definition for variant '{}'", v.name),
                                ),
                                (old_loc, "Variant previously defined here"),
                            ));
                        }
                        E::StructVariant {
                            attributes: flatten_attributes(
                                context,
                                AttributePosition::Struct,
                                v.attributes,
                            ),
                            loc: v.loc,
                            name: v.name,
                            fields: struct_fields(context, v.fields),
                            is_positional: v.is_positional,
                        }
                    })
                    .collect(),
            )
        },
    }
}

fn struct_fields(context: &mut Context, fields: Vec<(P::Field, P::Type)>) -> Fields<E::Type> {
    let mut field_map = UniqueMap::new();
    for (idx, (field, pt)) in fields.into_iter().enumerate() {
        let t = type_(context, pt);
        if let Err((field, old_loc)) = field_map.add(field, (idx, t)) {
            context.env.add_diag(diag!(
                Declarations::DuplicateItem,
                (
                    field.loc(),
                    format!("Duplicate definition for field '{}'", field),
                ),
                (old_loc, "Field previously defined here"),
            ));
        }
    }
    field_map
}

//**************************************************************************************************
// Friends
//**************************************************************************************************

fn friend(
    context: &mut Context,
    friends: &mut UniqueMap<ModuleIdent, E::Friend>,
    pfriend: P::FriendDecl,
) {
    match friend_(context, pfriend) {
        Some((mident, friend)) => match friends.get(&mident) {
            None => friends.add(mident, friend).unwrap(),
            Some(old_friend) => {
                let msg = format!(
                    "Duplicate friend declaration '{}'. Friend declarations in a module must be \
                     unique",
                    mident
                );
                context.env.add_diag(diag!(
                    Declarations::DuplicateItem,
                    (friend.loc, msg),
                    (old_friend.loc, "Friend previously declared here"),
                ));
            },
        },
        None => assert!(context.env.has_errors()),
    };
}

fn friend_(context: &mut Context, pfriend_decl: P::FriendDecl) -> Option<(ModuleIdent, E::Friend)> {
    assert!(context.exp_specs.is_empty());
    let P::FriendDecl {
        attributes: pattributes,
        loc,
        friend: pfriend,
    } = pfriend_decl;
    let mident = name_access_chain_to_module_ident(context, pfriend)?;
    let attributes = flatten_attributes(context, AttributePosition::Friend, pattributes);
    Some((mident, E::Friend { attributes, loc }))
}

//**************************************************************************************************
// Constants
//**************************************************************************************************

fn constant(
    context: &mut Context,
    constants: &mut UniqueMap<ConstantName, E::Constant>,
    pconstant: P::Constant,
) {
    let (name, constant) = constant_(context, pconstant);
    if let Err(_old_loc) = constants.add(name, constant) {
        assert!(context.env.has_errors())
    }
}

fn constant_(context: &mut Context, pconstant: P::Constant) -> (ConstantName, E::Constant) {
    assert!(context.exp_specs.is_empty());
    let P::Constant {
        attributes: pattributes,
        loc,
        name,
        signature: psignature,
        value: pvalue,
    } = pconstant;
    let was_in_deprecated_code = context.enter_possibly_deprecated_member(&name.0);
    let attributes = flatten_attributes(context, AttributePosition::Constant, pattributes);
    let signature = type_(context, psignature);
    let value = exp_(context, pvalue);
    let _specs = context.extract_exp_specs();
    let constant = E::Constant {
        attributes,
        loc,
        signature,
        value,
    };
    context.set_in_deprecated_code(was_in_deprecated_code);
    (name, constant)
}

//**************************************************************************************************
// Functions
//**************************************************************************************************

fn function(
    context: &mut Context,
    functions: &mut UniqueMap<FunctionName, E::Function>,
    pfunction: P::Function,
) {
    let (fname, fdef) = function_(context, pfunction);
    if let Err(_old_loc) = functions.add(fname, fdef) {
        assert!(context.env.has_errors())
    }
}

fn function_(context: &mut Context, pfunction: P::Function) -> (FunctionName, E::Function) {
    let P::Function {
        attributes: pattributes,
        loc,
        inline,
        name,
        visibility: pvisibility,
        entry,
        signature: psignature,
        body: pbody,
        access_specifiers,
    } = pfunction;
    assert!(context.exp_specs.is_empty());
    let was_in_deprecated_code = context.enter_possibly_deprecated_member(&name.0);
    let attributes = flatten_attributes(context, AttributePosition::Function, pattributes);
    let visibility = visibility(pvisibility);
    let (old_aliases, signature) = function_signature(context, psignature);
    let (acquires, access_specifiers) = (vec![], access_specifier_list(context, access_specifiers));
    let body = function_body(context, pbody);
    let specs = context.extract_exp_specs();
    let fdef = E::Function {
        attributes,
        loc,
        inline,
        visibility,
        entry,
        signature,
        acquires,
        access_specifiers,
        body,
        specs,
    };
    context.set_to_outer_scope(old_aliases);
    context.set_in_deprecated_code(was_in_deprecated_code);
    (name, fdef)
}

fn access_specifier_list(
    context: &mut Context,
    access_specifiers: Option<Vec<P::AccessSpecifier>>,
) -> Option<Vec<E::AccessSpecifier>> {
    access_specifiers.map(|specs| {
        specs
            .into_iter()
            .map(|s| access_specifier(context, s))
            .collect::<Vec<_>>()
    })
}

fn invalid_variant_access(context: &mut Context, loc: Loc) {
    context.env.add_diag(diag!(
        Syntax::InvalidVariantAccess,
        (loc, "variant name not expected in this context".to_owned())
    ));
}

fn access_specifier(context: &mut Context, specifier: P::AccessSpecifier) -> E::AccessSpecifier {
    let (negated, kind, chain, type_args, address) = match specifier.value {
        AccessSpecifier_::Acquires(negated, chain, type_args, address) => (
            negated,
            AccessSpecifierKind::LegacyAcquires,
            chain,
            type_args,
            address,
        ),
        AccessSpecifier_::Reads(negated, chain, type_args, address) => (
            negated,
            AccessSpecifierKind::Reads,
            chain,
            type_args,
            address,
        ),
        AccessSpecifier_::Writes(negated, chain, type_args, address) => (
            negated,
            AccessSpecifierKind::Writes,
            chain,
            type_args,
            address,
        ),
    };
    let (module_address, module_name, resource_name) =
        access_specifier_name_access_chain(context, chain);
    let type_args = optional_types(context, type_args);
    let address = address_specifier(context, address);
    sp(specifier.loc, E::AccessSpecifier_ {
        kind,
        negated,
        module_address,
        module_name,
        resource_name,
        type_args,
        address,
    })
}

fn access_specifier_name_access_chain(
    context: &mut Context,
    chain: NameAccessChain,
) -> (Option<Address>, Option<ModuleName>, Option<Name>) {
    match chain.value {
        NameAccessChain_::Four(..) => {
            invalid_variant_access(context, chain.loc);
            (None, None, None)
        },
        NameAccessChain_::One(name) if name.value.as_str() == "*" => {
            // A single wildcard means any resource at the specified address, e.g. `*(0x2)`
            (None, None, None)
        },
        NameAccessChain_::One(name) => {
            // A single name is resolved as a member
            match context.aliases.member_alias_get(&name) {
                Some((mident, mem)) => (
                    Some(mident.value.address),
                    Some(mident.value.module),
                    Some(mem),
                ),
                None => (None, None, Some(name)),
            }
        },
        NameAccessChain_::Two(leading, second) => {
            match leading.value {
                LeadingNameAccess_::AnonymousAddress(_) => {
                    // An address with just one following name cannot be a resource,
                    // so we reject it
                    context.env.add_diag(diag!(
                        Syntax::InvalidAccessSpecifier,
                        (
                            chain.loc,
                            "address followed by single name is not a valid access specifier"
                                .to_owned()
                        )
                    ));
                    (None, None, None)
                },
                LeadingNameAccess_::Name(name) => {
                    if context
                        .named_address_mapping
                        .as_ref()
                        .unwrap()
                        .get(&name.value)
                        .is_some()
                    {
                        // This resolves as an address, so the second name must be a module,
                        // which we reject.
                        context.env.add_diag(diag!(
                            Syntax::InvalidAccessSpecifier,
                            (
                                chain.loc,
                                format!(
                                    "`{}` is an address alias which followed by a name is \
                                not a valid access specifier",
                                    name.value
                                )
                            )
                        ));
                        (None, None, None)
                    } else if let Some(ident) = context.aliases.module_alias_get(&name) {
                        // Resolves as a module alias
                        let ModuleIdent_ { address, module } = ident.value;
                        (Some(address), Some(module), Some(second))
                    } else {
                        context.env.add_diag(diag!(
                            NameResolution::UnboundModule,
                            (name.loc, format!("Unbound module alias '{}'", name))
                        ));
                        (None, None, None)
                    }
                },
            }
        },
        NameAccessChain_::Three(prefix, third) => {
            // This case is determined to be an address followed by module followed by resource
            let (leading, second) = prefix.value;
            let addr = match leading.value {
                LeadingNameAccess_::AnonymousAddress(addr) => addr,
                LeadingNameAccess_::Name(name) => {
                    if let Some(addr) = context
                        .named_address_mapping
                        .as_ref()
                        .unwrap()
                        .get(&name.value)
                    {
                        *addr
                    } else {
                        context
                            .env
                            .add_diag(address_without_value_error(false, name.loc, &name));
                        NumericalAddress::DEFAULT_ERROR_ADDRESS
                    }
                },
            };
            (
                Some(Address::Numerical(None, sp(leading.loc, addr))),
                Some(ModuleName(second)),
                Some(third),
            )
        },
    }
}

fn address_specifier(context: &mut Context, specifier: P::AddressSpecifier) -> E::AddressSpecifier {
    let s = match specifier.value {
        AddressSpecifier_::Empty => E::AddressSpecifier_::Empty,
        AddressSpecifier_::Any => E::AddressSpecifier_::Any,
        AddressSpecifier_::Literal(addr) => E::AddressSpecifier_::Literal(addr),
        AddressSpecifier_::Name(name) => E::AddressSpecifier_::Name(name),
        AddressSpecifier_::Call(chain, type_args, name) => {
            if let Some(maccess) = name_access_chain(
                context,
                Access::ApplyPositional,
                chain,
                Some(DeprecatedItem::Function),
            ) {
                E::AddressSpecifier_::Call(maccess, optional_types(context, type_args), name)
            } else {
                debug_assert!(context.env.has_errors());
                E::AddressSpecifier_::Any
            }
        },
    };
    sp(specifier.loc, s)
}

fn visibility(pvisibility: P::Visibility) -> E::Visibility {
    match pvisibility {
        P::Visibility::Public(loc) => E::Visibility::Public(loc),
        P::Visibility::Script(loc) => E::Visibility::Public(loc),
        P::Visibility::Package(loc) => E::Visibility::Package(loc),
        P::Visibility::Friend(loc) => E::Visibility::Friend(loc),
        P::Visibility::Internal => E::Visibility::Internal,
    }
}

fn function_signature(
    context: &mut Context,
    psignature: P::FunctionSignature,
) -> (OldAliasMap, E::FunctionSignature) {
    let P::FunctionSignature {
        type_parameters: pty_params,
        parameters: pparams,
        return_type: pret_ty,
    } = psignature;
    let type_parameters = type_parameters(context, pty_params);
    let old_aliases = context
        .aliases
        .shadow_for_type_parameters(type_parameters.iter().map(|(name, _)| name));
    let parameters = pparams
        .into_iter()
        .map(|(v, t)| (v, type_(context, t)))
        .collect::<Vec<_>>();
    for (v, _) in &parameters {
        check_valid_local_name(context, v)
    }
    let return_type = type_(context, pret_ty);
    let signature = E::FunctionSignature {
        type_parameters,
        parameters,
        return_type,
    };
    (old_aliases, signature)
}

fn function_body(context: &mut Context, sp!(loc, pbody_): P::FunctionBody) -> E::FunctionBody {
    use E::FunctionBody_ as EF;
    use P::FunctionBody_ as PF;
    let body_ = match pbody_ {
        PF::Native => EF::Native,
        PF::Defined(seq) => EF::Defined(sequence(context, loc, seq)),
    };
    sp(loc, body_)
}

//**************************************************************************************************
// Specification Blocks
//**************************************************************************************************

fn specs(context: &mut Context, pspecs: Vec<P::SpecBlock>) -> Vec<E::SpecBlock> {
    pspecs.into_iter().map(|s| spec(context, s)).collect()
}

fn spec(context: &mut Context, sp!(loc, pspec): P::SpecBlock) -> E::SpecBlock {
    let P::SpecBlock_ {
        attributes: pattributes,
        target,
        uses: puses,
        members: pmembers,
    } = pspec;

    let attributes = flatten_attributes(context, AttributePosition::Spec, pattributes);
    context.in_spec_context = true;
    let new_scope = uses(context, puses);
    let old_aliases = context.aliases.add_and_shadow_all(new_scope);

    let members = pmembers
        .into_iter()
        .map(|m| spec_member(context, m))
        .collect();

    context.set_to_outer_scope(old_aliases);
    context.in_spec_context = false;

    sp(loc, E::SpecBlock_ {
        attributes,
        target: spec_target(context, target),
        members,
    })
}

fn spec_target(context: &mut Context, sp!(loc, pt): P::SpecBlockTarget) -> E::SpecBlockTarget {
    use E::SpecBlockTarget_ as ET;
    use P::SpecBlockTarget_ as PT;
    let et = match pt {
        PT::Code => ET::Code,
        PT::Module => ET::Module,
        PT::Schema(name, type_params) => ET::Schema(name, type_parameters(context, type_params)),
        PT::Member(name, signature_opt) => ET::Member(
            name,
            signature_opt.map(|s| {
                let (old_aliases, signature) = function_signature(context, *s);
                context.set_to_outer_scope(old_aliases);
                Box::new(signature)
            }),
        ),
    };
    sp(loc, et)
}

fn spec_condition_kind(
    context: &mut Context,
    sp!(loc, kind): P::SpecConditionKind,
) -> (E::SpecConditionKind, Option<OldAliasMap>) {
    let (kind_, aliases_opt) = match kind {
        P::SpecConditionKind_::Assert => (E::SpecConditionKind_::Assert, None),
        P::SpecConditionKind_::Assume => (E::SpecConditionKind_::Assume, None),
        P::SpecConditionKind_::Decreases => (E::SpecConditionKind_::Decreases, None),
        P::SpecConditionKind_::AbortsIf => (E::SpecConditionKind_::AbortsIf, None),
        P::SpecConditionKind_::AbortsWith => (E::SpecConditionKind_::AbortsWith, None),
        P::SpecConditionKind_::SucceedsIf => (E::SpecConditionKind_::SucceedsIf, None),
        P::SpecConditionKind_::Modifies => (E::SpecConditionKind_::Modifies, None),
        P::SpecConditionKind_::Emits => (E::SpecConditionKind_::Emits, None),
        P::SpecConditionKind_::Ensures => (E::SpecConditionKind_::Ensures, None),
        P::SpecConditionKind_::Requires => (E::SpecConditionKind_::Requires, None),
        P::SpecConditionKind_::Invariant(pty_params) => {
            let ety_params = type_parameters(context, pty_params);
            let old_aliases = context
                .aliases
                .shadow_for_type_parameters(ety_params.iter().map(|(name, _)| name));
            (
                E::SpecConditionKind_::Invariant(ety_params),
                Some(old_aliases),
            )
        },
        P::SpecConditionKind_::InvariantUpdate(pty_params) => {
            let ety_params = type_parameters(context, pty_params);
            let old_aliases = context
                .aliases
                .shadow_for_type_parameters(ety_params.iter().map(|(name, _)| name));
            (
                E::SpecConditionKind_::InvariantUpdate(ety_params),
                Some(old_aliases),
            )
        },
        P::SpecConditionKind_::Axiom(pty_params) => {
            let ety_params = type_parameters(context, pty_params);
            let old_aliases = context
                .aliases
                .shadow_for_type_parameters(ety_params.iter().map(|(name, _)| name));
            (E::SpecConditionKind_::Axiom(ety_params), Some(old_aliases))
        },
    };
    (sp(loc, kind_), aliases_opt)
}

fn spec_member(context: &mut Context, sp!(loc, pm): P::SpecBlockMember) -> E::SpecBlockMember {
    use E::SpecBlockMember_ as EM;
    use P::SpecBlockMember_ as PM;
    let em = match pm {
        PM::Condition {
            kind: pkind,
            properties: pproperties,
            exp,
            additional_exps,
        } => {
            let (kind, old_aliases_opt) = spec_condition_kind(context, pkind);
            let properties = pproperties
                .into_iter()
                .map(|p| pragma_property(context, p))
                .collect();
            let exp = exp_(context, exp);
            let additional_exps = additional_exps
                .into_iter()
                .map(|e| exp_(context, e))
                .collect();
            match old_aliases_opt {
                None => (),
                Some(old_aliases) => context.set_to_outer_scope(old_aliases),
            }
            EM::Condition {
                kind,
                properties,
                exp,
                additional_exps,
            }
        },
        PM::Function {
            name,
            uninterpreted,
            signature,
            body,
        } => {
            let (old_aliases, signature) = function_signature(context, signature);
            let body = function_body(context, body);
            context.set_to_outer_scope(old_aliases);
            EM::Function {
                uninterpreted,
                name,
                signature,
                body,
            }
        },
        PM::Variable {
            is_global,
            name,
            type_parameters: pty_params,
            type_: t,
            init,
        } => {
            let type_parameters = type_parameters(context, pty_params);
            let old_aliases = context
                .aliases
                .shadow_for_type_parameters(type_parameters.iter().map(|(name, _)| name));
            let t = type_(context, t);
            let i = init.map(|e| exp_(context, e));
            context.set_to_outer_scope(old_aliases);
            EM::Variable {
                is_global,
                name,
                type_parameters,
                type_: t,
                init: i,
            }
        },
        PM::Update { lhs, rhs } => {
            let lhs = exp_(context, lhs);
            let rhs = exp_(context, rhs);
            EM::Update { lhs, rhs }
        },

        PM::Let {
            name,
            post_state: old,
            def: pdef,
        } => {
            let def = exp_(context, pdef);
            EM::Let {
                name,
                post_state: old,
                def,
            }
        },
        PM::Include {
            properties: pproperties,
            exp: pexp,
        } => {
            let properties = pproperties
                .into_iter()
                .map(|p| pragma_property(context, p))
                .collect();
            EM::Include {
                properties,
                exp: exp_(context, pexp),
            }
        },
        PM::Apply {
            exp: pexp,
            patterns,
            exclusion_patterns,
        } => EM::Apply {
            exp: exp_(context, pexp),
            patterns,
            exclusion_patterns,
        },
        PM::Pragma {
            properties: pproperties,
        } => {
            let properties = pproperties
                .into_iter()
                .map(|p| pragma_property(context, p))
                .collect();
            EM::Pragma { properties }
        },
    };
    sp(loc, em)
}

fn pragma_property(context: &mut Context, sp!(loc, pp_): P::PragmaProperty) -> E::PragmaProperty {
    let P::PragmaProperty_ {
        name,
        value: pv_opt,
    } = pp_;
    let value = pv_opt.and_then(|pv| pragma_value(context, pv));
    sp(loc, E::PragmaProperty_ { name, value })
}

fn pragma_value(context: &mut Context, pv: P::PragmaValue) -> Option<E::PragmaValue> {
    match pv {
        P::PragmaValue::Literal(v) => value(context, v).map(E::PragmaValue::Literal),
        P::PragmaValue::Ident(ma) => {
            name_access_chain(context, Access::Term, ma, None).map(E::PragmaValue::Ident)
        },
    }
}

//**************************************************************************************************
// Types
//**************************************************************************************************

fn ability_set(context: &mut Context, case: &str, abilities_vec: Vec<Ability>) -> E::AbilitySet {
    let mut set = E::AbilitySet::empty();
    for ability in abilities_vec {
        let loc = ability.loc;
        if let Err(prev_loc) = set.add(ability) {
            context.env.add_diag(diag!(
                Declarations::DuplicateItem,
                (loc, format!("Duplicate '{}' ability {}", ability, case)),
                (prev_loc, "Ability previously given here")
            ));
        }
    }
    set
}

fn type_parameters(
    context: &mut Context,
    pty_params: Vec<(Name, Vec<Ability>)>,
) -> Vec<(Name, E::AbilitySet)> {
    pty_params
        .into_iter()
        .map(|(name, constraints_vec)| {
            let constraints = ability_set(context, "constraint", constraints_vec);
            (name, constraints)
        })
        .collect()
}

fn struct_type_parameters(
    context: &mut Context,
    pty_params: Vec<P::StructTypeParameter>,
) -> Vec<E::StructTypeParameter> {
    pty_params
        .into_iter()
        .map(|param| E::StructTypeParameter {
            is_phantom: param.is_phantom,
            name: param.name,
            constraints: ability_set(context, "constraint", param.constraints),
        })
        .collect()
}

fn type_(context: &mut Context, sp!(loc, pt_): P::Type) -> E::Type {
    use E::Type_ as ET;
    use P::Type_ as PT;
    let t_ = match pt_ {
        PT::Unit => ET::Unit,
        PT::Multiple(ts) => ET::Multiple(types(context, ts)),
        PT::Apply(pn, ptyargs) => {
            let tyargs = types(context, ptyargs);
            match name_access_chain(context, Access::Type, *pn, Some(DeprecatedItem::Struct)) {
                None => {
                    assert!(context.env.has_errors());
                    ET::UnresolvedError
                },
                Some(n) => ET::Apply(n, tyargs),
            }
        },
        PT::Ref(mut_, inner) => ET::Ref(mut_, Box::new(type_(context, *inner))),
        PT::Fun(args, result, abilities_vec) => {
            let args = types(context, args);
            let result = type_(context, *result);
            let abilities = ability_set(context, "modifier", abilities_vec);
            ET::Fun(args, Box::new(result), abilities)
        },
    };
    sp(loc, t_)
}

fn types(context: &mut Context, pts: Vec<P::Type>) -> Vec<E::Type> {
    pts.into_iter().map(|pt| type_(context, pt)).collect()
}

fn optional_types(context: &mut Context, pts_opt: Option<Vec<P::Type>>) -> Option<Vec<E::Type>> {
    pts_opt.map(|pts| pts.into_iter().map(|pt| type_(context, pt)).collect())
}

#[derive(Clone, Copy)]
enum Access {
    Type,
    ApplyNamed,
    ApplyPositional,
    Term,
}

#[allow(dead_code)]
#[derive(Clone, Copy)]
enum DeprecatedMemberKind {}

fn name_access_chain(
    context: &mut Context,
    access: Access,
    sp!(loc, ptn_): P::NameAccessChain,
    deprecated_item_kind: Option<DeprecatedItem>,
) -> Option<E::ModuleAccess> {
    use E::ModuleAccess_ as EN;
    use P::{LeadingNameAccess_ as LN, NameAccessChain_ as PN};

    let tn_ = match (access, ptn_) {
        (Access::ApplyPositional, PN::One(n))
        | (Access::ApplyNamed, PN::One(n))
        | (Access::Type, PN::One(n)) => match context.aliases.member_alias_get(&n) {
            Some((mident, mem)) => EN::ModuleAccess(mident, mem, None),
            None => {
                // left unresolved
                EN::Name(n)
            },
        },
        (Access::Term, PN::One(n)) if is_valid_struct_constant_or_schema_name(n.value.as_str()) => {
            match context.aliases.member_alias_get(&n) {
                Some((mident, mem)) => EN::ModuleAccess(mident, mem, None),
                None => EN::Name(n),
            }
        },
        (Access::Term, PN::One(n)) => EN::Name(n),
        (_, PN::Two(sp!(nloc, LN::AnonymousAddress(_)), _)) => {
            context
                .env
                .add_diag(unexpected_address_module_error(loc, nloc, access));
            return None;
        },
        (_, PN::Two(sp!(_, LN::Name(n1)), n2)) => {
            if let Some((mident, mem)) = context.aliases.member_alias_get(&n1).filter(|_| {
                context.env.flags().lang_v2()
                    && is_valid_struct_constant_or_schema_name(n1.value.as_str())
                    && is_valid_struct_constant_or_schema_name(n2.value.as_str())
            }) {
                // n1 is interpreted as a type and n2 as a variant in the type
                EN::ModuleAccess(mident, mem, Some(n2))
            } else if let Some(mident) = context.aliases.module_alias_get(&n1) {
                // n1 is interpreted as a module and n2 as type.
                EN::ModuleAccess(mident, n2, None)
            } else {
                context.env.add_diag(diag!(
                    NameResolution::UnboundModule,
                    (n1.loc, format!("Unbound module or type alias '{}'", n1))
                ));
                return None;
            }
        },
        (_, PN::Three(sp!(ident_loc, (ln, n2)), n3)) => {
            let default_interpretation = |context: &mut Context| {
                let addr = address(context, /* suggest_declaration */ false, ln);
                let mident = sp(ident_loc, ModuleIdent_::new(addr, ModuleName(n2)));
                EN::ModuleAccess(mident, n3, None)
            };
            match &ln.value {
                LeadingNameAccess_::Name(n1)
                    if is_valid_struct_constant_or_schema_name(n2.value.as_str()) =>
                {
                    // Attempt to interpret n1 as module alias. This is for
                    // reaching struct variants as in `module::Struct::Variant`.
                    if let Some(mident) = context.aliases.module_alias_get(n1) {
                        EN::ModuleAccess(mident, n2, Some(n3))
                    } else {
                        default_interpretation(context)
                    }
                },
                _ => default_interpretation(context),
            }
        },
        (_, PN::Four(sp!(ident_loc, (ln, n2)), n3, n4)) => {
            let addr = address(context, /* suggest_declaration */ false, ln);
            let mident = sp(ident_loc, ModuleIdent_::new(addr, ModuleName(n2)));
            EN::ModuleAccess(mident, n3, Some(n4))
        },
    };

    if let Some(deprecated_item_kind) = deprecated_item_kind {
        match &tn_ {
            EN::ModuleAccess(mident, n, _) => {
                check_for_deprecated_member_use(context, Some(mident), n, deprecated_item_kind);
            },
            EN::Name(n) => {
                check_for_deprecated_member_use(context, None, n, deprecated_item_kind);
            },
        };
    };

    Some(sp(loc, tn_))
}

fn name_access_chain_to_module_ident(
    context: &mut Context,
    sp!(loc, pn_): P::NameAccessChain,
) -> Option<E::ModuleIdent> {
    use P::NameAccessChain_ as PN;
    match pn_ {
        PN::One(name) => match context.aliases.module_alias_get(&name) {
            None => {
                context.env.add_diag(diag!(
                    NameResolution::UnboundModule,
                    (name.loc, format!("Unbound module alias '{}'", name)),
                ));
                None
            },
            Some(mident) => {
                check_for_deprecated_module_use(context, &mident); // name
                Some(mident)
            },
        },
        PN::Two(ln, n) => {
            let pmident_ = P::ModuleIdent_ {
                address: ln,
                module: ModuleName(n),
            };
            Some(module_ident(context, sp(loc, pmident_)))
        },
        PN::Three(sp!(ident_loc, (ln, n)), mem) | PN::Four(sp!(ident_loc, (ln, n)), mem, _) => {
            // Process the module ident just for errors
            let pmident_ = P::ModuleIdent_ {
                address: ln,
                module: ModuleName(n),
            };
            let _ = module_ident(context, sp(ident_loc, pmident_));
            context.env.add_diag(diag!(
                NameResolution::NamePositionMismatch,
                (
                    mem.loc,
                    "Unexpected module member access. Expected a module identifier only",
                )
            ));
            None
        },
    }
}

fn unexpected_address_module_error(loc: Loc, nloc: Loc, access: Access) -> Diagnostic {
    let case = match access {
        Access::Type | Access::ApplyNamed | Access::ApplyPositional => "type",
        Access::Term => "expression",
    };
    let unexpected_msg = format!(
        "Unexpected module identifier. A module identifier is not a valid {}",
        case
    );
    diag!(
        NameResolution::NamePositionMismatch,
        (loc, unexpected_msg),
        (nloc, "Expected a module name".to_owned()),
    )
}

//**************************************************************************************************
// Expressions
//**************************************************************************************************

fn sequence(context: &mut Context, loc: Loc, seq: P::Sequence) -> E::Sequence {
    let (puses, pitems, maybe_last_semicolon_loc, pfinal_item) = seq;

    let new_scope = uses(context, puses);
    let old_aliases = context.aliases.add_and_shadow_all(new_scope);
    let mut items: VecDeque<E::SequenceItem> = pitems
        .into_iter()
        .map(|item| sequence_item(context, item))
        .collect();
    let final_e_opt = pfinal_item.map(|item| exp_(context, item));
    let final_e = match final_e_opt {
        None => {
            let last_semicolon_loc = match maybe_last_semicolon_loc {
                Some(l) => l,
                None => loc,
            };
            sp(last_semicolon_loc, E::Exp_::Unit { trailing: true })
        },
        Some(e) => e,
    };
    let final_item = sp(final_e.loc, E::SequenceItem_::Seq(final_e));
    items.push_back(final_item);
    context.set_to_outer_scope(old_aliases);
    items
}

fn sequence_item(context: &mut Context, sp!(loc, pitem_): P::SequenceItem) -> E::SequenceItem {
    use E::SequenceItem_ as ES;
    use P::SequenceItem_ as PS;
    let item_ = match pitem_ {
        PS::Seq(e) => ES::Seq(exp_(context, *e)),
        PS::Declare(pb, pty_opt) => {
            let b_opt = bind_list(context, pb);
            let ty_opt = pty_opt.map(|t| type_(context, t));
            match b_opt {
                None => {
                    assert!(context.env.has_errors());
                    ES::Seq(sp(loc, E::Exp_::UnresolvedError))
                },
                Some(b) => ES::Declare(b, ty_opt),
            }
        },
        PS::Bind(pb, pty_opt, pe) => {
            let b_opt = bind_list(context, pb);
            let ty_opt = pty_opt.map(|t| type_(context, t));
            let e_ = exp_(context, *pe);
            let e = match ty_opt {
                None => e_,
                Some(ty) => sp(e_.loc, E::Exp_::Annotate(Box::new(e_), ty)),
            };
            match b_opt {
                None => {
                    assert!(context.env.has_errors());
                    ES::Seq(sp(loc, E::Exp_::UnresolvedError))
                },
                Some(b) => ES::Bind(b, e),
            }
        },
    };
    sp(loc, item_)
}

fn exps(context: &mut Context, pes: Vec<P::Exp>) -> Vec<E::Exp> {
    pes.into_iter().map(|pe| exp_(context, pe)).collect()
}

fn exp(context: &mut Context, pe: P::Exp) -> Box<E::Exp> {
    Box::new(exp_(context, pe))
}

fn exp_(context: &mut Context, sp!(loc, pe_): P::Exp) -> E::Exp {
    use E::Exp_ as EE;
    use P::Exp_ as PE;
    let e_ = match pe_ {
        PE::Unit => EE::Unit { trailing: false },
        PE::Value(pv) => match value(context, pv) {
            Some(v) => EE::Value(v),
            None => {
                assert!(context.env.has_errors());
                EE::UnresolvedError
            },
        },
        PE::Move(v) => EE::Move(v),
        PE::Copy(v) => EE::Copy(v),
        PE::Name(_pn, Some(_ty)) if !context.in_spec_context && !context.env.flags().lang_v2() => {
            context.env.add_diag(diag!(
                Syntax::SpecContextRestricted,
                (
                    loc,
                    "Expected name to be followed by a brace-enclosed list of field expressions \
                     or a parenthesized list of arguments for a function call",
                )
            ));
            EE::UnresolvedError
        },
        PE::Name(pn, ptys_opt) => {
            let en_opt = name_access_chain(context, Access::Term, pn, Some(DeprecatedItem::Member));
            let tys_opt = optional_types(context, ptys_opt);
            match en_opt {
                Some(en) => EE::Name(en, tys_opt),
                None => {
                    assert!(context.env.has_errors());
                    EE::UnresolvedError
                },
            }
        },
        PE::Call(pn, kind, ptys_opt, sp!(rloc, prs)) => {
            let tys_opt = optional_types(context, ptys_opt);
            let ers = sp(rloc, exps(context, prs));
            let en_opt = if kind != CallKind::Receiver {
                name_access_chain(
                    context,
                    Access::ApplyPositional,
                    pn,
                    Some(DeprecatedItem::Function),
                )
            } else {
                // Skip resolution for receiver calls, which are expected to use a single name
                let P::NameAccessChain_::One(name) = pn.value else {
                    panic!("unexpected qualified name in receiver call")
                };
                Some(E::ModuleAccess::new(pn.loc, E::ModuleAccess_::Name(name)))
            };
            match en_opt {
                Some(en) => EE::Call(en, kind, tys_opt, ers),
                None => {
                    assert!(context.env.has_errors());
                    EE::UnresolvedError
                },
            }
        },
        PE::ExpCall(boxed_fexp, sp!(rloc, args)) => {
            let e_fexp = exp(context, *boxed_fexp);
            let e_args = sp(rloc, exps(context, args));
            EE::ExpCall(e_fexp, e_args)
        },
        PE::Pack(pn, ptys_opt, pfields) => {
            let en_opt = name_access_chain(
                context,
                Access::ApplyNamed,
                pn,
                Some(DeprecatedItem::Struct),
            );
            let tys_opt = optional_types(context, ptys_opt);
            let efields_vec = pfields
                .into_iter()
                .map(|(f, pe)| (f, exp_(context, pe)))
                .collect();
            let efields = fields(context, loc, "construction", "argument", efields_vec);
            match en_opt {
                Some(en) => EE::Pack(en, tys_opt, efields),
                None => {
                    assert!(context.env.has_errors());
                    EE::UnresolvedError
                },
            }
        },
        PE::Vector(vec_loc, ptys_opt, sp!(args_loc, pargs_)) => {
            let tys_opt = optional_types(context, ptys_opt);
            let args = sp(args_loc, exps(context, pargs_));
            EE::Vector(vec_loc, tys_opt, args)
        },
        PE::IfElse(pb, pt, pf_opt) => {
            let eb = exp(context, *pb);
            let et = exp(context, *pt);
            let ef = match pf_opt {
                None => Box::new(sp(loc, EE::Unit { trailing: false })),
                Some(pf) => exp(context, *pf),
            };
            EE::IfElse(eb, et, ef)
        },
        PE::Match(pd, parms) => {
            let discriminator = exp(context, *pd);
            let match_arms = parms
                .into_iter()
                .filter_map(|parm| {
                    let loc = parm.loc;
                    let (pbl, pc, pb) = parm.value;
                    if let Some(bind_list) = bind_list(context, pbl) {
                        let opt_cond = pc.map(|e| *exp(context, e));
                        let body = *exp(context, pb);
                        Some(sp(loc, (bind_list, opt_cond, body)))
                    } else {
                        if !context.env.has_errors() {
                            context.env.add_diag(diag!(
                                Syntax::InvalidLValue,
                                (loc, "bind list cannot be constructed")
                            ));
                        }
                        None
                    }
                })
                .collect::<Vec<_>>();
            EE::Match(discriminator, match_arms)
        },
        PE::While(label, pb, ploop) => EE::While(label, exp(context, *pb), exp(context, *ploop)),
        PE::Loop(label, ploop) => EE::Loop(label, exp(context, *ploop)),
        PE::Block(seq) => EE::Block(sequence(context, loc, seq)),
        PE::Lambda(pbs, pe, capture_kind, spec_opt) => {
            let tbs_opt = typed_bind_list(context, pbs);
            let e = exp_(context, *pe);
            let ee_spec = spec_opt.map(|spec| Box::new(exp_(context, *spec)));
            match tbs_opt {
                Some(tbs) => EE::Lambda(tbs, Box::new(e), capture_kind, ee_spec),
                None => {
                    assert!(context.env.has_errors());
                    EE::UnresolvedError
                },
            }
        },
        PE::Quant(k, prs, ptrs, pc, pe) => {
            if !context.in_spec_context {
                context.env.add_diag(diag!(
                    Syntax::SpecContextRestricted,
                    (loc, "quantifer expression only allowed in specifications")
                ));
                EE::UnresolvedError
            } else {
                let rs_opt = bind_with_range_list(context, prs);
                let rtrs = ptrs
                    .into_iter()
                    .map(|trs| trs.into_iter().map(|tr| exp_(context, tr)).collect())
                    .collect();
                let rc = pc.map(|c| Box::new(exp_(context, *c)));
                let re = exp_(context, *pe);
                match rs_opt {
                    Some(rs) => EE::Quant(k, rs, rtrs, rc, Box::new(re)),
                    None => {
                        assert!(context.env.has_errors());
                        EE::UnresolvedError
                    },
                }
            }
        },
        PE::ExpList(pes) => {
            assert!(pes.len() > 1);
            EE::ExpList(exps(context, pes))
        },

        PE::Assign(lvalue, op_opt, rhs) => {
            let l_opt = lvalues(context, *lvalue);
            let er = exp(context, *rhs);
            match l_opt {
                None => {
                    assert!(context.env.has_errors());
                    EE::UnresolvedError
                },
                Some(LValue::Assigns(al)) => match op_opt {
                    Some(op) => {
                        if al.value.len() == 1 {
                            match &al.value[0] {
                                // x += e (similarly for other binary operators)
                                // =>
                                // { let t = e; x = x + t; }
                                sp!(var_loc, LValue_::Var(module_access, ty_opt)) => {
                                    let x = sp(
                                        *var_loc,
                                        EE::Name(module_access.clone(), ty_opt.clone()),
                                    );
                                    // t, let t = e;
                                    let (t, bind) =
                                        let_symbol_eq_exp(er.loc, Symbol::from("$t"), *er);
                                    // x + t;
                                    let rhs_expanded =
                                        sp(loc, EE::BinopExp(Box::new(x), op, Box::new(t)));
                                    // x = x + t;
                                    let assign = sp(loc, EE::Assign(al, Box::new(rhs_expanded)));
                                    // { let t = e; x = x + t; }
                                    let sequence =
                                        VecDeque::from([bind, sp(loc, SequenceItem_::Seq(assign))]);
                                    EE::Block(sequence)
                                },
                                _ => {
                                    context.env.add_diag(diag!(Syntax::InvalidLValue, (loc, "Invalid assignment syntax. Expected: a local, a field write, or a deconstructing assignment")));
                                    EE::UnresolvedError
                                },
                            }
                        } else {
                            context.env.add_diag(diag!(Syntax::InvalidLValue, (loc, "Invalid assignment syntax. Expected: a local, a field write, or a deconstructing assignment")));
                            EE::UnresolvedError
                        }
                    },
                    None => EE::Assign(al, er),
                },
                Some(LValue::Mutate(el)) => {
                    match op_opt {
                        // *e1 += e2
                        // =>
                        // { let t2 = e2; let t1 = e1; *t1 = *t1 + t2 }
                        Some(op) => {
                            // t2, let t2 = e2;
                            let (tmp2, bind2) = let_symbol_eq_exp(er.loc, Symbol::from("$t2"), *er);
                            // t1, let t1 = e1;
                            let (tmp1, bind1) =
                                let_symbol_eq_exp(el.loc, Symbol::from("$t1"), match &el.value {
                                    EE::Index(..) => sp(el.loc, EE::Borrow(true, el)),
                                    _ => *el,
                                });
                            // *t1
                            let deref_tmp1 = sp(loc, EE::Dereference(Box::new(tmp1.clone())));
                            // *t1 + t2
                            let rhs_expanded =
                                sp(loc, EE::BinopExp(Box::new(deref_tmp1), op, Box::new(tmp2)));
                            // *t1 = *t1 + t2
                            let assign =
                                sp(loc, EE::Mutate(Box::new(tmp1), Box::new(rhs_expanded)));
                            // { let t2 = e2; let t1 = e1; *t1 = *t1 + t2 }
                            let sequence =
                                VecDeque::from([bind2, bind1, sp(loc, SequenceItem_::Seq(assign))]);
                            EE::Block(sequence)
                        },
                        None => EE::Mutate(el, er),
                    }
                },
                Some(LValue::FieldMutate(edotted)) => match op_opt {
                    // e1.f += e2
                    // =>
                    // { let t2 = e2; let t1 = &mut e1.f; *t1 = *t1 + t2 }
                    Some(op) => {
                        let lhs_loc = edotted.loc;
                        // t2, let t2 = e2;
                        let (tmp2, bind2) = let_symbol_eq_exp(er.loc, Symbol::from("$t2"), *er);
                        // e1.f
                        let e = sp(lhs_loc, EE::ExpDotted(edotted));
                        // &mut e1.f
                        let e_mut = sp(lhs_loc, EE::Borrow(true, Box::new(e)));
                        // t1, let t1 = &mut e1.f;
                        let (tmp1, bind1) = let_symbol_eq_exp(lhs_loc, Symbol::from("$t1"), e_mut);
                        // *t1
                        let deref_tmp1 = sp(loc, EE::Dereference(Box::new(tmp1.clone())));
                        // *t1 + t2
                        let rhs_expanded =
                            sp(loc, EE::BinopExp(Box::new(deref_tmp1), op, Box::new(tmp2)));
                        // *t1 = *t1 + t2
                        let assign = sp(loc, EE::Mutate(Box::new(tmp1), Box::new(rhs_expanded)));
                        // { let t2 = e2; let t1 = &mut e1.f; *t1 = *t1 + t2 }
                        let sequence =
                            VecDeque::from([bind2, bind1, sp(loc, SequenceItem_::Seq(assign))]);
                        EE::Block(sequence)
                    },
                    None => EE::FieldMutate(edotted, er),
                },
            }
        },
        PE::Return(pe_opt) => {
            let ev = match pe_opt {
                None => Box::new(sp(loc, EE::Unit { trailing: false })),
                Some(pe) => exp(context, *pe),
            };
            EE::Return(ev)
        },
        PE::Abort(pe) => EE::Abort(exp(context, *pe)),
        PE::Break(l) => EE::Break(l),
        PE::Continue(l) => EE::Continue(l),
        PE::Dereference(pe) => EE::Dereference(exp(context, *pe)),
        PE::UnaryExp(op, pe) => EE::UnaryExp(op, exp(context, *pe)),
        PE::BinopExp(pl, op, pr) => {
            if op.value.is_spec_only() && !context.in_spec_context {
                let msg = format!(
                    "`{}` operator only allowed in specifications",
                    op.value.symbol()
                );
                context
                    .env
                    .add_diag(diag!(Syntax::SpecContextRestricted, (loc, msg)));
                EE::UnresolvedError
            } else {
                EE::BinopExp(exp(context, *pl), op, exp(context, *pr))
            }
        },
        PE::Borrow(mut_, pr) => EE::Borrow(mut_, exp(context, *pr)),
        pdotted_ @ PE::Dot(_, _) => match exp_dotted(context, sp(loc, pdotted_)) {
            Some(edotted) => EE::ExpDotted(Box::new(edotted)),
            None => {
                assert!(context.env.has_errors());
                EE::UnresolvedError
            },
        },
        PE::Cast(e, ty) => EE::Cast(exp(context, *e), type_(context, ty)),
        PE::Test(e, tys) => EE::Test(
            exp(context, *e),
            tys.into_iter().map(|ty| type_(context, ty)).collect(),
        ),
        PE::Index(e, i) => {
            if context.env.flags().lang_v2() || context.in_spec_context {
                EE::Index(exp(context, *e), exp(context, *i))
            } else {
                // If it is a name, call `name_access_chain` to avoid
                // the unused alias warning
                if let PE::Name(pn, _) = e.value {
                    let _ = name_access_chain(context, Access::Term, pn, None);
                }
                context.env.add_diag(diag!(
                    Syntax::UnsupportedLanguageItem,
                    (
                        loc,
                        "`_[_]` index operator in non-specification code only allowed in Move 2 and beyond"
                    )
                ));
                EE::UnresolvedError
            }
        },
        PE::Annotate(e, ty) => EE::Annotate(exp(context, *e), type_(context, ty)),
        PE::Spec(_) if context.in_spec_context => {
            context.env.add_diag(diag!(
                Syntax::SpecContextRestricted,
                (loc, "'spec' blocks cannot be used inside of a spec context",)
            ));
            EE::UnresolvedError
        },
        PE::Spec(spec_block) => {
            let (spec_id, unbound_names) = context.bind_exp_spec(spec_block);
            let UnboundNames {
                vars: unbound_vars,
                func_ptrs: unbound_func_ptrs,
            } = unbound_names;
            EE::Spec(spec_id, unbound_vars, unbound_func_ptrs)
        },
        PE::UnresolvedError => panic!("ICE error should have been thrown"),
    };
    sp(loc, e_)
}

fn exp_dotted(context: &mut Context, sp!(loc, pdotted_): P::Exp) -> Option<E::ExpDotted> {
    use E::ExpDotted_ as EE;
    use P::Exp_ as PE;
    let edotted_ = match pdotted_ {
        PE::Dot(plhs, field) => {
            let lhs = exp_dotted(context, *plhs)?;
            EE::Dot(Box::new(lhs), field)
        },
        pe_ => EE::Exp(exp_(context, sp(loc, pe_))),
    };
    Some(sp(loc, edotted_))
}

fn value(context: &mut Context, sp!(loc, pvalue_): P::Value) -> Option<E::Value> {
    use E::Value_ as EV;
    use P::Value_ as PV;
    let value_ = match pvalue_ {
        PV::Address(addr) => {
            let addr = address(context, /* suggest_declaration */ true, addr);
            EV::Address(addr)
        },
        PV::Num(s) if s.ends_with("u8") => match parse_u8(&s[..s.len() - 2]) {
            Ok((u, _format)) => EV::U8(u),
            Err(_) => {
                context.env.add_diag(num_too_big_error(loc, "'u8'"));
                return None;
            },
        },
        PV::Num(s) if s.ends_with("u16") => match parse_u16(&s[..s.len() - 3]) {
            Ok((u, _format)) => EV::U16(u),
            Err(_) => {
                context.env.add_diag(num_too_big_error(loc, "'u16'"));
                return None;
            },
        },
        PV::Num(s) if s.ends_with("u32") => match parse_u32(&s[..s.len() - 3]) {
            Ok((u, _format)) => EV::U32(u),
            Err(_) => {
                context.env.add_diag(num_too_big_error(loc, "'u32'"));
                return None;
            },
        },
        PV::Num(s) if s.ends_with("u64") => match parse_u64(&s[..s.len() - 3]) {
            Ok((u, _format)) => EV::U64(u),
            Err(_) => {
                context.env.add_diag(num_too_big_error(loc, "'u64'"));
                return None;
            },
        },
        PV::Num(s) if s.ends_with("u128") => match parse_u128(&s[..s.len() - 4]) {
            Ok((u, _format)) => EV::U128(u),
            Err(_) => {
                context.env.add_diag(num_too_big_error(loc, "'u128'"));
                return None;
            },
        },
        PV::Num(s) if s.ends_with("u256") => match parse_u256(&s[..s.len() - 4]) {
            Ok((u, _format)) => EV::U256(u),
            Err(_) => {
                context.env.add_diag(num_too_big_error(loc, "'u256'"));
                return None;
            },
        },

        PV::Num(s) => match parse_u256(&s) {
            Ok((u, _format)) => EV::InferredNum(u),
            Err(_) => {
                context.env.add_diag(num_too_big_error(
                    loc,
                    "the largest possible integer type, 'u256'",
                ));
                return None;
            },
        },
        PV::Bool(b) => EV::Bool(b),
        PV::HexString(s) => match hex_string::decode(loc, &s) {
            Ok(v) => EV::Bytearray(v),
            Err(e) => {
                context.env.add_diag(*e);
                return None;
            },
        },
        PV::ByteString(s) => match byte_string::decode(loc, &s) {
            Ok(v) => EV::Bytearray(v),
            Err(e) => {
                context.env.add_diags(e);
                return None;
            },
        },
    };
    Some(sp(loc, value_))
}

// Create an error for an integer literal that is too big to fit in its type.
// This assumes that the literal is the current token.
fn num_too_big_error(loc: Loc, type_description: &'static str) -> Diagnostic {
    diag!(
        Syntax::InvalidNumber,
        (
            loc,
            format!(
                "Invalid number literal. The given literal is too large to fit into {}",
                type_description
            )
        ),
    )
}

//**************************************************************************************************
// Fields
//**************************************************************************************************

fn fields<T>(
    context: &mut Context,
    loc: Loc,
    case: &str,
    verb: &str,
    xs: Vec<(Field, T)>,
) -> Fields<T> {
    let mut fmap = UniqueMap::new();
    for (idx, (field, x)) in xs.into_iter().enumerate() {
        if let Err((field, old_loc)) = fmap.add(field, (idx, x)) {
            context.env.add_diag(diag!(
                Declarations::DuplicateItem,
                (loc, format!("Invalid {}", case)),
                (
                    field.loc(),
                    format!("Duplicate {} given for field '{}'", verb, field),
                ),
                (old_loc, "Field previously defined here".into()),
            ))
        }
    }
    fmap
}

//**************************************************************************************************
// LValues
//**************************************************************************************************

fn typed_bind_list(
    context: &mut Context,
    sp!(loc, pbs_): P::TypedBindList,
) -> Option<E::TypedLValueList> {
    let bs_: Option<Vec<E::TypedLValue>> = pbs_
        .into_iter()
        .map(|tpb| typed_bind(context, tpb))
        .collect();
    Some(sp(loc, bs_?))
}

fn typed_bind(context: &mut Context, sp!(loc, tpb_): P::TypedBind) -> Option<E::TypedLValue> {
    let P::TypedBind_(pb, opt_type) = tpb_;
    let b = bind(context, pb)?;
    let ot = opt_type.map(|ty| type_(context, ty));
    Some(sp(loc, E::TypedLValue_(b, ot)))
}

fn bind_list(context: &mut Context, sp!(loc, pbs_): P::BindList) -> Option<E::LValueList> {
    let bs_: Option<Vec<E::LValue>> = pbs_.into_iter().map(|pb| bind(context, pb)).collect();
    Some(sp(loc, bs_?))
}

fn bind_with_range_list(
    context: &mut Context,
    sp!(loc, prs_): P::BindWithRangeList,
) -> Option<E::LValueWithRangeList> {
    let rs_: Option<Vec<E::LValueWithRange>> = prs_
        .into_iter()
        .map(|sp!(loc, (pb, pr))| -> Option<E::LValueWithRange> {
            let r = exp_(context, pr);
            let b = bind(context, pb)?;
            Some(sp(loc, (b, r)))
        })
        .collect();
    Some(sp(loc, rs_?))
}

fn bind(context: &mut Context, sp!(loc, pb_): P::Bind) -> Option<E::LValue> {
    use E::LValue_ as EL;
    use P::Bind_ as PB;
    let b_ = match pb_ {
        PB::Var(v) => {
            if context.env.flags().lang_v2()
                && is_valid_struct_constant_or_schema_name(v.value().as_str())
            {
                // Interpret as an unqualified module access
                EL::Unpack(
                    sp(v.loc(), ModuleAccess_::Name(v.0)),
                    None,
                    Fields::new(),
                    None,
                )
            } else {
                check_valid_local_name(context, &v);
                EL::Var(sp(loc, E::ModuleAccess_::Name(v.0)), None)
            }
        },
        PB::Unpack(ptn, ptys_opt, pfields) => {
            // check for type use
            let tn = name_access_chain(
                context,
                Access::ApplyNamed,
                *ptn,
                Some(DeprecatedItem::Struct),
            )?;
            let tys_opt = optional_types(context, ptys_opt);
            let mut dotdot = None;
            let mut vfields = vec![];
            for (i, pfield) in pfields.iter().enumerate() {
                match &pfield.value {
                    P::BindFieldOrDotDot_::FieldBind(field, pbind) => {
                        let lval = bind(context, pbind.clone())?;
                        vfields.push((*field, lval));
                    },
                    P::BindFieldOrDotDot_::DotDot => {
                        if i != pfields.len() - 1 {
                            context.env.add_diag(diag!(
                                Syntax::UnexpectedToken,
                                (pfield.loc, "`..` must be at the end in a named field list")
                            ));
                        } else {
                            dotdot = Some(sp(pfield.loc, E::DotDot_));
                        }
                    },
                }
            }
            let fields = fields(context, loc, "deconstruction binding", "binding", vfields);
            EL::Unpack(tn, tys_opt, fields, dotdot)
        },
        PB::PositionalUnpack(ptn, ptys_opt, pargs) => {
            let tn = name_access_chain(
                context,
                Access::ApplyPositional,
                *ptn,
                Some(DeprecatedItem::Struct),
            )?;
            let tys_opt = optional_types(context, ptys_opt);
            let mut dot_seen = false;
            let fields: Option<Vec<E::LValueOrDotDot>> = pargs
                .into_iter()
                .map(|pb_or_dotdot| {
                    let sp!(loc, pb_or_dotdot_) = pb_or_dotdot;
                    match pb_or_dotdot_ {
                        P::BindOrDotDot_::Bind(pb) => {
                            bind(context, pb).map(|b| sp(b.loc, LValueOrDotDot_::LValue(b)))
                        },
                        P::BindOrDotDot_::DotDot => {
                            if dot_seen {
                                context.env.add_diag(diag!(
                                    Syntax::UnexpectedToken,
                                    (loc, "there can be at most one `..` per struct or variant pattern")
                                ));
                                None
                            } else {
                                dot_seen = true;
                                Some(sp(loc, LValueOrDotDot_::DotDot))
                            }
                        },
                    }
                })
                .collect();
            EL::PositionalUnpack(tn, tys_opt, Spanned::new(loc, fields?))
        },
    };
    Some(sp(loc, b_))
}

enum LValue {
    Assigns(E::LValueList),
    FieldMutate(Box<E::ExpDotted>),
    Mutate(Box<E::Exp>),
}

fn lvalues(context: &mut Context, sp!(loc, e_): P::Exp) -> Option<LValue> {
    use LValue as L;
    use P::Exp_ as PE;
    let al: LValue = match e_ {
        PE::Unit => L::Assigns(sp(loc, vec![])),
        PE::ExpList(pes) => {
            let al_opt: Option<E::LValueList_> =
                pes.into_iter().map(|pe| assign(context, pe)).collect();
            L::Assigns(sp(loc, al_opt?))
        },
        PE::Index(_, _) if context.env.flags().lang_v2() => {
            let er = exp(context, sp(loc, e_));
            L::Mutate(er)
        },
        PE::Dereference(pr) => {
            let er = exp(context, *pr);
            L::Mutate(er)
        },
        pdotted_ @ PE::Dot(_, _) => {
            let dotted = exp_dotted(context, sp(loc, pdotted_))?;
            L::FieldMutate(Box::new(dotted))
        },
        _ => L::Assigns(sp(loc, vec![assign(context, sp(loc, e_))?])),
    };
    Some(al)
}

fn assign(context: &mut Context, sp!(loc, e_): P::Exp) -> Option<E::LValue> {
    use E::LValue_ as EL;
    use P::Exp_ as PE;
    let a_ = match e_ {
        PE::Name(n @ sp!(_, P::NameAccessChain_::Two(_, _)), _)
        | PE::Name(n @ sp!(_, P::NameAccessChain_::Three(_, _)), _)
            if !context.in_spec_context =>
        {
            let msg = format!(
                "Unexpected assignment of module access without fields outside of a spec \
                 context.\nIf you are trying to unpack a struct, try adding fields, e.g. '{} {{}}'",
                n
            );
            context
                .env
                .add_diag(diag!(Syntax::SpecContextRestricted, (loc, msg)));

            // For unused alias warnings and unbound modules
            name_access_chain(context, Access::Term, n, None);

            return None;
        },
        PE::Name(sp!(_, P::NameAccessChain_::Four(_, _, _)), _) => {
            invalid_variant_access(context, loc);
            return None;
        },
        PE::Name(n, Some(_)) if !context.in_spec_context => {
            let msg = format!(
                "Unexpected assignment of instantiated type without fields outside of a spec \
                 context.\nIf you are trying to unpack a struct, try adding fields, e.g. '{} {{}}'",
                n
            );
            context
                .env
                .add_diag(diag!(Syntax::SpecContextRestricted, (loc, msg)));

            // For unused alias warnings and unbound modules
            name_access_chain(context, Access::Term, n, None);

            return None;
        },
        PE::Name(pn, ptys_opt) => {
            let en = name_access_chain(context, Access::Term, pn, Some(DeprecatedItem::Struct))?;
            match &en.value {
                E::ModuleAccess_::ModuleAccess(m, n, _) if !context.in_spec_context => {
                    let msg = format!(
                        "Unexpected assignment of module access without fields outside of a spec \
                         context.\nIf you are trying to unpack a struct, try adding fields, e.g. \
                         '{}::{} {{}}'",
                        m, n,
                    );
                    context
                        .env
                        .add_diag(diag!(Syntax::SpecContextRestricted, (loc, msg)));
                    return None;
                },
                _ => {
                    let tys_opt = optional_types(context, ptys_opt);
                    EL::Var(en, tys_opt)
                },
            }
        },
        PE::Pack(pn, ptys_opt, pfields) => {
            let en = name_access_chain(
                context,
                Access::ApplyNamed,
                pn,
                Some(DeprecatedItem::Struct),
            )?;
            let tys_opt = optional_types(context, ptys_opt);
            let efields = assign_unpack_fields(context, loc, pfields)?;
            // we have not implemented .. in the LHS of an assignment
            // and the `pfields` doesn't have a `..` in it, so we use `None` here
            EL::Unpack(en, tys_opt, efields, None)
        },
        _ => {
            context.env.add_diag(diag!(
                Syntax::InvalidLValue,
                (
                    loc,
                    "Invalid assignment syntax. Expected: a local, a field write, or a \
                     deconstructing assignment"
                )
            ));
            return None;
        },
    };
    Some(sp(loc, a_))
}

fn assign_unpack_fields(
    context: &mut Context,
    loc: Loc,
    pfields: Vec<(Field, P::Exp)>,
) -> Option<Fields<E::LValue>> {
    let afields = pfields
        .into_iter()
        .map(|(f, e)| Some((f, assign(context, e)?)))
        .collect::<Option<_>>()?;
    Some(fields(
        context,
        loc,
        "deconstructing assignment",
        "assignment binding",
        afields,
    ))
}

//**************************************************************************************************
// Unbound names
//**************************************************************************************************

#[derive(Default)]
struct UnboundNames {
    vars: BTreeSet<Name>,
    func_ptrs: BTreeSet<Name>,
}

fn unbound_names_spec_block(unbound: &mut UnboundNames, sp!(_, sb_): &E::SpecBlock) {
    sb_.members
        .iter()
        .for_each(|member| unbound_names_spec_block_member(unbound, member))
}

fn unbound_names_spec_block_member(unbound: &mut UnboundNames, sp!(_, m_): &E::SpecBlockMember) {
    use E::SpecBlockMember_ as M;
    match &m_ {
        M::Condition {
            exp,
            additional_exps,
            ..
        } => {
            unbound_names_exp(unbound, exp);
            additional_exps
                .iter()
                .for_each(|e| unbound_names_exp(unbound, e));
        },
        M::Update { rhs, .. } => {
            unbound_names_exp(unbound, rhs);
        },
        // No unbound names
        // And will error in the Move prover
        M::Function { .. }
        | M::Variable { .. }
        | M::Let { .. }
        | M::Include { .. }
        | M::Apply { .. }
        | M::Pragma { .. } => (),
    }
}

fn unbound_names_exp(unbound: &mut UnboundNames, sp!(_, e_): &E::Exp) {
    use E::Exp_ as EE;
    match e_ {
        EE::Value(_)
        | EE::Break(_)
        | EE::Continue(_)
        | EE::UnresolvedError
        | EE::Name(sp!(_, E::ModuleAccess_::ModuleAccess(..)), _)
        | EE::Unit { .. } => (),
        EE::Copy(v) | EE::Move(v) => {
            unbound.vars.insert(v.0);
        },
        EE::Name(sp!(_, E::ModuleAccess_::Name(n)), _) => {
            unbound.vars.insert(*n);
        },
        EE::Call(sp!(_, ma_), _, _, sp!(_, es_)) => {
            match ma_ {
                // capture the case of calling a lambda / function pointer
                // NOTE: this also captures calls to built-in move and spec functions
                // (e.g., `move_to` and `len`), which will be filtered out in later passes.
                E::ModuleAccess_::Name(n) => {
                    unbound.func_ptrs.insert(*n);
                },
                E::ModuleAccess_::ModuleAccess(..) => (),
            }
            unbound_names_exps(unbound, es_);
        },
        EE::ExpCall(fexp, sp!(_, es_)) => {
            unbound_names_exp(unbound, fexp);
            unbound_names_exps(unbound, es_);
        },
        EE::Vector(_, _, sp!(_, es_)) => unbound_names_exps(unbound, es_),
        EE::Pack(_, _, es) => unbound_names_exps(unbound, es.iter().map(|(_, _, (_, e))| e)),
        EE::IfElse(econd, et, ef) => {
            unbound_names_exp(unbound, ef);
            unbound_names_exp(unbound, et);
            unbound_names_exp(unbound, econd)
        },
        EE::Match(ed, arms) => {
            unbound_names_exp(unbound, ed);
            for arm in arms {
                unbound_names_binds(unbound, &arm.value.0);
                if let Some(c) = &arm.value.1 {
                    unbound_names_exp(unbound, c)
                }
                unbound_names_exp(unbound, &arm.value.2)
            }
        },
        EE::While(_, econd, eloop) => {
            unbound_names_exp(unbound, eloop);
            unbound_names_exp(unbound, econd)
        },
        EE::Loop(_, eloop) => unbound_names_exp(unbound, eloop),

        EE::Block(seq) => unbound_names_sequence(unbound, seq),
        EE::Lambda(ls, er, _capture_kind, spec_opt) => {
            unbound_names_exp(unbound, er);
            // remove anything in `ls`
            unbound_names_typed_binds(unbound, ls);
            if let Some(spec) = spec_opt {
                unbound_names_exp(unbound, spec);
            }
        },
        EE::Quant(_, rs, trs, cr_opt, er) => {
            unbound_names_exp(unbound, er);
            if let Some(cr) = cr_opt {
                unbound_names_exp(unbound, cr);
            }
            for tr in trs {
                unbound_names_exps(unbound, tr);
            }
            // remove anything in `rs`
            unbound_names_binds_with_range(unbound, rs);
        },
        EE::Assign(ls, er) => {
            unbound_names_exp(unbound, er);
            // remove anything in `ls`
            unbound_names_assigns(unbound, ls);
        },
        EE::Return(e)
        | EE::Abort(e)
        | EE::Dereference(e)
        | EE::UnaryExp(_, e)
        | EE::Borrow(_, e)
        | EE::Cast(e, _)
        | EE::Test(e, _)
        | EE::Annotate(e, _) => unbound_names_exp(unbound, e),
        EE::FieldMutate(ed, er) => {
            unbound_names_exp(unbound, er);
            unbound_names_dotted(unbound, ed)
        },
        EE::Mutate(el, er) | EE::BinopExp(el, _, er) => {
            unbound_names_exp(unbound, er);
            unbound_names_exp(unbound, el)
        },
        EE::ExpList(es) => unbound_names_exps(unbound, es),
        EE::ExpDotted(ed) => unbound_names_dotted(unbound, ed),
        EE::Index(el, ei) => {
            unbound_names_exp(unbound, ei);
            unbound_names_exp(unbound, el)
        },

        EE::Spec(_, unbound_vars, unbound_func_ptrs) => {
            unbound.vars.extend(unbound_vars);
            unbound.func_ptrs.extend(unbound_func_ptrs);
        },
    }
}

fn unbound_names_exps<'a>(unbound: &mut UnboundNames, es: impl IntoIterator<Item = &'a E::Exp>) {
    es.into_iter().for_each(|e| unbound_names_exp(unbound, e))
}

fn unbound_names_sequence(unbound: &mut UnboundNames, seq: &E::Sequence) {
    seq.iter()
        .rev()
        .for_each(|s| unbound_names_sequence_item(unbound, s))
}

fn unbound_names_sequence_item(unbound: &mut UnboundNames, sp!(_, es_): &E::SequenceItem) {
    use E::SequenceItem_ as ES;
    match es_ {
        ES::Seq(e) => unbound_names_exp(unbound, e),
        ES::Declare(ls, _) => unbound_names_binds(unbound, ls),
        ES::Bind(ls, er) => {
            unbound_names_exp(unbound, er);
            // remove anything in `ls`
            unbound_names_binds(unbound, ls);
        },
    }
}

fn unbound_names_binds(unbound: &mut UnboundNames, sp!(_, ls_): &E::LValueList) {
    ls_.iter()
        .rev()
        .for_each(|l| unbound_names_bind(unbound, l))
}

fn unbound_names_typed_binds(unbound: &mut UnboundNames, sp!(_, ls_): &E::TypedLValueList) {
    ls_.iter()
        .rev()
        .for_each(|sp!(_loc, E::TypedLValue_(l, _opt_ty))| unbound_names_bind(unbound, l))
}

fn unbound_names_binds_with_range(
    unbound: &mut UnboundNames,
    sp!(_, rs_): &E::LValueWithRangeList,
) {
    rs_.iter().rev().for_each(|sp!(_, (b, r))| {
        unbound_names_bind(unbound, b);
        unbound_names_exp(unbound, r)
    })
}

fn unbound_names_bind(unbound: &mut UnboundNames, sp!(_, l_): &E::LValue) {
    use E::LValue_ as EL;
    match l_ {
        EL::Var(sp!(_, E::ModuleAccess_::Name(n)), _) => {
            unbound.vars.remove(n);
            unbound.func_ptrs.remove(n);
        },
        EL::Var(sp!(_, E::ModuleAccess_::ModuleAccess(..)), _) => {
            // Qualified vars are not considered in unbound set.
        },
        EL::Unpack(_, _, efields, _hasdotdot) => efields
            .iter()
            .for_each(|(_, _, (_, l))| unbound_names_bind(unbound, l)),
        EL::PositionalUnpack(_, _, ls) => {
            let loc = ls.loc;
            let ls = ls
                .value
                .iter()
                .filter_map(|l| {
                    if let sp!(_, LValueOrDotDot_::LValue(l)) = l {
                        Some(l.clone())
                    } else {
                        None
                    }
                })
                .collect();
            unbound_names_binds(unbound, &sp(loc, ls))
        },
    }
}

fn unbound_names_assigns(unbound: &mut UnboundNames, sp!(_, ls_): &E::LValueList) {
    ls_.iter()
        .rev()
        .for_each(|l| unbound_names_assign(unbound, l))
}

fn unbound_names_assign(unbound: &mut UnboundNames, sp!(_, l_): &E::LValue) {
    use E::LValue_ as EL;
    match l_ {
        EL::Var(sp!(_, E::ModuleAccess_::Name(n)), _) => {
            unbound.vars.insert(*n);
        },
        EL::Var(sp!(_, E::ModuleAccess_::ModuleAccess(..)), _) => {
            // Qualified vars are not considered in unbound set.
        },
        EL::Unpack(_, _, efields, _) => efields
            .iter()
            .for_each(|(_, _, (_, l))| unbound_names_assign(unbound, l)),
        EL::PositionalUnpack(_, _, ls) => {
            let loc = ls.loc;
            let ls = ls
                .value
                .iter()
                .filter_map(|l| {
                    if let sp!(_, LValueOrDotDot_::LValue(l)) = l {
                        Some(l.clone())
                    } else {
                        None
                    }
                })
                .collect();
            unbound_names_assigns(unbound, &sp(loc, ls))
        },
    }
}

fn unbound_names_dotted(unbound: &mut UnboundNames, sp!(_, edot_): &E::ExpDotted) {
    use E::ExpDotted_ as ED;
    match edot_ {
        ED::Exp(e) => unbound_names_exp(unbound, e),
        ED::Dot(d, _) => unbound_names_dotted(unbound, d),
    }
}

//**************************************************************************************************
// Valid names
//**************************************************************************************************

fn check_valid_address_name_(
    env: &mut CompilationEnv,
    sp!(_, ln_): &P::LeadingNameAccess,
) -> Result<(), ()> {
    use P::LeadingNameAccess_ as LN;
    match ln_ {
        LN::AnonymousAddress(_) => Ok(()),
        LN::Name(n) => check_restricted_name_all_cases_(env, NameCase::Address, n),
    }
}

fn check_valid_local_name(context: &mut Context, v: &Var) {
    fn is_valid(s: Symbol) -> bool {
        s.starts_with('_') || s.starts_with(|c: char| c.is_ascii_lowercase())
    }
    if !is_valid(v.value()) {
        let msg = format!(
            "Invalid local variable name '{}'. Local variable names must start with 'a'..'z' (or \
             '_')",
            v,
        );
        context
            .env
            .add_diag(diag!(Declarations::InvalidName, (v.loc(), msg)));
    }
    let _ = check_restricted_name_all_cases(context, NameCase::Variable, &v.0);
}

#[derive(Copy, Clone, Debug)]
struct ModuleMemberInfo {
    pub kind: ModuleMemberKind,
    pub deprecation: Option<Loc>, // Some(loc) if member is deprecated at loc
}

#[derive(Copy, Clone, Debug)]
enum ModuleMemberKind {
    Constant,
    Function,
    Struct,
    Schema,
}

impl ModuleMemberKind {
    fn case(self) -> NameCase {
        match self {
            ModuleMemberKind::Constant => NameCase::Constant,
            ModuleMemberKind::Function => NameCase::Function,
            ModuleMemberKind::Struct => NameCase::Struct,
            ModuleMemberKind::Schema => NameCase::Schema,
        }
    }
}

#[derive(Copy, Clone, Debug)]
enum NameCase {
    Constant,
    Function,
    Struct,
    Schema,
    Module,
    ModuleMemberAlias(ModuleMemberKind),
    ModuleAlias,
    Variable,
    Address,
}

impl NameCase {
    const fn name(&self) -> &'static str {
        match self {
            NameCase::Constant => "constant",
            NameCase::Function => "function",
            NameCase::Struct => "struct",
            NameCase::Schema => "schema",
            NameCase::Module => "module",
            NameCase::ModuleMemberAlias(ModuleMemberKind::Function) => "function alias",
            NameCase::ModuleMemberAlias(ModuleMemberKind::Constant) => "constant alias",
            NameCase::ModuleMemberAlias(ModuleMemberKind::Struct) => "struct alias",
            NameCase::ModuleMemberAlias(ModuleMemberKind::Schema) => "schema alias",
            NameCase::ModuleAlias => "module alias",
            NameCase::Variable => "variable",
            NameCase::Address => "address",
        }
    }
}

fn check_valid_module_member_name(
    context: &mut Context,
    member: ModuleMemberKind,
    name: Name,
) -> Option<Name> {
    match check_valid_module_member_name_impl(context, member, &name, member.case()) {
        Err(()) => None,
        Ok(()) => Some(name),
    }
}

fn check_valid_module_member_alias(
    context: &mut Context,
    member: ModuleMemberKind,
    alias: Name,
) -> Option<Name> {
    match check_valid_module_member_name_impl(
        context,
        member,
        &alias,
        NameCase::ModuleMemberAlias(member),
    ) {
        Err(()) => None,
        Ok(()) => Some(alias),
    }
}

fn check_valid_module_member_name_impl(
    context: &mut Context,
    member: ModuleMemberKind,
    n: &Name,
    case: NameCase,
) -> Result<(), ()> {
    use ModuleMemberKind as M;
    fn upper_first_letter(s: &str) -> String {
        let mut chars = s.chars();
        match chars.next() {
            None => String::new(),
            Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
        }
    }
    match member {
        M::Function => {
            if n.value.starts_with(|c| c == '_') {
                let msg = format!(
                    "Invalid {} name '{}'. {} names cannot start with '_'",
                    case.name(),
                    n,
                    upper_first_letter(case.name()),
                );
                context
                    .env
                    .add_diag(diag!(Declarations::InvalidName, (n.loc, msg)));
                return Err(());
            }
        },
        M::Constant | M::Struct | M::Schema => {
            if !is_valid_struct_constant_or_schema_name(&n.value) {
                let msg = format!(
                    "Invalid {} name '{}'. {} names must start with 'A'..'Z'",
                    case.name(),
                    n,
                    upper_first_letter(case.name()),
                );
                context
                    .env
                    .add_diag(diag!(Declarations::InvalidName, (n.loc, msg)));
                return Err(());
            }
        },
    }

    // TODO move these names to a more central place?
    check_restricted_names(context, case, n, builtins::all_function_names())?;
    check_restricted_names(context, case, n, builtins::all_type_names())?;

    // Restricting Self for now in the case where we ever have impls
    // Otherwise, we could allow it
    check_restricted_name_all_cases(context, case, n)?;

    Ok(())
}

pub fn is_valid_struct_constant_or_schema_name(s: &str) -> bool {
    s.starts_with(|c: char| c.is_ascii_uppercase())
}

// Checks for a restricted name in any decl case
// Self and vector are not allowed
fn check_restricted_name_all_cases(
    context: &mut Context,
    case: NameCase,
    n: &Name,
) -> Result<(), ()> {
    check_restricted_name_all_cases_(context.env, case, n)
}

fn check_restricted_name_all_cases_(
    env: &mut CompilationEnv,
    case: NameCase,
    n: &Name,
) -> Result<(), ()> {
    let n_str = n.value.as_str();
    let can_be_vector = matches!(case, NameCase::Module | NameCase::ModuleAlias);
    if n_str == ModuleName::SELF_NAME || (!can_be_vector && n_str == builtins::VECTOR) {
        env.add_diag(restricted_name_error(case, n.loc, n_str));
        Err(())
    } else {
        Ok(())
    }
}

fn check_restricted_names(
    context: &mut Context,
    case: NameCase,
    sp!(loc, n_): &Name,
    all_names: &BTreeSet<Symbol>,
) -> Result<(), ()> {
    if all_names.contains(n_) {
        context.env.add_diag(restricted_name_error(case, *loc, n_));
        Err(())
    } else {
        Ok(())
    }
}

fn restricted_name_error(case: NameCase, loc: Loc, restricted: &str) -> Diagnostic {
    let a_or_an = match case.name().chars().next().unwrap() {
        // TODO this is not exhaustive to the indefinite article rules in English
        // but 'case' is never user generated, so it should be okay for a while/forever...
        'a' | 'e' | 'i' | 'o' | 'u' => "an",
        _ => "a",
    };
    let msg = format!(
        "Invalid {case} name '{restricted}'. '{restricted}' is restricted and cannot be used to \
         name {a_or_an} {case}",
        a_or_an = a_or_an,
        case = case.name(),
        restricted = restricted,
    );
    diag!(NameResolution::ReservedName, (loc, msg))
}

//**************************************************************************************************
// Utility functions
//**************************************************************************************************

/// Returns expansion expressions `(t, let t = e)` where `t` is a variable named `symbol`
fn let_symbol_eq_exp(loc: Loc, symbol: Symbol, e: E::Exp) -> (E::Exp, E::SequenceItem) {
    // t
    let tmp_name = sp(loc, symbol);
    let mod_acc = ModuleAccess_::Name(tmp_name);
    let tmp_ = E::Exp_::Name(sp(loc, mod_acc.clone()), None);
    let tmp = sp(loc, tmp_);
    // let t = e;
    let lval_ = LValue_::Var(sp(loc, mod_acc), None);
    let lval = sp(loc, lval_);
    let lvals = sp(loc, vec![lval]);
    let bind_ = SequenceItem_::Bind(lvals, e);
    let bind = sp(loc, bind_);
    (tmp, bind)
}
