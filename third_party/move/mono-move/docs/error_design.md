# Error System

## 1. Motivation

The current MoveVM error model has accumulated several pain points:

1. **Error code names rarely communicate what went wrong.** Names like `UNKNOWN_INVARIANT_VIOLATION_ERROR`, `UNKNOWN_VERIFICATION_ERROR`, `DATA_FORMAT_ERROR`, or `VECTOR_OPERATION_ERROR` say very little on their own, which is why error sites have grown to attach free-form messages alongside the code. The name is decorative; the message is what readers actually consult.

2. **Adding a new code is enough boilerplate that people reuse existing ones.** Introducing a new `StatusCode` variant means touching the enum, picking a number in the right range, updating conversions to and from `ExecutionResult`, and potentially adjusting serialisation. New error sites tend to reuse a vaguely-related existing code rather than introduce a precise one — so the taxonomy drifts toward generic codes carrying specific messages.

3. **Codes are rarely branched on in production code.** Outside of tests, there are essentially no `if status == StatusCode::FOO { ... } else { ... }` branches anywhere in the codebase. Downstream code overwhelmingly inspects the *message*, not the code. A 273-variant surface optimises for a use case that does not really exist; what callers actually need is a small set of categories they *can* branch on, plus a useful message.

4. **i18n is not a goal and never will be.** There is no plan to translate codes into different languages. The reason error codes traditionally avoid carrying English text — to leave room for translation — does not apply here, so messages can carry human-readable context directly.

5. **Messages are inconsistently surfaced.** Notably, transactions rejected in mempool or prologue have their message dropped before reaching users — only the status code surfaces (e.g. `E_SEQUENCE_NUMBER_TOO_NEW`), without the descriptive context the message would carry.

6. **There is no compile-time guarantee of error-path coverage.** With error variants encoded as integer constants attached to a free-form message, tests cannot assert against a specific error path without inspecting message strings, and adding a new error site does not force a corresponding test to be added.

A code survey corroborates these patterns: of the 273 `StatusCode` variants, roughly 35% are never raised, another 40% appear at fewer than three call sites, every numeric range has a catch-all `UNKNOWN_*` code, and the structured fields (`indices`, `offsets`, `sub_status`) are inconsistently set across error sites. The "live" surface is closer to 60 codes than 273.

MonoMove starts from a clean slate, so the error system can be redesigned around an explicit set of goals.

---

## 2. Goals

1. **Small, stable public surface.** A caller can branch on a small number of categories with full coverage of the failure space. Adding a new error site inside the VM should not add a new public category.

