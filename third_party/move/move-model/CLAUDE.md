---
name: move model
description: How to work with Move model, stackless bytecode, etc.
---

The Move Model allows to navigate all dimensions of Move code: the source language, abstract syntax tree, and intermediate representation.

Code is at `third_party/move/move-model`.

# Directory Structure

```
move-model/
├── src/                          # Core model and AST
│   ├── ast.rs                    # Abstract syntax tree definitions
│   ├── model.rs                  # Core data structures (GlobalEnv, ModuleEnv, etc.)
│   ├── ty.rs                     # Type system definitions
│   ├── lib.rs                    # Entry points and model building
│   ├── builder/                  # Model construction modules
│   ├── exp_builder.rs            # Expression construction helpers
│   ├── exp_rewriter.rs           # Expression transformation visitor
│   ├── exp_generator.rs          # Expression generation utilities
│   ├── constant_folder.rs        # Constant folding optimization
│   ├── sourcifier.rs             # Convert AST back to source code
│   ├── spec_translator.rs        # Translate spec constructs
│   ├── pragmas.rs                # Specification pragmas
│   ├── intrinsics.rs             # Intrinsic type declarations
│   ├── symbol.rs                 # Symbol pool management
│   └── well_known.rs             # Well-known names and constants
├── bytecode/                     # Stackless bytecode and analysis
│   └── src/
│       ├── stackless_bytecode.rs # Bytecode and operation definitions
│       ├── stackless_bytecode_generator.rs # Generate from AST
│       ├── function_target.rs    # Function-level analysis context
│       ├── function_target_pipeline.rs # Transformation pipeline
│       ├── borrow_analysis.rs    # Borrow checking analysis
│       ├── livevar_analysis.rs   # Live variable analysis
│       ├── reaching_def_analysis.rs # Reaching definitions
│       ├── dataflow_analysis.rs  # Generic dataflow framework
│       └── dataflow_domains.rs   # Domain definitions
└── bytecode-test-utils/          # Testing utilities
```

# Environment Hierarchy

The model is organized as a nested hierarchy of environments:

```
GlobalEnv              // Root: contains all modules, files, diagnostics
  └── ModuleEnv        // A specific module in the global env
      ├── StructEnv    // A struct defined in the module
      └── FunctionEnv  // A function defined in the module
```

- **GlobalEnv** (`model.rs`) - Owns all model data: modules, source files, diagnostics, symbol pool. Central authority for type checking and error reporting.
- **ModuleEnv** - Reference wrapper around `ModuleData`. Methods: `get_structs()`, `get_functions()`, `get_spec_vars()`, `get_named_constants()`
- **StructEnv** - Reference wrapper around `StructData`. Methods: `get_fields()`, `get_variants()`, `get_abilities()`
- **FunctionEnv** - Reference wrapper around `FunctionData`. Methods: `get_parameters()`, `get_return_type()`, `get_spec()`, `get_bytecode()`

## Entity Identifiers

All major entities have unique identifier types:
- **ModuleId** - Index-based, always valid when obtained from `GlobalEnv`
- **StructId**, **FunId**, **FieldId** - Symbol-based, relative to parent
- **NodeId** - Unique ID for AST nodes, used to attach type/location info
- **QualifiedId<Id>** - ID qualified by a module: `(ModuleId, Id)`
- **QualifiedInstId<Id>** - Qualified ID with type instantiation

# Expression AST

Expressions use a two-level structure (`ast.rs`):

```rust
pub struct Exp(Rc<ExpData>);  // Immutable, shared, cheap to clone

pub enum ExpData {
    Invalid(NodeId),
    Value(NodeId, Value),
    LocalVar(NodeId, Symbol),
    Temporary(NodeId, TempIndex),
    Call(NodeId, Operation, Vec<Exp>),
    Invoke(NodeId, Exp, Vec<Exp>),
    Lambda(NodeId, Pattern, Exp, LambdaCaptureKind, Option<Exp>),
    Quant(NodeId, QuantKind, Vec<(Pattern, Exp)>, Triggers, Exp),
    Block(NodeId, Vec<(Pattern, Exp)>, Exp),
    IfElse(NodeId, Exp, Exp, Exp),
    // ... 40+ variants total
}
```

## Operations

The `Operation` enum covers all functions and operators:
- **Arithmetic**: Add, Sub, Mul, Div, Mod, Neg
- **Bitwise**: BitOr, BitAnd, Xor, Shl, Shr
- **Comparisons**: Eq, Neq, Lt, Le, Gt, Ge
- **Logic**: And, Or, Not, Implies, Iff
- **Spec-only**: Exists, Global, Old, Trace, TypeDomain
- **Structural**: Pack, Select, Tuple, UpdateField
- **Memory**: BorrowGlobal, Deref, MoveTo, MoveFrom
- **Vector**: Vector, Index, Slice, Range, Len
- **Advanced**: Closure, Invoke, Behavior (requires_of, ensures_of, etc.)

## Expression Visitors

