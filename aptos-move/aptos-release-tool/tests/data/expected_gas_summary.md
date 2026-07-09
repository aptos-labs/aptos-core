# Gas Schedule Changes

Gas feature version: 30 -> 31

- [ ] I have reviewed the gas schedule changes below.

## Changes

| change   | parameter             |       old |        new | sign-off |
| -------- | --------------------- | --------: | ---------: | -------- |
| modified | instr.add             |        50 |         65 |          |
| added    | instr.mul             |         / |         90 |          |
| removed  | instr.sub             |        80 |          / |          |
| modified | txn.max_execution_gas | 920000000 | 1000000000 | [ ]      |
