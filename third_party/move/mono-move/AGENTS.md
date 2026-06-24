# mono-move

Conventions for every crate under `third_party/move/mono-move/`. Crate-specific
docs live in each crate's `AGENTS.md`; design rationale in `docs/`.

## TODO labels

Every inline TODO is `// TODO(<group>): message`, where `<group>` is one or more
(comma-separated, most relevant first) of:

| group | covers |
|---|---|
| correctness | wrong results, unsoundness, aliasing/security, legacy-VM parity |
| completeness | unimplemented functionality, `todo!()` gaps, future-feature constraints |
| metering | gas charging and recursion/size/cache-DoS bounds |
| perf | speed-only optimizations |
| cleanup | refactor, rename, error-type unification, design questions, docs |
| testing | test infra and missing tests |

No roadmap `TODO*.md` files — items live inline at the owning code site. To
verify (prints nothing if clean):

```bash
grep -rn "TODO" --include='*.rs' third_party/move/mono-move \
  | grep -v 'todo!\|unimplemented!\|unreachable!' \
  | grep -vE 'TODO\(((correctness|completeness|metering|perf|cleanup|testing)(, )?)+\):'
```