2. **Messages are surfaceable and deterministic.** Every error carries a message that can be surfaced when needed (e.g. for debugging). The message is deterministic for a given internal variant and payload — the same error in the same context produces the same bytes. Messages are not i18n-translated and never will be. Phrasing changes are a soft contract: possible across major versions but reviewed the way protocol constants are. Downstream consumers that need *programmatic* access to a value should rely on the public category and any structured payload it carries ([§10.1](#101-structured-payloads-on-vmerrorkind-variants)), not parse the message string.

3. **Exhaustive internal taxonomy.** Every error site is a variant of a typed internal enum carrying structured payload (indices, types, addresses — whatever is useful for diagnosis). There is no untyped catch-all variant and no `with_message("...")` escape hatch. This lets tests match on variants and fields directly, rather than on numeric codes or message substrings.

4. **Compile-time category decisions.** The internal-to-public mapping is a single exhaustive `match` per subsystem (e.g. a `From` impl). Adding an internal variant fails to compile until the corresponding public category is chosen explicitly.

5. **Uniform propagation through a single boundary error.** Inside the VM, errors propagate as `VMInternalError` — a boxed `IntoExecutionError` — so `?` works across functions and components without every signature naming (and depending on) the concrete error type. This erasure is *not* lossy: the concrete typed error is recoverable (`downcast_ref`) and its public category is always available, so the typed taxonomy of Goal 3 is preserved. What stays banned is *lossy* erasure — `anyhow::Error`, `Result<T, String>`, `?`-on-anyhow — which drops the variant with no way to recover it. anyhow remains acceptable at outer harness boundaries (e.g. testsuite glue).

6. **Aborts and errors are structurally separate end-to-end.** A user abort is the program asking to stop; an error is the VM unable to continue. They flow through distinct types and distinct return channels at every layer — different categories of result, not different cases of the same error type.

---

## 3. Non-Goals

- **Backward compatibility with `StatusCode` numeric values.** MonoMove does not promise to preserve the integer codes from the existing VM. A test helper for comparing old-VM and new-VM outcomes (e.g. for replay parity) is in scope, but it is not a stable mapping.
- **A general-purpose error framework.** This design is specific to MonoMove's needs. It is not a recommendation for Aptos-level error types outside the VM.
- **Reproducing every diagnostic field of the current `ExecutionError`.** Fields like `indices` and `offsets` are encoded into the message string at conversion time; they are not separately retrievable from `ExecutionError`. If a downstream consumer needs them programmatically, that is a structured-payload question ([§10.1](#101-structured-payloads-on-vmerrorkind-variants)).
- **Transaction-prologue error handling.** The current VM reserves codes 0–999 for transaction validation (signature checks, sequence numbers, balance checks). These belong outside the VM in MonoMove's layering and have their own error type. The same two-layer shape (typed internal taxonomy + small public surface) is recommended for that layer too — including persisting the message for txns rejected in mempool or prologue, which the current pipeline does not do — but the design of that layer is not in scope here.

---

## 4. Terminology

| Term | Definition |
| --- | --- |
| **ExecutionResult** | The top-level result of executing a transaction or script: success, user abort, or VM failure. Replaces the same-named type from VM 1.0. |
| **ExecutionError** | The public error type returned for non-success, non-abort outcomes. Carries a small `kind` enum, a message, and an optional location. |
| **ExecutionErrorKind** | The small public category enum. The set of values callers can meaningfully branch on. |
| **Internal error** | A fully-typed internal enum (e.g. `RuntimeError`, `VerifierError`) with one variant per real error site. Used inside the VM crates and in tests. |
| **Conversion** | The `From` impl that maps an internal error to a `ExecutionError`, formatting the message and selecting the public category. |

---

## 5. Two-Layer Architecture

Errors flow through two layers — a typed internal error specific to each subsystem, then a small public surface that callers consume. The two layers are described in §5.1–§5.3. Aborts are not errors and take a separate channel; see §6 for the public `ExecutionResult` and §6.1 for how the runtime produces it.

### 5.1 Internal errors

Each VM subsystem (verifier, deserializer, loader, runtime) defines its own `enum` whose variants enumerate every concrete error condition that subsystem can produce. Variants carry structured payload (indices, offsets, types, addresses) — whatever is useful for diagnosis or for tests to assert on.

```rust
pub(crate) enum RuntimeError {
    VectorIndexOutOfBounds { index: u64, len: u64 },
    PopFromEmptyVector,
    ResourceNotFound { addr: AccountAddress, ty: TypeTag },
    ResourceAlreadyExists { addr: AccountAddress, ty: TypeTag },
    TypeMismatch { expected: Type, actual: Type },
    DivisionByZero,
    ArithmeticOverflow { op: ArithOp, lhs: u128, rhs: u128 },
    CallStackTooDeep { depth: usize },
    // ... one variant per real runtime error site
}
```

These types are crate-private. They are the contract that tests and internal callers see, but not the public API. They implement `Display` (typically derived via `thiserror::Error`) so that the message lives with the variant — the conversion to `ExecutionError` only needs to pick the public category, not format a string.

### 5.2 Public ExecutionError

```rust
pub struct ExecutionError {
    pub kind: ExecutionErrorKind,
    pub message: String,
    pub location: Option<Location>,
}

pub enum ExecutionErrorKind {
    OutOfGas,
    RuntimeLimitExceeded,
    InvalidOperation,
    VerificationFailed,
    DeserializationFailed,
    LinkError,
    InvariantViolation,
}
```

`ExecutionErrorKind` is the stable public surface. Callers branch on it; they do not branch on the internal types or on the message string. The message is human-readable and may be persisted, but should not be parsed programmatically.

Variants may carry kind-specific structured payload (e.g. `OutOfGas { gas_used: u64, gas_limit: u64 }`) when downstream consumers need programmatic access to a value rather than parsing it out of the message. Which variants warrant payload is the open question [§10.1](#101-structured-payloads-on-vmerrorkind-variants).

### 5.3 Conversion

Each internal error type implements `Into<ExecutionError>`. The message comes from the internal type's `Display` impl; the conversion's only job is to pick the public category:

```rust
impl From<RuntimeError> for ExecutionError {
    fn from(err: RuntimeError) -> Self {
        use RuntimeError::*;
        let kind = match err {
            VectorIndexOutOfBounds { .. } => ExecutionErrorKind::InvalidOperation,
            PopFromEmptyVector            => ExecutionErrorKind::InvalidOperation,
            ResourceNotFound { .. }       => ExecutionErrorKind::InvalidOperation,
            ResourceAlreadyExists { .. }  => ExecutionErrorKind::InvalidOperation,
            TypeMismatch { .. }           => ExecutionErrorKind::InvalidOperation,
            DivisionByZero                => ExecutionErrorKind::InvalidOperation,
            ArithmeticOverflow { .. }     => ExecutionErrorKind::InvalidOperation,
            CallStackTooDeep { .. }       => ExecutionErrorKind::RuntimeLimitExceeded,
            // ...
        };
        ExecutionError { kind, message: err.to_string(), location: None }
    }
}
```

The category match is the only place where the mapping internal→public is encoded. It is exhaustive (no wildcard), so adding a new internal variant fails to compile until its public category is chosen explicitly.

---

## 6. Top-Level ExecutionResult

A user abort is the program asking to stop; an error is the VM unable to continue. Conflating them in a single error enum forces every caller to inspect a discriminator before doing anything useful. Splitting them at the top level is clearer:

```rust
pub enum ExecutionResult {
    Success,
    Aborted {
        code: u64,
        message: Option<String>,
        location: Location,
    },
    Failed(ExecutionError),
}
```

`Aborted` carries a structured `code: u64` because every downstream consumer (Move source-level error mapping, CLI output, indexer) needs to read it programmatically. The optional `message` carries a user-supplied string from Move source — populated when an abort uses the message form (`AbortWithMessage` bytecode) or when a native function aborts with a message — and is `None` for code-only aborts. The `location` identifies which module raised the abort.

The `code` follows the existing `errors.move` convention: the upper byte categorises (`INVALID_ARGUMENT`, `INVALID_STATE`, `OUT_OF_RANGE`, `LIMIT_EXCEEDED`, `INTERNAL`, `NOT_IMPLEMENTED`, `UNAVAILABLE`, etc.) and the lower bytes carry a module-specific reason code. This is the place where Move-source-level canonical error categories surface — they classify *application* failures and are orthogonal to `ExecutionErrorKind`, which classifies VM-detected failures.

Abort messages were added to the current VM recently (new `AbortWithMessage` bytecode instruction and abort-message support in native functions, gated by the `NATIVE_ABORT_MESSAGES` feature flag). MonoMove adopts them natively — no feature gate — and treats the message as an integral part of the abort outcome, persisted alongside the code. The same size cap that the current VM enforces applies, to keep abort-message storage costs predictable.

### 6.1 How the runtime produces it

The interpreter's `run` method ([interpreter.rs:1202](third_party/move/mono-move/runtime/src/interpreter.rs#L1202)) currently returns `ExecutionResult<()>`. Under this design, it returns a result over an `RuntimeStatus` payload covering the two successful terminal cases:

```rust
pub(crate) enum RuntimeStatus {
    Success,
    Aborted { code: u64, message: Option<String>, location: Location },
}

impl Interpreter {
    pub fn run(&mut self) -> Result<RuntimeStatus, RuntimeError> { .. }
}
```

The three terminal cases are:

- Normal return from the entry function → `Ok(RuntimeStatus::Success)`
- `Abort` opcode or abort-returning native function → `Ok(RuntimeStatus::Aborted { ... })`
- Runtime failure (vector OOB at the VM level, missing resource, gas exhaustion, etc.) → `Err(RuntimeError::...)`

The orchestrator lifts these into `ExecutionResult`:

```rust
match interpreter.run() {
    Ok(RuntimeStatus::Success) => ExecutionResult::Success,
    Ok(RuntimeStatus::Aborted { code, message, location }) => ExecutionResult::Aborted { code, message, location },
    Err(e) => ExecutionResult::Failed(ExecutionError::from(e)),
}
```

`RuntimeError` has no abort variant; the `From<RuntimeError> for ExecutionError` conversion in §5.3 has no abort case. Aborts are tagged at every internal level (e.g. native function dispatch returns a `NativeResult` enum with a distinct `Abort` variant) so they propagate up through the `Ok` channel of the `Result`, structurally distinct from runtime failures in the `Err` channel — which is what makes Goal §2.6 hold.

---

## 7. Public Categories

The seven `ExecutionErrorKind` variants, with the rough class of internal errors that map into each:

| Kind | Meaning | Examples of internal variants |
| --- | --- | --- |
| `OutOfGas` | Gas budget exhausted. | `GasExhausted { used, limit }` |
| `RuntimeLimitExceeded` | A static or dynamic structural limit was hit. | `CallStackTooDeep`, `ValueDepthTooDeep`, `TypeDepthTooDeep` |
| `InvalidOperation` | Program attempted an operation that failed at runtime. | `VectorIndexOutOfBounds`, `PopFromEmptyVector`, `ResourceNotFound`, `ResourceAlreadyExists`, `TypeMismatch`, `BadCast`, `DivisionByZero`, `ArithmeticOverflow`, `ArithmeticUnderflow`, `ShiftAmountOutOfRange` |
| `VerificationFailed` | Bytecode verifier rejected a module before execution. | All variants of `VerifierError` (control-flow, type safety, bounds, ability checks). |
| `DeserializationFailed` | A binary blob (module, script, value) was malformed. | `BadMagic`, `UnknownOpcode`, `TruncatedInput`, `InvalidUtf8`, `BadFieldEncoding` |
| `LinkError` | A referenced module, function, or struct could not be resolved or had an incompatible signature. | `ModuleNotFound`, `FunctionNotFound`, `StructNotFound`, `SignatureMismatch`, `UpgradeIncompatible` |
| `InvariantViolation` | A condition that should never occur — a VM bug. | `MissingFrame`, `LoaderInconsistency`, `ParanoidCheckFailed` |

`InvariantViolation` is operationally distinct from the rest: it indicates a VM bug, not a program bug. Production deployments should alert on it; users should never see it surface as a transaction failure with diagnostic detail. Keeping it as a category (rather than panicking) lets the orchestrator translate the transaction outcome cleanly without unwinding. These can also be raised speculatively (e.g. from a parallel re-execution that read inconsistent state), so alerts should fire only on non-speculative failures.

Note: speculative aborts should be folded into this same handling.
Block-STM today sees a speculative abort and early-halts execution (or waits).
Conceptually it can do the same on every `InvariantViolation`: we do not have a consistent way to say *this violation can only be speculative*, *this may or may not be speculative*, or *this can only happen due to a bug and never due to speculation*.
So ideally any such violation is an early-halt / wait signal, with alerting gated on whether the failure ultimately commits.

`Aborted` is *not* in this list because it lives at the `ExecutionResult` level (§6), not inside `ExecutionError`.

---

## 8. Why This Categorisation

The categories are chosen by **what action a caller takes when they see one**, not by where in the VM the error originated. Two error sites that the caller handles identically should map to the same kind; two error sites that demand different handling should not.

Conversely, a single subsystem typically produces errors across multiple categories. The loader, for instance, raises `DeserializationFailed` (malformed module bytes), `VerificationFailed` (verifier rejection during load), `LinkError` (missing or incompatible dependency), and occasionally `InvariantViolation` (cache corruption). The specializer leans toward `RuntimeLimitExceeded` (depth and instantiation caps) and `InvariantViolation` (since its input is already verified, most failures genuinely shouldn't happen). The runtime spans almost every category. The classification is variant-level, not subsystem-level — each subsystem's internal enum is mapped variant-by-variant to a public category by its `From` impl.

- `OutOfGas` vs. `RuntimeLimitExceeded`: both are "you asked for too much," but the caller's response differs. `OutOfGas` is fixable by raising the gas budget; `RuntimeLimitExceeded` is not (raising gas does nothing for a 200-deep call stack).
- `VerificationFailed` vs. `DeserializationFailed` vs. `LinkError`: all are pre-execution module-loading failures, but the caller's response differs. Verification failures mean the module is invalid; deserialisation failures mean the bytes are corrupted; link errors mean a dependency is missing or incompatible (and may be fixable by publishing the dependency).
- `InvariantViolation` vs. all others: invariant violations require operational alerting; everything else is a normal failure mode.

Whether `InvalidOperation` should be sub-categorised (e.g. into `ResourceError`, `TypeError`, `IndexError`) is an open question. Those distinctions are useful for diagnosis and would naturally surface in the internal variant and message regardless; promoting them to public categories depends on whether callers want to branch on them.

---

## 9. Testability

The design's testability claim rests on the internal error types being exhaustive and their variants carrying structured payload. A test for a runtime feature can assert:

```rust
assert!(matches!(
    result,
    Err(RuntimeError::VectorIndexOutOfBounds { index: 7, len: 5 })
));
```

— rather than:

```rust
assert_eq!(result.unwrap_err().major_status, StatusCode::VECTOR_OPERATION_ERROR);
assert!(result.unwrap_err().message.unwrap().contains("index 7"));
```

The former binds against types; the latter binds against integers and English. Adding a new error site is a new internal variant, and the exhaustive `From` impl forces an explicit category decision, which means every error path has a compile-time-verified place in both the internal taxonomy and the public taxonomy.

---

## 10. Open Questions

### 10.1 Structured payloads on `ExecutionErrorKind` variants

The drafted `ExecutionErrorKind` shows all variants as units; the design permits structured payload on any of them (see §5.2). Which variants should grow payload?

Arguments for: downstream consumers (indexer, gas profiler, CLI) sometimes need to extract values programmatically, and parsing the message is fragile.

Arguments against: every additional structured field is a versioning commitment, and the internal error type already carries the data.

Pragmatic middle ground: keep variants as units by default, and promote a field to structured payload only when every downstream consumer would otherwise have to parse it out of the message. Candidates worth considering: `RuntimeLimitExceeded` (which limit, and the value), `InvalidOperation` (operation kind), `LinkError` (the missing module/function name).

### 10.2 Error chaining

Should errors carry a `source: Option<Box<dyn std::error::Error>>` for chained causes (e.g. an I/O error during module load wrapped as a `LinkError`)? Useful for debugging, but introduces dynamic dispatch on the public surface.

Probably not on the public `ExecutionError`. The component-specific internal errors (`RuntimeError`, `VerifierError`, etc.) are a more natural home if chained causes turn out to be useful in practice.

### 10.3 Compatibility with the existing `TransactionStatus`

The on-chain shape of a transaction outcome is `TransactionStatus`, which wraps `ExecutionStatus` (5 variants: `Success`, `OutOfGas`, `MoveAbort`, `ExecutionFailure`, `MiscellaneousError`). The integration between the VM and the rest of Aptos lives at one site, [`TransactionStatus::from_vm_status`](aptos-move/aptos-vm/src/aptos_vm.rs#L629), where today's `VMStatus` gets boiled down.

The open question is how MonoMove's `ExecutionResult` plugs into that same boundary — directly, or via a thin shim that mirrors `from_vm_status`. The mapping is straightforward (`ExecutionResult::Success` → `ExecutionStatus::Success`, `ExecutionResult::Aborted { ... }` → `ExecutionStatus::MoveAbort { ... }`, the various `ExecutionErrorKind`s split between `OutOfGas`, `ExecutionFailure`, and `MiscellaneousError`), but it does need to be designed alongside how the VM is wired in.

Block-STM (the block executor in `aptos-move/block-executor`) sits on this path: it runs the VM per-transaction and propagates outputs onward, so the block executor is part of the integration scope too — not just the single `from_vm_status` site.
