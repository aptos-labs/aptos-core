# Running tests

```bash
cargo test -vv -p move-decompiler
```

# Generating missing -decompiled files

```bash
UPDATE_EXPECTED_OUTPUT=1 cargo test -p move-decompiler
```

or generate all -decompiled files


```bash
FORCE_UPDATE_EXPECTED_OUTPUT=1 cargo test -p move-decompiler
```
