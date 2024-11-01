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

## V1 Test Migration

Tests from the v1 compiler test suite are incrementally ported to the v2 tree. Every single test
should be vetted that the v2 compiler delivers the correct (equivalent) result before it is ported
into v2. Exception to this rule should be marked with a github issue.

There are two files which represent the current state of test migration:

- [`v1.unmatched`](./v1.unmatched): this contains a list of the tests which currently have no
  matching equivalent in the v2 test suite.
- [`v1.matched`](./v1.matched): this contains a list of the pairs of matched test
  expectation (`.exp`) files, for further processing

To update those files run the script [`update_v1_diff.sh`](./update_v1_diff.sh). To see the rules
how those lists are produced, see the code at [`tools/testdiff`](../tools/testdiff).

In order to migrate a test such that the tool can keep track of it, ensure that you place it in a
similar named parent directory (anywhere in the v2 test tree). For example, for a
test `move-check/x/y.move`, ensure the test can be found somewhere at `x/y.move` in the v2 tree.

### About v1/v2 test comparison

Notice that test comparison is a tedious manual process for the following reasons:

- The errors reported by v1 and v2 have a rather different structure (different text, more or
  less additional notes and labels, etc.) . Also the order in which errors are generated is
  different. A textual diff is therefore basically useless. Rather the manual comparison entails: (
  a) going one-by-one over each error in the v1 exp file. and find the error at the same line numer
  in the v2 .exp file (b) deciding whether the errors are compatible (c) reasoning whether if one
  error is missed, it is semantically represented by a different one (the same logical error can
  reported at different locations in the file, an artifact of type inference) (d) checking out all
  v2 errors whether non are obsolete.

- v1 and v2 have different phase structure and order of analysis. For example, many files in the
  v1 test suite do not fully compile, and don't need to, because they hit the tested blocking error
  before a secondary one is produced. But then in the other compiler (either v1 or v2), the
  secondary error may become the primary one, masking the tested error. For example, in v1 reference
  analysis errors mask ability analysis errors, but in v2 its the other way around. This leads to
  that test sources needed to be modified.

- In the case of reference safety comparison becomes even more difficult because the semantics of
  those both is different. For example, v1 enforces borrow rules on x in statement like x; (refer to
  x and forget its value). Not so v2: one must actually use the variable (as in *x;). Those
  differences have been discussed in multiple team meetings and are by design.

Because of this it is expensive to do test comparison, and essential that we follow the migration
process as outlined above. Specifically, do _not_ bulk copy tests into the v2 tree without
manual auditing them, and do _not_ fork tests, even if they are modified, so the relation
between v1/v2 tests is maintained.