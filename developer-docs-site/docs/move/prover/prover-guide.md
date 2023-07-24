---
title: "Move Prover User Guide"
slug: "prover-guide"
---

# Move Prover User Guide

This is the user guide for the Move Prover. This document accompanies the
[Move specification language](spec-lang.md). See the sections below for details.

## Running the Move Prover

The Move Prover is invoked via the [Aptos CLI](../../tools/aptos-cli/use-cli/use-aptos-cli.md#move-examples). In order to call the CLI, you must have a [*Move package*](../book/packages.md) in place. In the simplest case, a Move package is defined by a directory with a set of `.move` files in it and a manifest of the name `Move.toml`. You can create a new Move package at a given location by running the command: `aptos move init --name <name>`

Once the package exists, call the Move Prover from the directory to be tested or by supplying its path to the `--package-dir` argument:

  * Prove the sources of the package in the current directory:
    ```shell
  aptos move prove
  ```

  * Prove the sources of the package at &lt;path&gt;:
  ```shell
  aptos move prove --package-dir <path>
  ```

See example output and other available options in the [Proving Move](../../tools/aptos-cli/use-cli/use-aptos-cli.md#proving-move) section of Use Aptos CLI.

### Target filtering

By default, the `aptos move prove` command verifies all files of a package. During iterative development of larger packages, it is often more effective to focus verification on particular files with the
`-f` (`--filter`) option, like so:

```shell script
aptos move prove -f coin
```

In general, if the string provided to the `-f` option is contained somewhere in the file name of a source, that source will be included for verification.

> NOTE: the Move Prover ensures there is no semantic difference between verifying modules one-by-one
> or all at once. However, if your goal is to verify all modules, verifying them in a single
> `aptos move prove` run will be significantly faster than sequentially.

### Prover options

The Move Prover has a number of options (such as the filter option above) that you pass with an invocation of: `aptos move prove <options>`. The most commonly used option is the `-t` (`--trace`) option that causes the Move Prover to produce richer diagnosis when it encounters errors:

```shell script
aptos move prove -f coin -t
```

To see the list of all command line options, run: `aptos move prove --help`

### Prover configuration file

You can also create a Move Prover configuration file named `Prover.toml` that lives side-by-side with the `Move.toml` manifest file in the root of the package directory. For example, to enable tracing by default for a package, add a `Prover.toml` file with the following configuration:

```toml
[prover]
auto_trace_level = "VerifiedFunction"
```

Find the most commonly used options in the example `.toml` below, which you can cut and paste and adopt for your needs (adjusting the defaults shown in the displayed values as needed):

```toml
# Verbosity level
# Possible values: "ERROR", "WARN", "INFO", "DEBUG". Each level subsumes the output of the previous one.
verbosity_level = "INFO"

[prover]
# Set auto-tracing level, which enhances the diagnosis the Move Prover produces on verification errors.
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

# The number of processor cores to assume for concurrent check of verification conditions.
proc_cores = 4
```

> HINT: For local verification, you may want to set `proc_cores` to an aggressive number
> (your actual cores) to speed up the turnaround cycle.


## Prover diagnosis

When the Move Prover finds a verification error, it prints diagnosis to standard output in a style similar to a compiler or a debugger. We explain the different types of diagnoses below, based on the following evolving example:

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

### Unexpected abort

If we run the Move Prover on the example immediately above, we get the following error:

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

The Move Prover has generated an example counter that leads to an overflow when adding 1 to the value of 255 for an `u8`. This overflow occurs if the function specification calls for abort behavior, but the condition under which the function is aborting is not covered by the specification. And in fact, with `aborts_if !exists<Counter>(a)`, we only cover the abort caused by the absence of the resource, but not the abort caused by the arithmetic overflow.

Let's fix the above and add the following condition:

```move
spec increment {
    aborts_if global<Counter>(a).value == 255;
    ...
}
```

With this, the Move Prover will succeed without any errors.

### Postcondition failure

Let us inject an error into the `ensures` condition of the above example:

```move
spec increment {
    ensures global<Counter>(a).value == /*old*/(global<Counter>(a).value) + 1;
}
```

With this, the Move Prover will produce the following diagnosis:

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

While we know what the error is (as we just injected it), this is not particularly obvious in the output This is because we don't directly see on which values the `ensures` condition was actually evaluated. To see
this, use the `-t` (`--trace`) option; this is not enabled by default because it makes the verification problem slightly harder for the solver.

Instead or in addition to the `--trace` option, you can use the built-in function `TRACE(exp)` in conditions to explicitly mark expressions whose values should be printed on verification failures.

> NOTE: Expressions that depend on quantified symbols cannot be traced. Also, expressions appearing in
> specification functions can not currently be traced.

## Debugging the Move Prover

The Move Prover is an evolving tool with bugs and deficiencies. Sometimes it might be necessary to debug a problem based on the output the Move Prover passes to the underlying backends. If you pass the option `--dump`, the Move Prover will output the original Move bytecode, as well as the Move Prover bytecode, as the former is transformed during compilation.
