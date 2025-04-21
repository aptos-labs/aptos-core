# Compiler Unit Tests

This directory contains the unit tests for the compiler. For end-to-end tests, see
the [`transactional_tests`](../transactional-tests).

## Test Organization

Unit tests are organized along phases of the compiler. Ideally a unit test is focused on the
particular aspect this phase implements.

The compiler phases are organized as follows:

- Building of the `GlobalEnv`, which includes type checking and inference of the program. Related
  tests are in [`checking`](./checking).
- Transformation of the GlobalEnv (e.g. inlining)
- Generation of stack-less bytecode, tests are in [`bytecode-generator`](./bytecode-generator).
- Any number of bytecode level checkers or transformers (currently `live-var` and `reference-safety`
  and `visibility-checker`)
- The last and final phase of the file format generator, tests are
  in [`file-format-generator`](./file_format_generator)
