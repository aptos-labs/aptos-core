---
name: prover
description: Information how to develop and extend the Move Prover
---

The Move Prover is a formal verification tool for Move smart contracts. It translates Move code and specifications to Boogie, which then uses SMT solvers (Z3 or CVC5) to verify correctness.

Code is at `third_party/move/move-prover`.

# Directory Structure

```
move-prover/
├── src/                          # Main prover driver
│   ├── cli.rs                   # CLI argument parsing
│   ├── lib.rs                   # Main entry points and orchestration
│   └── main.rs                  # CLI binary entry point
├── boogie-backend/              # Boogie translation
│   ├── src/
│   │   ├── bytecode_translator.rs   # Bytecode → Boogie
│   │   ├── spec_translator.rs       # Specs → Boogie
│   │   ├── boogie_wrapper.rs        # Boogie runner & output parsing
│   │   ├── boogie_helpers.rs        # Name generation & type mapping
│   │   ├── options.rs              # Boogie configuration
│   │   └── prelude/               # Boogie template files
│   └── Cargo.toml
├── bytecode-pipeline/           # Bytecode transformations
│   ├── src/
│   │   ├── pipeline_factory.rs        # Pipeline composition
│   │   ├── spec_instrumentation.rs    # Inject specs into bytecode
│   │   ├── spec_inference.rs          # Auto-infer specs via WP
│   │   ├── loop_analysis.rs           # Loop invariant handling
│   │   ├── verification_analysis.rs   # Verify scope & globals
│   │   ├── global_invariant_*.rs      # Global invariant handling
│   │   ├── mono_analysis.rs           # Monomorphization
│   │   └── options.rs                 # ProverOptions
│   └── Cargo.toml
├── tests/                       # Test suite
│   ├── sources/functional/      # Functional tests
│   └── sources/regression/      # Regression tests
└── doc/                         # Documentation
    ├── user/                    # User guide
    └── dev/                     # Developer docs
```

# End-to-End Architecture

```
Move Source Code
    ↓
[1] Move Compiler v2
    ├── Parses Move syntax
    ├── Type checking
    └── Produces GlobalEnv (semantic model)
    ↓
[2] Bytecode Pipeline
    ├── Creates FunctionTargetsHolder
    └── Runs transformation processors (20+)
    ↓
[3] Boogie Code Generation
    ├── Add prelude templates
    ├── Translate bytecode → Boogie
    └── Translate specs → Boogie
    ↓
[4] Boogie Verification
    ├── Call Boogie (z3/cvc5 backend)
    ├── Parse verification output
    └── Map errors back to source
    ↓
Verification Results
```

# Bytecode Pipeline

The pipeline transforms Move bytecode with semantic analysis and verification instrumentation.

## Key Processors (ordered)

1. `EliminateImmRefsProcessor` - Remove immutable references
2. `MutRefInstrumenter` - Handle mutable references
3. `ReachingDefProcessor` - Reaching definitions analysis
4. `LiveVarAnalysisProcessor` - Liveness analysis
5. `BorrowAnalysisProcessor` - Borrow checking
6. `MemoryInstrumentationProcessor` - Memory safety assertions
7. `CleanAndOptimizeProcessor` - Dead code elimination
8. `UsageProcessor` - Track variable usage
9. `VerificationAnalysisProcessor` - Determine what to verify
10. `LoopAnalysisProcessor` - Loop invariant instrumentation
11. `SpecInferenceProcessor` - Auto-infer specs via weakest precondition
12. `SpecInstrumentationProcessor` - Inject spec conditions as assertions
13. `GlobalInvariantAnalysisProcessor` - Analyze global invariant scope
14. `GlobalInvariantInstrumentationProcessor` - Add global invariant checks
15. `DataInvariantInstrumentationProcessor` - Data invariant checks
16. `MonoAnalysisProcessor` - Compute monomorphization instances

## Processor Interface

```rust
trait FunctionTargetProcessor {
    fn process(
        &self,
        targets: &mut FunctionTargetsHolder,
        func_env: &FunctionEnv,
        data: FunctionData,
        scc_opt: Option<&[FunctionEnv]>,
    ) -> FunctionData;

    fn dump_result(
        &self,
        f: &mut fmt::Formatter,
        env: &GlobalEnv,
        targets: &FunctionTargetsHolder,
    ) -> fmt::Result { Ok(()) }
}
```

## Adding a New Processor

1. Create `my_processor.rs` in `bytecode-pipeline/src/`
2. Implement `FunctionTargetProcessor` trait
3. Add `pub mod my_processor;` to `lib.rs`
4. Register in `pipeline_factory.rs`:
   ```rust
   pipeline.add_processor(MyProcessor::new());
   ```
5. Add test case in `tests/my_processor/` with `.move` and `.exp` files
6. Add to `testsuite.rs` pipeline match

# Boogie Backend

## Main Components