Use `ExpData::any()` to search expressions:
```rust
exp.as_ref().any(&mut |e| matches!(e, ExpData::Temporary(_, idx) if *idx >= n))
```

Use `ExpData::visit_pre_order()` or `visit_pre_post()` for traversal:
```rust
exp.as_ref().visit_pre_order(&mut |e| { /* process */ true });
```

Use `ExpRewriter` for transformations:
```rust
let mut rewriter = ExpRewriter::new(env, &mut |node_id, target| {
    match target {
        RewriteTarget::LocalVar(sym) => Some(replacement),
        _ => None,
    }
});
let rewritten = rewriter.rewrite_exp(exp);
```

# Type System

```rust
pub enum Type {
    Primitive(PrimitiveType),         // u8, u64, bool, etc.
    Tuple(Vec<Type>),
    Vector(Box<Type>),
    Struct(ModuleId, StructId, Vec<Type>),
    TypeParameter(u16),
    Fun(Box<Type>, Box<Type>, AbilitySet),
    Reference(ReferenceKind, Box<Type>),
    // Spec-only types
    TypeDomain(Box<Type>),
    ResourceDomain(ModuleId, StructId, Option<Vec<Type>>),
    // Internal
    Error, Var(u32),
}
```

**Abilities**: Copy, Drop, Key, Store

# Stackless Bytecode

Instead of stack-based, uses temporary variables and explicit control flow:

```rust
pub enum Bytecode {
    Assign(AttrId, TempIndex, TempIndex, AssignKind),
    Call(AttrId, Vec<TempIndex>, Operation, Vec<TempIndex>, Option<AbortAction>),
    Ret(AttrId, Vec<TempIndex>),
    Load(AttrId, TempIndex, Constant),
    Branch(AttrId, Label, Label, TempIndex),
    Jump(AttrId, Label),
    Label(AttrId, Label),
    Abort(AttrId, TempIndex, Option<TempIndex>),
    Prop(AttrId, PropKind, Exp),  // Assert/assume
    // ...
}
```

## FunctionTarget and Pipeline

- **FunctionTarget** - Wraps `FunctionEnv` with mutable bytecode/type info. Enables multiple "variants" (baseline, verification).
- **FunctionTargetsHolder** - Container for all `FunctionTarget`s indexed by qualified function ID and variant.
- **FunctionTargetPipeline** - Chain of `FunctionTargetProcessor`s that transform bytecode.

## Bytecode Analysis Framework

- **Borrow Analysis** - Tracks reference lifetime and ownership
- **Livevar Analysis** - Identifies live variables at each point
- **Reaching Definitions** - Data flow analysis
- **Dataflow Framework** - Generic forward/backward analysis with `TransferFunctions` trait

# Specifications

## Condition Kinds

```rust
pub enum ConditionKind {
    Requires, Ensures, AbortsIf, AbortsIfNot, AbortsWith,
    Modifies, Emits, Invariant, InvariantUpdate,
    LetPre(Symbol, Loc), LetPost(Symbol, Loc),
    Update, // ...
}
```

## Spec Structure

```rust
pub struct Spec {
    pub loc: Option<Loc>,
    pub conditions: Vec<Condition>,
    pub properties: PropertyBag,
    pub on_impl: BTreeMap<ConditionKind, Condition>,
}

pub struct Condition {
    pub loc: Loc,
    pub kind: ConditionKind,
    pub properties: PropertyBag,
    pub exp: Exp,
    pub additional_exps: Vec<Exp>,
}
```

# Common Patterns

## Iterating Over Modules and Functions

```rust
for module in env.get_modules() {
    if !module.is_target() { continue; }
    for func in module.get_functions() {
        if func.is_native() { continue; }
        // Process func
    }
}
```

## Getting Function Spec

```rust
let spec = func_env.get_spec();
for cond in &spec.conditions {
    if matches!(cond.kind, ConditionKind::Ensures) {
        // Handle ensures condition
    }
}
```

## Modifying Function Spec

```rust
let mut spec = func_env.get_mut_spec();
spec.conditions.push(Condition {
    loc: func_env.get_loc(),
    kind: ConditionKind::Ensures,
    properties: BTreeMap::new(),
    exp: my_exp,
    additional_exps: vec![],
});
```

## Building Expressions

```rust
let node_id = env.new_node(loc, result_type);
let exp = ExpData::Call(node_id, Operation::Eq, vec![left, right]).into_exp();
```

## Processing Bytecode

```rust
let target = targets.get_target(&func_env, &FunctionVariant::Baseline);
for (offset, bc) in target.get_bytecode().iter().enumerate() {
    match bc {
        Bytecode::Ret(_, vals) => { /* handle return */ },
        Bytecode::Call(_, dests, op, srcs, _) => { /* handle call */ },
        _ => {}
    }
}
```

# Coding

- Do always look into move-model helper functions before creating new functions on common data types like expressions.

# Testing

There are baseline tests in the `move-model` and the `move-stackless-bytecode`  in the `bytecode` subdirectory controlled by `testsuite.rs`.
