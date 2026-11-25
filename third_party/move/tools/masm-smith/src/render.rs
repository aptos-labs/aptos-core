// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Prints a test as `.masm` text that `move-asm` and the transactional test runner accept.

use crate::ir::{
    Block, BlockId, Field, Function, InitDirective, Instr, Local, MasmTest, Module, Operand,
    RunCall, RuntimeArg, Script, Step, StructDecl, StructLayout, Type, TypeParam, Value, Variant,
};
use move_binary_format::file_format::Visibility;
use move_core_types::account_address::AccountAddress;
use std::{collections::BTreeMap, fmt::Write};

const INDENT: &str = "  ";
const STRING_WRITE: &str = "writing to a String cannot fail";

impl MasmTest {
    pub fn render(&self) -> String {
        let mut buf = String::new();
        if let Some(init) = &self.init {
            render_init(&mut buf, init);
        }
        for step in &self.steps {
            render_step(&mut buf, step);
        }
        buf
    }
}

fn render_init(buf: &mut String, init: &InitDirective) {
    if init.addresses.is_empty() {
        writeln!(buf, "//# init").expect(STRING_WRITE);
    } else {
        let parts: Vec<String> = init
            .addresses
            .iter()
            .map(|(name, addr)| format!("{}={}", name, render_address(addr)))
            .collect();
        writeln!(buf, "//# init --addresses {}", parts.join(" ")).expect(STRING_WRITE);
    }
    // Separate `init` from the next task directive with the blank line the task parser requires.
    writeln!(buf).expect(STRING_WRITE);
}

fn render_step(buf: &mut String, step: &Step) {
    match step {
        Step::Publish(module) => {
            writeln!(buf, "//# publish").expect(STRING_WRITE);
            render_module(buf, module);
            writeln!(buf).expect(STRING_WRITE);
        },
        Step::RunScript(script) => {
            write!(buf, "//# run").expect(STRING_WRITE);
            if !script.signers.is_empty() {
                write!(buf, " --signers").expect(STRING_WRITE);
                for signer in &script.signers {
                    write!(buf, " {}", render_address(signer)).expect(STRING_WRITE);
                }
            }
            writeln!(buf).expect(STRING_WRITE);
            render_script(buf, script);
            writeln!(buf).expect(STRING_WRITE);
        },
        Step::RunCall(call) => {
            render_run_call(buf, call);
        },
    }
}

// ============================================================================
// Module rendering
// ============================================================================

fn render_module(buf: &mut String, module: &Module) {
    writeln!(buf, "module {}", module.id.short_str_lossless()).expect(STRING_WRITE);
    for (alias, addr) in &module.address_aliases {
        writeln!(
            buf,
            "{}address {} = {}",
            INDENT,
            alias,
            render_address(addr)
        )
        .expect(STRING_WRITE);
    }
    for (alias, mid) in &module.module_aliases {
        writeln!(buf, "use {} as {}", mid.short_str_lossless(), alias).expect(STRING_WRITE);
    }
    for friend in &module.friends {
        writeln!(buf, "friend {}", friend.short_str_lossless()).expect(STRING_WRITE);
    }
    for s in &module.structs {
        render_struct(buf, s);
    }
    for f in &module.functions {
        render_function(buf, f);
    }
}

fn render_struct(buf: &mut String, s: &StructDecl) {
    let mut header = String::new();
    match &s.layout {
        StructLayout::Singleton(_) => write!(header, "struct {}", s.name).expect(STRING_WRITE),
        StructLayout::Variants(_) => write!(header, "enum {}", s.name).expect(STRING_WRITE),
    }
    if !s.type_params.is_empty() {
        write!(header, "<{}>", render_type_params(&s.type_params)).expect(STRING_WRITE);
    }
    if !s.abilities.is_empty() {
        write!(header, " has {}", s.abilities).expect(STRING_WRITE);
    }
    writeln!(buf, "{}", header).expect(STRING_WRITE);

    match &s.layout {
        StructLayout::Singleton(fields) => {
            for f in fields {
                render_field(buf, f, 1);
            }
        },
        StructLayout::Variants(variants) => {
            for v in variants {
                render_variant(buf, v);
            }
        },
    }
    writeln!(buf).expect(STRING_WRITE);
}

fn render_variant(buf: &mut String, v: &Variant) {
    writeln!(buf, "{}{}", INDENT, v.name).expect(STRING_WRITE);
    for f in &v.fields {
        render_field(buf, f, 2);
    }
}

