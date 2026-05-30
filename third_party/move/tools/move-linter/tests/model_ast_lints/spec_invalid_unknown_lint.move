// @checks=-not_a_real_lint
// Verifies that referencing an unknown lint name in `--checks=` produces a
// `Lint configuration error` instead of silently running anything.

module 0xc0ffee::m {
    public fun foo(): u64 { 1 }
}
