---
id: velor-gas-calibration
title: Velor Automated Gas Calibration
---

## Velor Automated Gas Calibration

The Velor Automated Gas Calibration is a tool that lets anyone write Move Samples (or also Move IR) to calibrate the gas parameters for Native Functions (and also Move bytecode instructions). 

### Terminology:

- User: Anyone writing Move Native Functions.
- Move Sample: A Move package in the `/samples` directory.
- Abstract Gas Usage: Records the number of times a gas parameter has been called.
- Calibration Function: A function used to track the running time and Abstract Gas Usage.

### How the system works at a high-level:

1. User implements the Native function and respective gas formula.
2. Write a Move Sample and some Calibration Functions.
3. Determine the Abstract Gas Usage of the Calibration Function to create a gas formula.
4. Determine the running time of the Calibration Function and formulate a linear equation.
5. Repeat 3-4 for all Move Samples in `/samples` 
6. Solve the system of linear equations using Linear Algebra
7. Output a result to the User (the gas parameter costs, or any outliers, or gas parameters that couldn’t be solved)

## Creating a Move Sample

Create a Move project under `/samples` by running:

```bash
mkdir MY-PROJECT
cd MY-PROJECT
cargo run -p velor -- move init --name MY-PROJECT
```

## Writing Your First Calibration Function

Calibration Functions need to be marked with `entry` and have a prefix of `calibrate_`. For example, the following functions would work:

```Move
//// VALID
public entry fun calibrate() {}

public entry fun calibrate_another_txn() {}

public entry fun calibrate123() {}

//// INVALID
public fun calibrate() {}

public fun test_my_txn() {}

public entry fun calibrate_addition(_x: u64, _y: u64) {}
```

If the Calibration Function is expected to error, please denote it with the postfix `_should_error`.

```
//// VALID
public entry fun calibrate_my_test_should_error() {}

public entry fun calibrate_should_error() {}

//// INVALID
public entry fun should_error_calibrate() {}

public entry fun calibrate_test_error() {}

public entry fun calibrate_test_should_error_() {}
```
Note: These are still valid Calibration Functions that will still run, but would not error.

## Usage

```bash
cargo run --release -- --help 
Automated Gas Calibration to calibrate Move bytecode and Native Functions

Usage: velor-gas-calibration [OPTIONS]

Options:
  -p, --pattern <PATTERN>                         Specific tests to run that match a pattern [default: ""]
  -i, --iterations <ITERATIONS>                   Number of iterations to run each Calibration Function [default: 20]
  -m, --max_execution_time <MAX_EXECUTION_TIME>   Maximum execution time in milliseconds [default: 300]
  -h, --help                                      Print help
```

## Examples

There are examples of how to write Calibration Functions under `/samples_ir` and `/samples`. There will be more examples in the future as more Users write Move Samples and add it to the calibration set. 

Here is an example written in the Move source language:
```Move
public fun calibrate_blake2b_256_impl(num_iterations: u64) {
    let i = 0;
    let msg = b"abcdefghijkl"
    while i < num_iterations {
        // This is what I want to calibrate:
        velor_hash::blake2b_256(msg);
        velor_hash::blake2b_256(msg);
        velor_hash::blake2b_256(msg);
        velor_hash::blake2b_256(msg);
        velor_hash::blake2b_256(msg);
        velor_hash::blake2b_256(msg);
        velor_hash::blake2b_256(msg);
        velor_hash::blake2b_256(msg);
        velor_hash::blake2b_256(msg);
        velor_hash::blake2b_256(msg);        
        i += 1;
    }
}

public entry fun calibrate_blake2b_256_x500() {
    calibrate_blake2b_256_impl(50);
}

public entry fun calibrate_blake2b_256_x1000() {
    calibrate_blake2b_256_impl(100);
}

public entry fun calibrate_blake2b_256_x5000() {
    calibrate_blake2b_256_impl(500);
}
```
As you can see in this example, we want the thing that we are trying to calibrate to run for a long enough time, and to also be called enough times. This is why we have the Native Function being called 10 times. Furthermore, we want to sample different lengths (discussed in more detail below), which is why we have data points at 500, 1000, and 5000. This allows the system to record an accurate and appropriate gas usage of the instructions being called, while finding the best line of fit. 

## FAQ

### How many Calibration Functions to provide?

In order for the system to find deterministic values for the gas parameters, the number of Calibration Functions needs to be at least the number of linearly independent samples for each gas parameter that we’re solving. For example, if a User was calling a function, it would use the gas parameters: `CALL_BASE`, `CALL_PER_ARG`, `CALL_PER_LOCAL`, which would require at least three Calibration Functions. 

### How do I write "good" Calibration Functions?

A good Calibration Function should run for many iterations (i.e., see `/samples_ir/ld/ldu8.mvir`). This allows the system to record a good representation of the gas usage. Furthermore, the idea is that we are sampling different data points at different "lengths" to approximate a line of best fit. In the `ldu8.mvir` example, we have the data points at 100, 500, and 1000 iterations. 

### How are the gas parameters calculated?

For every Calibration Function, the Abstract Gas Usage and running time are determined. This forms a linear equation. For all the Calibration Functions, we can create a system of lienar equations. To solve this system of linear equations, we compute the Least Squares Solution. 

If the matrix that represents this system is not invertible, then we report the undetermined gas parameters, or the linearly dependent combinations of gas parameters. The exact math can be found under `/src/math.rs`. 

Otherwise, the User can expect to see all the values, along with the running times and any outliers. 

### I see "linearly dependent variables" instead of the gas costs, what do I do?

If you happen to see something like:

```
linearly dependent variables are:

- gas parameter: HASH_BLAKE2B_256_BASE
- gas parameter: HASH_BLAKE2B_256_PER_BYTE
```

There are a few reasons as to why this would happen. The first reason would be that you may have an insufficient number of Calibration Functions for the gas parameters you are trying to calculate. In this example, there should be at least two Calibration Functions, since there are two gas parameters. Another reason would be that too many of the Calibration Functions are linearly dependent. That is, try writing them using different input sizes and varying number of iterations.