fn render_field(buf: &mut String, f: &Field, depth: usize) {
    writeln!(
        buf,
        "{}{}: {}",
        INDENT.repeat(depth),
        f.name,
        render_type(&f.ty)
    )
    .expect(STRING_WRITE);
}

fn render_function(buf: &mut String, f: &Function) {
    let mut header = String::new();
    // `move-asm` accepts a single attribute group per function.
    if !f.attributes.is_empty() {
        let attrs: Vec<String> = f.attributes.iter().map(ToString::to_string).collect();
        write!(header, "#[{}] ", attrs.join(", ")).expect(STRING_WRITE);
    }
    if f.is_entry {
        write!(header, "entry ").expect(STRING_WRITE);
    }
    match f.visibility {
        Visibility::Public => write!(header, "public ").expect(STRING_WRITE),
        Visibility::Friend => write!(header, "friend ").expect(STRING_WRITE),
        Visibility::Private => {},
    }
    if f.is_native {
        write!(header, "native ").expect(STRING_WRITE);
    }
    write!(header, "fun {}", f.name).expect(STRING_WRITE);
    if !f.type_params.is_empty() {
        write!(header, "<{}>", render_type_params(&f.type_params)).expect(STRING_WRITE);
    }
    write!(header, "({})", render_locals_inline(&f.params)).expect(STRING_WRITE);
    if !f.return_types.is_empty() {
        write!(header, ": {}", render_tuple_type(&f.return_types)).expect(STRING_WRITE);
    }
    if !f.acquires.is_empty() {
        write!(
            header,
            " acquires {}",
            f.acquires
                .iter()
                .map(|i| i.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )
        .expect(STRING_WRITE);
    }
    writeln!(buf, "{}", header).expect(STRING_WRITE);

    for l in &f.locals {
        writeln!(buf, "{}local {}: {}", INDENT, l.name, render_type(&l.ty)).expect(STRING_WRITE);
    }

    if !f.is_native {
        let label_map = build_label_map(&f.body);
        for block in &f.body {
            render_block(buf, block, &label_map);
        }
    }
    writeln!(buf).expect(STRING_WRITE);
}

fn build_label_map(body: &[Block]) -> BTreeMap<BlockId, String> {
    // The entry block is the first block; it carries no label in `.masm` syntax — the function
    // header serves as the entry point. Subsequent blocks get sequential labels `l0`, `l1`, ...
    let mut map = BTreeMap::new();
    for (i, block) in body.iter().enumerate() {
        if i == 0 {
            continue;
        }
        map.insert(block.id, format!("l{}", i - 1));
    }
    map
}

fn render_block(buf: &mut String, block: &Block, labels: &BTreeMap<BlockId, String>) {
    let block_label = labels.get(&block.id);
    for (i, instr) in block.instrs.iter().enumerate() {
        if i == 0 {
            if let Some(label) = block_label {
                write!(buf, "{}: ", label).expect(STRING_WRITE);
            } else {
                buf.push_str(INDENT);
            }
        } else {
            buf.push_str(INDENT);
        }
        render_instr(buf, instr, labels);
        buf.push('\n');
    }
}

fn render_instr(buf: &mut String, instr: &Instr, labels: &BTreeMap<BlockId, String>) {
    buf.push_str(&instr.opcode);
    for (i, op) in instr.operands.iter().enumerate() {
        if i == 0 {
            buf.push(' ');
        } else {
            buf.push_str(", ");
        }
        render_operand(buf, op, labels);
    }
}

fn render_operand(buf: &mut String, op: &Operand, labels: &BTreeMap<BlockId, String>) {
    match op {
        Operand::Imm(v) => render_value(buf, v),
        Operand::Local(name) => buf.push_str(name.as_str()),
        Operand::Label(bid) => {
            // The first block has no label, regardless of its ID. Missing branch labels
            // indicate a generator bug: assert when debug assertions are enabled, otherwise
            // emit a name the assembler will reject instead of targeting another block.
            match labels.get(bid) {
                Some(label) => buf.push_str(label),
                None => {
                    debug_assert!(false, "render: branch target {:?} has no label", bid);
                    buf.push_str("__masm_smith_missing_label__");
                },
            }
        },
        Operand::Function(q, type_args) => {
            write!(buf, "{}::{}", q.module.short_str_lossless(), q.name).expect(STRING_WRITE);
            if !type_args.is_empty() {
                write!(buf, "<{}>", render_type_list(type_args)).expect(STRING_WRITE);
            }
        },
        Operand::Struct(q, type_args) => {
            // Struct-definition operands name structs in the current module, so render
            // their unqualified names.
            write!(buf, "{}", q.name).expect(STRING_WRITE);
            if !type_args.is_empty() {
                write!(buf, "<{}>", render_type_list(type_args)).expect(STRING_WRITE);
            }
        },
        Operand::Field {
            owner,
            type_args,
            field,
        } => {
            write!(buf, "{}", owner.name).expect(STRING_WRITE);
            if !type_args.is_empty() {
                write!(buf, "<{}>", render_type_list(type_args)).expect(STRING_WRITE);
            }
            write!(buf, ", {}", field).expect(STRING_WRITE);
        },
        Operand::VariantFields {
            owner,
            type_args,
            fields,
        } => {
            write!(buf, "{}", owner.name).expect(STRING_WRITE);
            if !type_args.is_empty() {
                write!(buf, "<{}>", render_type_list(type_args)).expect(STRING_WRITE);
            }
            for (variant, field) in fields {
                write!(buf, ", {}::{}", variant, field).expect(STRING_WRITE);
            }
        },
        Operand::Type(t) => {
            // `<T>` form — the `.masm` parser uses angle brackets to disambiguate type-only
            // arguments from value/identifier arguments (e.g. `vec_pack <u64>, 3`).
            write!(buf, "<{}>", render_type(t)).expect(STRING_WRITE);
        },
        Operand::ClosureMask(mask) => write!(buf, "{}", mask).expect(STRING_WRITE),
    }
}

fn render_value(buf: &mut String, v: &Value) {
    match v {
        Value::U8(x) => write!(buf, "{}", x).expect(STRING_WRITE),
        Value::U16(x) => write!(buf, "{}", x).expect(STRING_WRITE),
        Value::U32(x) => write!(buf, "{}", x).expect(STRING_WRITE),
        Value::U64(x) => write!(buf, "{}", x).expect(STRING_WRITE),
        Value::U128(x) => write!(buf, "{}", x).expect(STRING_WRITE),
        Value::U256(x) => write!(buf, "{}", x).expect(STRING_WRITE),
        Value::I8(x) => write!(buf, "{}", x).expect(STRING_WRITE),
        Value::I16(x) => write!(buf, "{}", x).expect(STRING_WRITE),
        Value::I32(x) => write!(buf, "{}", x).expect(STRING_WRITE),
        Value::I64(x) => write!(buf, "{}", x).expect(STRING_WRITE),
        Value::I128(x) => write!(buf, "{}", x).expect(STRING_WRITE),
        Value::I256(x) => write!(buf, "{}", x).expect(STRING_WRITE),
        // `move-asm`'s value parser (see `tools/move-asm/src/syntax.rs::value`) accepts
        // only numeric / bracketed-list forms — the identifier tokens `true`/`false` are
        // *not* accepted as values, even though they're keywords elsewhere. Bool constants
        // therefore render as `0`/`1`; the assembler's `AsmValue::to_move_value` decodes
        // these against `SignatureToken::Bool` correctly (`num != 0`).
        Value::Bool(b) => buf.push_str(if *b { "1" } else { "0" }),
        Value::Address(a) => buf.push_str(&render_address(a)),
        Value::Bytes(bs) => {
            // The move-asm parser accepts vector literals as `[N1, N2, ...]` (numeric
            // elements only — no `x"..."` hex syntax), so we render byte vectors using
            // the decimal-element form. This is what `ld_const vector<u8> [...]` expects.
            buf.push('[');
            for (i, b) in bs.iter().enumerate() {
                if i > 0 {
                    buf.push_str(", ");
                }
                write!(buf, "{}", b).expect(STRING_WRITE);
            }
            buf.push(']');
        },
        Value::Vector(items) => {
            buf.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    buf.push_str(", ");
                }
                render_value(buf, item);
            }
            buf.push(']');
        },
    }
}

