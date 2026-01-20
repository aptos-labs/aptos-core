# Move Prover: Direct SMTLib Backend Plan

## Executive Summary

**Difficulty: High to Very High**
**Estimated Effort: 6-12 months for a complete replacement, 2-4 months for a hybrid approach**

Replacing the Boogie backend with direct SMTLib generation is a significant undertaking. The current Boogie backend represents ~11,000 lines of sophisticated code that handles:
- Bytecode-to-verification-condition translation
- Specification language compilation
- Type system axiomatization
- Memory model encoding
- Counterexample extraction

A direct replacement would require re-implementing all of this while also solving new challenges that Boogie currently abstracts away.

---

## Current Architecture

```
Move Source Code
       ↓
Move Compiler (AST → Bytecode)
       ↓
Bytecode Pipeline (15+ analysis passes)
       ↓
┌──────────────────────────────────────┐
│  BOOGIE BACKEND (~11K lines)         │  ← Replace this
│  - bytecode_translator.rs (3,807)    │
│  - spec_translator.rs (2,259)        │
│  - boogie_wrapper.rs (1,832)         │
│  - boogie_helpers.rs (1,221)         │
│  - Prelude templates (~2,000)        │
└──────────────────────────────────────┘
       ↓
Boogie (.bpl file)
       ↓
Boogie Tool (VC generation)
       ↓
Z3/CVC5 SMT Solver
       ↓
Verification Result
```

---

## Why Replace Boogie?

### Potential Benefits
1. **Remove external dependency** - Boogie requires .NET runtime, complex installation
2. **Direct solver control** - Fine-grained tuning of SMT queries
3. **Solver-specific optimizations** - Use Z3-specific features directly
4. **Simpler toolchain** - One fewer tool in the pipeline
5. **Better error messages** - Direct mapping from SMT failures to source
6. **Custom theories** - Easier to add Move-specific SMT theories

### Current Pain Points with Boogie
- Installation complexity (requires Mono/.NET + Boogie + Z3)
- Limited control over verification condition structure
- Text-based error parsing is fragile
- Boogie's own timeout/resource management

---

## Key Challenges

### 1. Loss of Procedural Abstraction (HARD)

Boogie provides procedures with control flow, early abort, and structured verification conditions:

```boogie
procedure Add_u64(src1: int, src2: int) returns (dst: int) {
    if (src1 + src2 > MAX_U64) {
        call $ExecFailureAbort();  // Early exit with abort
        return;
    }
    dst := src1 + src2;
}
```

SMT-LIB is pure logic - no procedures, no control flow. Must encode as:
```smt2
;; Need to model all paths and abort conditions explicitly
(declare-fun add_u64_result (Int Int) Int)
(declare-fun add_u64_aborts (Int Int) Bool)

(assert (forall ((s1 Int) (s2 Int))
  (= (add_u64_aborts s1 s2)
     (or (> (+ s1 s2) 18446744073709551615) (< (+ s1 s2) 0)))))
```

**Impact**: Every function needs explicit abort-path encoding.

### 2. Verification Condition Generation (HARD)

Boogie generates verification conditions using weakest-precondition calculus. Without Boogie, must implement:
- Weakest precondition transformer for all Move operations
- Loop invariant injection points
- Procedure inlining/modular verification
- Havoc semantics for `modifies` clauses

This is ~3,000-4,000 lines of core logic currently handled by Boogie itself.

### 3. Type System Axiomatization (MEDIUM-HARD)

Current prelude files define ~2,000 lines of axioms for:
- Integer types (u8, u16, u32, u64, u128, u256)
- Vectors (with 4 different theory encodings)
- Tables/Maps
- Resources and memory model
- Mutation/borrowing model

These would need direct SMT-LIB equivalents with careful trigger annotations.

### 4. Counterexample Extraction (MEDIUM)

Current `boogie_wrapper.rs` (1,832 lines) parses Boogie output to extract:
- Verification failure locations
- Execution traces
- Variable values at failure points
- Memory state

Would need new parser for Z3's S-expression output format.

### 5. Quantifier Trigger Engineering (HARD)

SMT solver performance heavily depends on quantifier triggers:
```boogie
// Boogie syntax - well-tuned triggers
axiom (forall v: Vec int, i: int :: {ReadVec(v, i)}
    0 <= i && i < LenVec(v) ==> InRange(ReadVec(v, i)));
```

Triggers must be carefully designed to:
- Avoid matching loops (solver hangs)
- Ensure sufficient instantiation (completeness)
- Balance performance and precision

---

## Proposed Approaches