- **BoogieTranslator** (`bytecode_translator.rs`) - Main bytecode → Boogie translation
- **SpecTranslator** (`spec_translator.rs`) - Spec conditions → Boogie expressions
- **BoogieWrapper** (`boogie_wrapper.rs`) - Execute Boogie and parse output
- **BoogieHelpers** (`boogie_helpers.rs`) - Name generation and type mapping

## Prelude Templates

Located in `boogie-backend/src/prelude/`:
- `prelude.bpl` - Core Boogie definitions
- `native.bpl` - Native function stubs
- `vector-*-theory.bpl` - Vector encoding (5 variants)

## Move to Boogie Mapping

**Types:**
- Primitives (u8-u256, bool, address) → Boogie types
- Structs → Boogie datatypes
- References → dereferenced at function boundaries

**Memory:**
- Global state: `memory(ModuleAddress, ResourceType) → Value`

**Specs:**
- `requires` → Function preconditions
- `ensures` → Function postconditions
- `aborts_if` → Abort condition assertions
- `modifies` → Memory permission checks

# Key Concepts

## Monomorphization

The prover computes all type instantiations needed:
```rust
pub struct MonoInfo {
    pub structs: BTreeMap<QualifiedId<StructId>, BTreeSet<Vec<Type>>>,
    pub funs: BTreeMap<(QualifiedId<FunId>, FunctionVariant), BTreeSet<Vec<Type>>>,
    pub fun_infos: BTreeMap<Type, BTreeSet<ClosureInfo>>,
}
```

## Loop Handling

- Loop invariants required for verification
- `LoopAnalysisProcessor` converts loops to DAGs
- Base case: assert invariant before loop
- Induction: havoc modified vars, assume invariant

## Global Invariants

- Module-level assertions that must hold in stable states
- `GlobalInvariantAnalysisProcessor` determines scope
- `GlobalInvariantInstrumentationProcessor` adds checks

## Function Values

- Supported via closure infrastructure
- Behavioral predicates (`requires_of`, `ensures_of`, `aborts_of`)
- Tracked in `MonoInfo.fun_infos`

# Configuration

## ProverOptions (`bytecode-pipeline/src/options.rs`)

```rust
pub struct ProverOptions {
    pub generate_only: bool,
    pub verify_scope: VerificationScope,  // All, OnlyVerified, OnlyPublic
    pub skip_loop_analysis: bool,
    pub check_inconsistency: bool,
    pub auto_trace_level: AutoTraceLevel,
    pub dump_bytecode: bool,
}
```

## BoogieOptions (`boogie-backend/src/options.rs`)

- Boogie/Z3/CVC5 paths
- Vector theory selection
- Timeout per VC (40s default)
- Sharding for parallel verification
- Loop unrolling depth

# Coding

- Do always look into move-model helper functions before creating new functions on common data types like expressions.

# Debugging

- To run the Move Prover directly on test files from the command line, use alias `mvp <source.move>`
- In order to inspect generated Boogie, use `mvp --keep <source.move>`. The following output will be generated:
  - `output.bpl` with the Boogie narrowed to verify given function
  - `output.bpl.log` the model as returned by Boogie to the Move Prover. The prover prints error messages derived from this to console
- In order to inspect generated smtlib file and the z3 log for a *given function*, use `mvp --generate-smt --z3-trace=<function> <source.move>`. The following output will be produced, assuming that source contains a function `<addr>::<module>::<function>`:
  - a file `_<addr>_<module>_<function>.smt` containing the smtlib input for Z3 as generated from the Boogie
  - a file `<function>.z3log` containing Z3 log during verifying the function.

# Testing

## Running Tests

```bash
cargo test -p move-prover                    # Run all tests
UB=1 cargo test -p move-prover              # Update baselines
MVP_TEST_FLAGS="-T=20" cargo test           # Custom flags
```

## Test Organization

- `tests/sources/functional/` - Positive test cases
- `tests/sources/regression/` - Regression tests
- Baseline `.exp` files store expected output

## Test Directives (in Move source comments)

- `// flag: <flags>` - Additional CLI flags
- `// no_ci:` - Skip in CI
- `// exclude_for: <feature>` - Exclude for feature

## Important


- Do MUST NOT automatically try to fix verification failures
- You can fix Rust or Boogie compilation failures
- For verification failures consult me before proceeding

# Documentation

- User docs are at `third_party/move/move-prover/doc/user`
- Design docs are at `third_party/move/move-prover/doc/dev`
- A major paper TACAS'22 is in `third_party/move/move-prover/doc/paper21`
- A longer version (incomplete) is in `third_party/move/move-prover/doc/report`

# Extension Points

1. **Custom Processors:** Implement `FunctionTargetProcessor` trait
2. **Custom Pipeline:** Compose processors in `pipeline_factory.rs`
3. **Custom Templates:** Provide Boogie templates for native functions
4. **Custom Options:** Extend `ProverOptions` or `BoogieOptions`