// ============================================================================
// Script & RunCall
// ============================================================================

fn render_script(buf: &mut String, script: &Script) {
    writeln!(buf, "script").expect(STRING_WRITE);
    for (alias, mid) in &script.module_aliases {
        writeln!(buf, "use {} as {}", mid.short_str_lossless(), alias).expect(STRING_WRITE);
    }
    render_function(buf, &script.entry);
}

fn render_run_call(buf: &mut String, call: &RunCall) {
    write!(
        buf,
        "//# run {}::{}",
        call.target.module.short_str_lossless(),
        call.target.name
    )
    .expect(STRING_WRITE);
    if !call.signers.is_empty() {
        write!(buf, " --signers").expect(STRING_WRITE);
        for s in &call.signers {
            write!(buf, " {}", render_address(s)).expect(STRING_WRITE);
        }
    }
    if !call.type_args.is_empty() {
        write!(buf, " --type-args {}", render_type_list(&call.type_args)).expect(STRING_WRITE);
    }
    if !call.args.is_empty() {
        write!(buf, " --args").expect(STRING_WRITE);
        for a in &call.args {
            write!(buf, " {}", render_runtime_arg(a)).expect(STRING_WRITE);
        }
    }
    writeln!(buf).expect(STRING_WRITE);
    writeln!(buf).expect(STRING_WRITE);
}

