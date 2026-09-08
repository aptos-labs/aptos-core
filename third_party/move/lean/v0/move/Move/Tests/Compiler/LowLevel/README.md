# Low-level Move source tests

These fixtures exercise implementation APIs rather than the recommended
contract-authoring surface:

- `SourceVerification.lean` tests prophecy loans, stores, and relational
  semantics directly.
- `ModuleVerification.lean` tests manually constructed semantic contracts and
  the explicit `verify f by ...` escape hatch.
- `../../Negative/Lowering.lean` uses compatibility attributes and explicit
  declaration lists to check compiler diagnostics for invalid inputs.

Ordinary source examples belong under `../../Language` and use `move_module`,
`fun`, `Action`, declarative `spec`, and automatic `verify` notation.
