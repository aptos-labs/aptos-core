# Ruby policy tests

Run these tests with Ruby 4.0.6 and the locked test bundle:

```sh
export BUNDLE_GEMFILE="$PWD/.github/workflows/tests/Gemfile"
export BUNDLE_PATH="${TMPDIR:-/tmp}/aptos-ruby-contract-gems"
export BUNDLE_FROZEN=true
bundle _4.0.16_ install
bundle _4.0.16_ exec ruby .github/workflows/tests/pr_ci_policy/test_policy_pr_sources.rb
```

`RUBY_PROPERTY_PROFILE=ci` is the default. Each property checks its deterministic
corpus, then 100 generated cases. `RUBY_PROPERTY_PROFILE=explore` checks 1,000
generated cases. Both profiles use seed `20260930` unless `RUBY_PROPERTY_SEED`
sets an unsigned 64-bit integer. Unknown profiles and invalid seeds are errors.

```sh
RUBY_PROPERTY_PROFILE=explore RUBY_PROPERTY_SEED=42 \
  bundle _4.0.16_ exec ruby .github/workflows/tests/pr_ci_policy/test_policy_pr_sources.rb
```

Each property has a separate random stream derived from its stable name and
seed. Failure messages include both seeds, the original input, and the reduced
input. Replay the named test with the reported `RUBY_PROPERTY_SEED` and profile.
Generators encode bounded choices as integer tuples so shrinking preserves their
input grammar. Nesting stays within four levels; generated workflows have at
most four jobs.

The runner properties cover pagination, duplicate and truncated listings,
protected path lookalikes, and rename directions. Status/presence rules and
count boundaries use exhaustive tables. Protected path expectations use literal
test-owned rules; separate contracts check the real policy manifest.

The privileged workflow tests reuse this helper for SHA validation, image tags,
CR/LF injection rejection, and Forge namespace derivation. They execute the
actual shell scripts and compare results with test-owned rules. Generated
strings are bounded; the original manifest variants retain bake-script checks.

Finite security rules are checked exhaustively. Generated combinations extend
those checks. Keep named bypass regressions, exact source/API/entrypoint checks,
workflow contracts, and the secret-scanning performance test. Preserve every
replaced input as a deterministic case. Remove a case only when its replacement
detects the same deliberate fault. Production behavior stays unchanged.
