# Compiler Unit Tests

This directory contains the unit tests for the compiler. For end-to-end tests, see the [`transactional_tests`](../transactional-tests).

## Test Organization

Unit tests are organized along phases of the compiler. Ideally a unit test is focused on the particular aspect this phase implements. 

The compiler phases are organized as follows:

- Building of the `GlobalEnv`, which includes type checking and inference of the program. Related tests are in [`checking`](./checking).
- Transformation of the GlobalEnv (e.g. inlining)
- Generation of stack-less bytecode, tests are in [`bytecode-generator`](./bytecode-generator).
- Any number of bytecode level checkers or transformers (currently `live-var` and `reference-safety` and `visibility-checker`)
- The last and final phase of the file format generator, tests are in [`file-format-generator`](./file_format_generator)


## V1 Test Migration

Tests from the v1 compiler test suite are incrementally ported to the v2 tree. Every single test should be vetted that the v2 compiler delivers the correct (equivalent) result before it is ported into v2. Exception to this rule should be marked with a github issue.

There are two files which represent the current state of test migration:

- [`v1.unmatched`](./v1.unmatched): this contains a list of the tests which currently have no matching equivalent in the v2 test suite.
- [`v1.matched`](./v1.matched): this contains a list of the pairs of matched test expectation (`.exp`) files, for further processing
 
To update those files run the script [`update_v1_diff.sh`](./update_v1_diff.sh). To see the rules how those lists are produced, see the code at [`tools/testdiff`](../tools/testdiff).

In order to migrate a test such that the tool can keep track of it, ensure that you place it in a similar named parent directory (anywhere in the v2 test tree). For example, for a test `move-check/x/y.move`, ensure the test can be found somewhere at `x/y.move` in the v2 tree.