fn render_runtime_arg(a: &RuntimeArg) -> String {
    match a {
        RuntimeArg::U8(x) => format!("{}u8", x),
        RuntimeArg::U16(x) => format!("{}u16", x),
        RuntimeArg::U32(x) => format!("{}u32", x),
        RuntimeArg::U64(x) => x.to_string(),
        RuntimeArg::U128(x) => format!("{}u128", x),
        RuntimeArg::U256(x) => format!("{}u256", x),
        RuntimeArg::Bool(b) => b.to_string(),
        RuntimeArg::Address(a) => render_address(a),
        RuntimeArg::Vector(items) => {
            let parts: Vec<String> = items.iter().map(render_runtime_arg).collect();
            format!("vector[{}]", parts.join(","))
        },
    }
}

// ============================================================================
// Type rendering
// ============================================================================

fn render_type(t: &Type) -> String {
    match t {
        Type::Bool => "bool".into(),
        Type::U8 => "u8".into(),
        Type::U16 => "u16".into(),
        Type::U32 => "u32".into(),
        Type::U64 => "u64".into(),
        Type::U128 => "u128".into(),
        Type::U256 => "u256".into(),
        Type::I8 => "i8".into(),
        Type::I16 => "i16".into(),
        Type::I32 => "i32".into(),
        Type::I64 => "i64".into(),
        Type::I128 => "i128".into(),
        Type::I256 => "i256".into(),
        Type::Address => "address".into(),
        Type::Signer => "signer".into(),
        Type::Vector(inner) => format!("vector<{}>", render_type(inner)),
        Type::Struct(qs, args) => {
            let mut s = format!("{}::{}", qs.module.short_str_lossless(), qs.name);
            if !args.is_empty() {
                s.push_str(&format!("<{}>", render_type_list(args)));
            }
            s
        },
        Type::Reference(is_mut, inner) => {
            if *is_mut {
                format!("&mut {}", render_type(inner))
            } else {
                format!("&{}", render_type(inner))
            }
        },
        // We render type parameters by their canonical declaration name, `T<idx>`.
        // Struct/function declarations introduce them under exactly this name, so the
        // assembler resolves the reference back to the correct parameter slot.
        Type::TypeParam(idx) => format!("T{}", idx),
        Type::Function {
            args,
            results,
            abilities,
        } => {
            let mut s = format!("|{}|", render_type_list(args));
            if !results.is_empty() {
                s.push_str(&format!(" {}", render_tuple_type(results)));
            }
            if !abilities.is_empty() {
                s.push_str(&format!(" has {}", abilities));
            }
            s
        },
    }
}

fn render_type_list(types: &[Type]) -> String {
    types.iter().map(render_type).collect::<Vec<_>>().join(", ")
}

fn render_tuple_type(types: &[Type]) -> String {
    if types.len() == 1 {
        render_type(&types[0])
    } else {
        format!("({})", render_type_list(types))
    }
}

fn render_type_params(params: &[TypeParam]) -> String {
    params
        .iter()
        .map(|p| {
            let mut s = String::new();
            if p.is_phantom {
                s.push_str("phantom ");
            }
            s.push_str(p.name.as_str());
            if !p.constraints.is_empty() {
                s.push_str(&format!(": {}", p.constraints));
            }
            s
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn render_locals_inline(locals: &[Local]) -> String {
    locals
        .iter()
        .map(|l| format!("{}: {}", l.name, render_type(&l.ty)))
        .collect::<Vec<_>>()
        .join(", ")
}

fn render_address(a: &AccountAddress) -> String {
    a.to_hex_literal()
}
