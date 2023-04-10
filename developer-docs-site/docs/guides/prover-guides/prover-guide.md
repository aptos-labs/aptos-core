# Move Prover User Guide

This is the user guide for the Move prover. This document does not describe the
[Move specification language](spec-lang.md), but accompanies it.

- [Running the Prover](#running-the-prover)
    - [Target Filtering](#target-filtering)
    - [Prover Options](#prover-options)
    - [Prover Configuration File](#prover-configuration-file)
    - [Prover Tests](#prover-tests)
- [Prover Diagnosis](#prover-diagnosis)
    - [Unexpected Abort](#unexpected-abort)
    - [Postcondition Failure](#postcondition-failure)
- [Debugging the Prover](#debugging-the-prover)

## Running the Prover

The prover is invoked via the Aptos CLI. In order to call the CLI, you must have a [*move package*](https://aptos.dev/guides/move-guides/book/packages). In the simplest case, a Move package is defined by a directory with a set of `.move` files in it and a manifest of the name `Move.toml`. You can create a new Move package at a given location by `aptos move init --name <name>`.

Now, to call the prover simply use one of the following commands:

```shell script
aptos move prove --package-dir <path> # Prove the sources of the package at <path>
aptos move prove                      # Equivalent to `aptos move prove --package-dir .`
```

### Target Filtering

By default, the `prove` command verifies all files of a package. During iterative development of larger packages, it is
often more effective to focus verification on particular files. You do this with the
`-f` (`--filter`) option:

```shell script
aptos move prove -f coin
```

In general, if the string provided via the `-f` option is contained somewhere in the file name of a source, that source
will be included for verification.

> NOTE: the Move prover ensures that there is no semantic difference between verifying modules one-by-one
> or all at once. However, if your goal is to verify all modules, verifying them in a single
> `aptos move prove` run will be significantly faster than sequentially.

### Prover Options

The prover has a number of options. Like the filter option, you pass options through with an invocation like `aptos move prove <options>`. The most commonly used option is the `-t` (`--trace`) option which lets the prover produce richer diagnosis when it encounters errors:

```shell script
aptos move prove -f coin -t
```

To see the list of all command line options, use `aptos move prove --help`.

### Prover Configuration File

You can also create a prover configuration file, named `Prover.toml` which lives side-by-side with the `Move.toml`
file. For example, to enable tracing by default for a package, you use a `Prover.toml` with the following content:
- [Move Specification Language](#move-specification-language)

```toml
[prover]
auto_trace_level = "VerifiedFunction"
```

The most commonly used options are documented by the example `.toml` below, which you can cut and paste and adopt for your
needs (the displayed values are the defaults):

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

# The number of processors cores to assume for concurrent check of verification conditions.
proc_cores = 4
```

> HINT: for local verification, you may want to set proc_cores to an aggressive number
> (your actual cores) to speed up the turn-around cycle.


## Prover Diagnosis

When the prover finds a verification error it prints out diagnosis in a style similar to a compiler or a debugger. We
explain the different types of diagnoses below, based on the following evolving example:

```move
module 0x0::m {
    struct Counter has key {
        value: u8,
    }

    public fun increment(a: address) acquires Counter {
        let r = borrow_global_mut<Counter>(a);
        r.value = r.value + 1;
    }

    spec increment {
        aborts_if !exists<Counter>(a);
        ensures global<Counter>(a).value == old(global<Counter>(a)).value + 1;
    }
}
```

We will modify this example as we demonstrate different types of diagnoses.

### Unexpected Abort

If we run the Move prover on the above example, we get the following error:

```
error: abort not covered by any of the `aborts_if` clauses
   ┌─ m.move:11:5
   │
 8 │           r.value = r.value + 1;
   │                             - abort happened here with execution failure
   ·
11 │ ╭     spec increment {
12 │ │         aborts_if !exists<Counter>(a);
13 │ │         ensures global<Counter>(a).value == old(global<Counter>(a)).value + 1;
14 │ │     }
   │ ╰─────^
   │
   =     at m.move:6: increment
   =         a = 0x29
   =     at m.move:7: increment
   =         r = &mmm.Counter{value = 255u8}
   =     at m.move:8: increment
   =         ABORTED

{
  "Error": "Move Prover failed: exiting with verification errors"
}
```

The prover has generated a counter example which leads to an overflow when adding 1 the value of 255 for an `u8`. This
happens if the function specification states something abort behavior, but the condition under which the function
is aborting is not covered by the specification. And in fact, with `aborts_if !exists<Counter>(a)` we only cover the
abort if the resource does not exists, but not the overflow.

Let's fix the above and add the following condition:

```move
spec increment {
    aborts_if global<Counter>(a).value == 255;
    ...
}
```

With this, the prover will succeed without any errors.

### Postcondition Failure

Let us inject an error into the `ensures` condition of the above example:

```move
spec increment {
    ensures global<Counter>(a).value == /*old*/(global<Counter>(a).value) + 1;
}
```

With this, the prover will produce the following diagnosis:

```
error: post-condition does not hold
   ┌─ m.move:14:9
   │
14 │         ensures global<Counter>(a).value == /*old*/(global<Counter>(a).value) + 1;
   │         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
   │
   =     at m.move:6: increment
   =         a = 0x29
   =     at m.move:7: increment
   =         r = &mmm.Counter{value = 0u8}
   =     at m.move:8: increment
   =     at m.move:9: increment
   =     at m.move:12: increment (spec)
   =     at m.move:15: increment (spec)
   =     at m.move:13: increment (spec)
   =     at m.move:14: increment (spec)

{
  "Error": "Move Prover failed: exiting with verification errors"
}
```

While we know what the error is (we just injected it), looking at the printed information makes it not particular
obvious. This is because we don't directly see on which values the `ensures` condition was actually evaluated. To see
this, use the `-t` (`--trace`) option; this is not enabled by default because it makes the verification problem slightly
harder for the solver.

Instead or in addition to the `--trace` option, one can also use the builtin function `TRACE(exp)` in conditions to
explicitly mark expressions whose value should be printed on verification failures.

> NOTE: expressions which depend on quantified symbols cannot be traced. Also, expressions appearing in
> specification functions can currently not be traced.

## Debugging the Prover

The Move prover is an evolving tool with bugs and deficiencies. Sometimes it might be necessary to debug a problem based
on the output it passes to the underlying backends. There are the following options to this end:

- If you prove the option `--dump`, the prover will dump the original Move bytecode as well as the Prover
  bytecode as it is transformed during compilation.
