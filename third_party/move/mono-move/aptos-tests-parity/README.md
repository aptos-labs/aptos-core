# Aptos test parity for MonoMove

`MONO_MOVE_ENV=1` adds `ENABLE_MONO_MOVE` to `FeatureFlag::default_features()`, so
every genesis built in the test process enables MonoMove. The
`mono-move-tests-parity` workflow runs the normal suites that way. No test is
modified, and no test needs to know MonoMove exists.

## failing.toml and skipped.toml

Between them, the tests that do not pass under MonoMove. Everything else must pass,
so a new test is covered as soon as it is written.

```toml
[[test]]
name = "e2e-move-tests::e2e_move_tests$tests::infinite_loop::infinite_loop_aborts"
note = "Relies on gas exhaustion to terminate; hangs instead of aborting."
```

`name` is the nextest test ID, so one file covers every crate. `note` is optional
free text. A test must not appear in both files.

`failing.toml` holds tests that cannot pass on MonoMove today but should in the
future. Drop a test from the list once it passes. This list should reach zero;
`--update` maintains it.

`skipped.toml` excludes the test from the run, and is edited by hand. Some entries
use a feature MonoMove will never support, such as publishing mid-block. Others may
hang: MonoMove has no metering, so a test that exercises an infinite loop never
returns, and those go once MonoMove is production-ready. Audit this list entry by
entry rather than aiming it at zero.

## Running

```bash
PARITY=third_party/move/mono-move/aptos-tests-parity/check_parity.py

MONO_MOVE_ENV=1 RUST_MIN_STACK=1073741824 NEXTEST_EXPERIMENTAL_LIBTEST_JSON=1 \
  cargo nextest run --profile ci --workspace \
    --no-fail-fast --message-format libtest-json \
    -E "$(python3 $PARITY --skip-filter)" > results.json
```

`--skip-filter` turns `skipped.toml` into a nextest filterset.

The variable only matters where a test builds genesis: unit tests that construct a
`FakeExecutor`, and every smoke test, since `LocalSwarm` builds genesis in the test
process. A test that passes `initial_features_override` pins its own feature set and
opts out. Forge is out of reach, because it runs prebuilt images in Kubernetes.

## Checking

```bash
python3 $PARITY --check results.json
```

Fails if a test broke and is in neither file, if a test in `failing.toml` passed, or
if a test in `skipped.toml` ran anyway.

Renaming a test changes its ID, so `--check` reports it twice: the old name as a
listed test that did not fail, the new name as an unlisted failure. `--update` fixes
both.

## Updating

```bash
python3 $PARITY --update results.json
```

Rewrites `failing.toml`: adds new failures, drops entries that now pass. It never
touches `skipped.toml`, because it cannot tell a hang from a test that was slow on a
loaded machine.