### Option A: Full Replacement (High Risk, High Reward)

Replace entire Boogie backend with direct SMT-LIB generation.

**Components needed:**
1. **SMT-LIB code emitter** (~4,000 lines)
   - Type definitions and axioms
   - Memory model encoding
   - Arithmetic with overflow checking

2. **Verification condition generator** (~4,000 lines)
   - Weakest precondition transformer
   - Loop handling (unrolling or invariants)
   - Function inlining/summarization

3. **Specification translator** (~2,000 lines)
   - Quantified formulas with triggers
   - Specification functions
   - Invariant injection

4. **Result parser** (~1,500 lines)
   - Z3/CVC5 output parsing
   - Counterexample extraction
   - Source location mapping

**Total estimate**: ~12,000 lines of new code

**Timeline**: 6-12 months with 2-3 engineers

**Risks**:
- Subtle semantic differences causing unsound verification
- Performance regression on complex proofs
- Loss of years of Boogie optimization tuning

---

### Option B: Boogie-to-SMT Compiler (Medium Risk, Medium Reward)

Keep existing Boogie backend but add a Boogie → SMT-LIB compiler.

```
Bytecode Pipeline
       ↓
Boogie Backend (existing)
       ↓
.bpl file
       ↓
┌────────────────────────────┐
│ NEW: Boogie-to-SMT Compiler│  ← Add this
└────────────────────────────┘
       ↓
.smt2 file
       ↓
Z3/CVC5 directly
```

**Components needed:**
1. **Boogie parser** (~2,000 lines)
   - Parse Boogie syntax into AST
   - Handle procedures, types, axioms

2. **VC generator** (~2,500 lines)
   - Implement Boogie semantics
   - Weakest precondition for Boogie commands

3. **SMT-LIB emitter** (~1,500 lines)
   - Convert Boogie AST to SMT-LIB

4. **Result parser** (~1,500 lines)
   - Same as Option A

**Total estimate**: ~7,500 lines of new code

**Timeline**: 2-4 months with 1-2 engineers

**Benefits**:
- Preserves battle-tested translation logic
- Lower risk of semantic bugs
- Incremental migration possible

---

### Option C: Hybrid/Experimental Backend (Low Risk)

Add SMT-LIB as alternative backend, keep Boogie as default.

**Use cases for direct SMT**:
- Specific hard verification problems
- Research on solver strategies
- Debugging verification failures

**Timeline**: 1-2 months for basic implementation

---

## Recommended Approach

**Start with Option B (Boogie-to-SMT Compiler)** because:

1. **Preserves investment** - 11K lines of proven translation code
2. **Lower risk** - Boogie semantics well-documented
3. **Incremental** - Can compare Boogie vs direct SMT results
4. **Reversible** - Can fall back to Boogie tool

Then optionally migrate to Option A if direct SMT proves superior.

---

## Detailed Implementation Plan

### Phase 1: Foundation (4-6 weeks)

#### 1.1 Boogie Parser
- Parse Boogie syntax (.bpl files)
- Build AST representation
- Handle: types, constants, functions, axioms, procedures

**Key files to reference:**
- `boogie-backend/src/prelude/*.bpl` - Input examples
- Boogie grammar: https://github.com/boogie-org/boogie/blob/master/Source/Core/BoogieLang.atg

#### 1.2 SMT-LIB Emitter Core
Create base infrastructure for SMT-LIB output:

```rust
pub struct SmtEmitter {
    writer: CodeWriter,
    type_cache: HashMap<Type, String>,
}

impl SmtEmitter {
    fn emit_sort(&mut self, ty: &BoogieType) -> String;
    fn emit_function(&mut self, fun: &BoogieFunction);
    fn emit_axiom(&mut self, ax: &BoogieAxiom);
}
```

**Output example:**
```smt2
; Type declarations
(declare-sort Vec 1)
(declare-sort $Mutation 1)

; Function declarations
(declare-fun LenVec ((Vec Int)) Int)
(declare-fun ReadVec ((Vec Int) Int) Int)

; Axioms
(assert (forall ((v (Vec Int)))
  (>= (LenVec v) 0)))
```

### Phase 2: Verification Condition Generation (4-6 weeks)

#### 2.1 Boogie Command Semantics

Implement weakest-precondition transformer:

```rust
fn wp(&self, cmd: &BoogieCmd, post: &Expr) -> Expr {
    match cmd {
        Assign(x, e) => post.substitute(x, e),
        Assume(p) => Expr::Implies(p, post),
        Assert(p) => Expr::And(p, post),
        Havoc(vars) => post.forall(vars),  // Universal quantification
        Call { .. } => self.wp_call(cmd, post),
        If { cond, then, else_ } => {
            Expr::And(
                Expr::Implies(cond, self.wp_block(then, post)),
                Expr::Implies(Expr::Not(cond), self.wp_block(else_, post))
            )
        }
        While { inv, body } => {
            // Loop invariant handling
            self.wp_loop(inv, body, post)
        }
    }
}
```

#### 2.2 Procedure Handling

For each Boogie procedure, generate SMT query:
```smt2
; Procedure preconditions
(assert preconditions)

; VC for procedure body
(assert (not (=> preconditions (wp_body postconditions))))

; Check satisfiability
(check-sat)
(get-model)  ; If sat, extract counterexample
```

### Phase 3: Prelude Translation (2-3 weeks)

Convert Boogie prelude files to SMT-LIB:

| Boogie Prelude | SMT-LIB Output |
|----------------|----------------|
| `prelude.bpl` | `prelude.smt2` |
| `vector-smt-seq-theory.bpl` | `vector.smt2` |
| `multiset-array-theory.bpl` | `multiset.smt2` |
| `table-array-theory.bpl` | `table.smt2` |

### Phase 4: Solver Integration (2-3 weeks)

#### 4.1 Solver Invocation
```rust
pub struct SmtSolver {
    z3_path: PathBuf,
    timeout: Duration,
}

impl SmtSolver {
    fn solve(&self, query: &SmtQuery) -> SolverResult {
        // Write .smt2 file
        // Invoke z3 with appropriate flags
        // Parse output
    }
}
```

#### 4.2 Counterexample Extraction
Parse Z3's S-expression output:
```rust
fn parse_model(&self, output: &str) -> Model {
    let sexpr = parse_sexpression(output);
    let mut model = Model::new();

    for define in sexpr.find_all("define-fun") {
        let name = define.get(0);
        let value = self.interpret_value(define.get(4));
        model.insert(name, value);
    }
    model
}
```

### Phase 5: Integration & Testing (3-4 weeks)

#### 5.1 Backend Selection
Add configuration option:
```rust
pub enum ProverBackend {
    Boogie,        // Existing (default)
    DirectSmt,     // New backend
    Hybrid,        // Try SMT, fallback to Boogie
}
```

#### 5.2 Regression Testing
- Run entire Move Prover test suite with both backends
- Compare verification results
- Benchmark performance

#### 5.3 Error Message Mapping
Map SMT errors back to source locations using existing infrastructure.

---

## Risk Mitigation

### Semantic Correctness
- Formal specification of Boogie → SMT translation
- Extensive differential testing vs Boogie
- Property-based testing of VC generator

### Performance
- Profile solver time on existing benchmarks
- Tune trigger strategies
- Consider incremental solving

### Compatibility
- Support both Z3 and CVC5
- Abstract solver-specific features
- Graceful degradation

---

## Files to Create/Modify

### New Files (~7,500 lines)
```
move-prover/smt-backend/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── boogie_parser.rs      # Parse .bpl files
│   ├── boogie_ast.rs         # Boogie AST types
│   ├── vc_generator.rs       # Weakest precondition
│   ├── smt_emitter.rs        # SMT-LIB output
│   ├── solver.rs             # Z3/CVC5 integration
│   ├── model_parser.rs       # Counterexample extraction
│   └── prelude/
│       ├── core.smt2
│       ├── integers.smt2
│       ├── vectors.smt2
│       └── memory.smt2
└── tests/
    ├── parser_tests.rs
    ├── vc_tests.rs
    └── integration_tests.rs
```

### Modified Files
```
move-prover/src/lib.rs         # Add backend selection
move-prover/src/cli.rs         # Add --smt-backend flag
move-prover/Cargo.toml         # Add smt-backend dependency
```

---

## Success Criteria

1. **Correctness**: 100% agreement with Boogie on existing test suite
2. **Performance**: Within 2x of Boogie on standard benchmarks
3. **Error Quality**: Source-mapped errors with execution traces
4. **Reliability**: No crashes on valid Move code

---

## Conclusion

Replacing the Boogie backend is feasible but represents a significant engineering effort. The recommended approach is:

1. **Short-term (2-4 months)**: Implement Boogie-to-SMT compiler (Option B)
2. **Medium-term (6-12 months)**: Optionally migrate to direct bytecode-to-SMT (Option A) if benefits materialize
3. **Ongoing**: Maintain hybrid capability for debugging and research

The main risk is subtle semantic bugs in the new VC generator. Extensive differential testing against the existing Boogie backend is essential.
