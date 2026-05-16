# Move Prover User Guide

This is the user guide for the Move Prover. This document does not describe the
[Move Specification Language](spec-lang.md), but accompanies it. For the
underlying CLI invocation, see [`aptos move prove`](cli-develop.md#aptos-move-prove).

## Running the Prover

The prover is invoked via the Aptos CLI from inside a Move package directory:

```shell
aptos move prove                  # verify the package in the current directory
aptos move prove --package-dir <path>
```

Before running the prover for the first time, install the external dependencies
it relies on (`boogie` and `z3`):

```shell
aptos update prover-dependencies
```

### Target Filtering

By default, `aptos move prove` verifies all source files of a package. During
iterative development of larger packages, it is often more effective to focus
verification on particular files. Use the `--filter` (alias `-f`) option:

```shell
aptos move prove --filter DiemConfig
```

If the string passed to `--filter` is contained anywhere in the file name of a
source, that source will be included for verification.

> NOTE: the Move Prover ensures that there is no semantic difference between
> verifying modules one-by-one or all at once. However, if your goal is to
> verify all modules, doing it in a single `aptos move prove` run is
> significantly faster than running once per module.

### Prover Options

The prover accepts a number of options that the CLI passes through directly.
Common ones surface as named flags (`--vc-timeout`, `--random-seed`,
`--trace`, etc.); for the full list run

```shell
aptos move prove --help
```

The most commonly used option is `--trace`, which makes the prover produce
richer diagnostics when it encounters errors:

```shell
aptos move prove --filter DiemConfig --trace
```

### Prover Configuration File

You can also create a prover configuration file named `Prover.toml` next to
your `Move.toml`. For example, to enable tracing by default for a package, use
a `Prover.toml` containing:

```toml
[prover]
auto_trace_level = "VerifiedFunction"
```

The most commonly used options are documented by the example TOML below, which
you can copy and adjust for your needs (the displayed values are the
defaults):

```toml
# Verbosity level
# Possible values: "ERROR", "WARN", "INFO", "DEBUG". Each level subsumes the output of the previous one.
verbosity_level = "INFO"

[prover]
# Set auto-tracing level, which enhances the diagnosis the prover produces on verification errors.
# Possible values: "Off", "VerifiedFunction", "AllFunctions"
auto_trace_level = "Off"

# Minimal severity level for diagnosis to be reported.
# Possible values: "Error", "Warning", "Note"
report_severity = "Warning"

[backend]
# Timeout in seconds for the solver backend. Note that this is a soft timeout and may not always
# be respected.
vc_timeout = 40

# Random seed for the solver backend. Different seeds can result in different verification run times,
# as the solver uses heuristics.
random_seed = 1

# The number of processor cores to assume for concurrent checking of verification conditions.
proc_cores = 4
```

> HINT: for local verification, you may want to set `proc_cores` to an aggressive
> number (your actual core count) to speed up the turnaround cycle.

> NOTE: To dump all available TOML options, run `aptos move prove --print-config`.
> The dump will, however, contain many more unrelated and potentially defunct
> experimental options.

### Prover Tests

The prover can also be run from a Rust testsuite — for example, to use
verification as a submit blocker. Add a Rust file to your testsuite (e.g.
`<crate>/tests/move_verification_test.rs`). Assuming the crate contains two
Move packages at relative paths `foo` and `bar`, the Rust source would look
like:

```rust
use move_cli::package::prover::ProverTest;

#[test]
fn prove_foo() {
    ProverTest::create("foo").run()
}

#[test]
fn prove_bar() {
    ProverTest::create("bar").run()
}
```

There are multiple ways to configure the test, for example by setting specific
options for the prover to use. See the `ProverTest` type for details.

## Prover Diagnosis

When the prover finds a verification error, it prints a diagnosis in a style
similar to a compiler or a debugger. We explain the different types of
diagnoses below, based on the following running example:

```move
module 0x42::counter {
    struct Counter has key {
        value: u8,
    }

    public fun increment(a: address) acquires Counter {
        Counter[a].value = Counter[a].value + 1;
    }

    spec increment {
        aborts_if !exists<Counter>(a);
        ensures global<Counter>(a).value == old(global<Counter>(a)).value + 1;
    }
}
```

We will modify this example as we demonstrate different types of diagnoses.

### Unexpected Abort

If we run the Move Prover on the above example, we get the following error:

```
error: abort not covered by any of the `aborts_if` clauses

   ┌── tutorial.move:6:3 ───
   │
 6 │ ╭   public fun increment(a: address) acquires Counter {
 7 │ │       Counter[a].value = Counter[a].value + 1;
 8 │ │   }
   │ ╰───^
   ·
 7 │       Counter[a].value = Counter[a].value + 1;
   │                                            - abort happened here
   │
   =     at tutorial.move:6:3: increment (entry)
   =     at tutorial.move:7:15: increment
   =         a = 0x5,
   =     at tutorial.move:7:36: increment (ABORTED)
```

The prover has generated a counterexample that leads to an overflow when
adding `1` to a value of `255` in a `u8`. This happens when the function
specification states something about abort behavior, but the condition under
which the function actually aborts is not covered by the specification. With
`aborts_if !exists<Counter>(a)`, we only cover the case where the resource
does not exist — not the overflow.

Let's fix that by adding the missing condition:

```move
spec increment {
    aborts_if global<Counter>(a).value == 255;
}
```

With this added, the prover will succeed without any errors.

### Postcondition Failure

Now let's inject an error into the `ensures` condition of the example:

```move
spec increment {
    ensures global<Counter>(a).value == /*old*/(global<Counter>(a).value) + 1;
}
```

The prover produces the following diagnosis:

```
error:  A postcondition might not hold on this return path.

    ┌── tutorial.move:14:7 ───
    │
 14 │       ensures global<Counter>(a).value == global<Counter>(a).value + 1;
    │       ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
    │
    =     at tutorial.move:6:3: increment (entry)
    =     at tutorial.move:7:15: increment
    =         a = 0x5,
    =     at tutorial.move:6:3: increment
    =     at tutorial.move:6:3: increment (exit)
```

While we know what the error is — we just injected it — the printed
information doesn't make that particularly obvious. This is because we don't
directly see the values on which the `ensures` condition was actually
evaluated. To see them, use the `--trace` option; it is not enabled by default
because it makes the verification problem slightly harder for the solver.

Instead of, or in addition to, the `--trace` option, you can use the built-in
function `TRACE(exp)` in conditions to explicitly mark expressions whose value
should be printed on verification failures.

> NOTE: expressions that depend on quantified symbols cannot be traced.
> Likewise, expressions appearing in specification functions cannot currently
> be traced.

## Debugging the Prover

The Move Prover is an evolving tool with bugs and deficiencies. Sometimes it
is necessary to debug a problem based on the output it passes to the
underlying backends. The following options help:

- `--dump` — writes intermediate artifacts next to the package: the generated
  Boogie code (`output.bpl`) and Boogie's reported errors (`output.bpl.log`),
  the SMT input passed to Z3, and the prover bytecode as transformed during
  compilation. This is the catch-all debug flag in the Aptos CLI.
- `--cvc5` — switches the underlying SMT backend from Z3 to cvc5. The `CVC5_EXE`
  environment variable must point at the binary.
- `--check-inconsistency` — injects an impossible assertion into each
  verification target. If verification still passes, the surrounding spec is
  vacuous (e.g. its preconditions are unsatisfiable) and isn't actually
  proving anything.
