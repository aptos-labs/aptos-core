---
id: aptos-gas-calibration
title: Aptos Automated Gas Calibration
---

## Aptos Automated Gas Calibration

The Aptos Automated Gas Calibration is a tool that lets anyone write Move Samples (or also Move IR) to calibrate the gas parameters for Native Functions (and also Move bytecode instructions). 

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
cargo run -p aptos -- move init --name MY-PROJECT
```

## Writing Your First Calibration Function

Calibration Functions need to be marked with `entry` and have a prefix of `calibrate_`. For example, the following functions would work:

```Move
//// acceptable formats
public entry fun calibrate() {}

public entry fun calibrate_another_txn() {}

public entry fun calibrate123() {}

//// inacceptable formats
public fun calibrate() {}

public fun test_my_txn() {}

public entry fun calibrate_addition(_x: u64, _y: u64) {}
```

If the Calibration Function is expected to error, please denote it with the postfix `_should_error`.

## Usage

```bash
cargo run -- --help 
Automated Gas Calibration to calibrate Move bytecode and Native Functions

Usage: aptos-gas-calibration [OPTIONS]

Options:
  -p, --pattern <PATTERN>        Specific tests to run that match a pattern [default: ]
  -i, --iterations <ITERATIONS>  Number of iterations to run each Calibration Function [default: 20]
  -h, --help                     Print help
```

## Examples

There are examples of how to write Calibration Functions under `/samples_ir` and `/samples`. There will be more examples in the future as more Users write Move Samples and add it to the calibration set.

## FAQ

### How many Calibration Functions to provide?

In order for the system to find deterministic values for the gas parameters, the number of Calibration Functions needs to be at least the number of linearly independent samples for each gas parameter that we’re solving. For example, if a User was calling a function, it would use the gas parameters: `CALL_BASE`, `CALL_PER_ARG`, `CALL_PER_LOCAL`, which would require at least three Calibration Functions. 

### How do I write "good" Calibration Functions?

A good Calibration Function should run for many iterations (i.e., see `/samples_ir/ld/ldu8.mvir`). This allows the system to record a good representation of the gas usage. Furthermore, the idea is that we are sampling different data points at different "lengths" to approximate a line of best fit. In the `ldu8.mvir` example, we have the data points at 100, 500, and 1000 iterations. 

### How are the gas parameters calculated?

For every Calibration Function, the Abstract Gas Usage and running time are determined. This forms a linear equation. For all the Calibration Functions, we can create a system of lienar equations. To solve this system of linear equations, we compute the Least Squares Solution. 

If the matrix that represents this system is not invertible, then we report the undetermined gas parameters, or the linearly dependent combinations of gas parameters. The exact math can be found under `/src/math.rs`. 

Otherwise, the User can expect to see all the values, expressed as Gas Usage per Microsecond, along with the running times and any outliers. 