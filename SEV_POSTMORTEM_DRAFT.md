# SEV Review: Combined Postmortem for #14109, #14154, #14156

**Date:** 2026-03-04
**Prepared by:** Move VM Team
**Review Date:** 2026-03-05

---

## Summary

This is a combined SEV review for three incidents. They were reported at around the same time, and were bundled as a single fix. All three SEVs had the potential to cause the network to crash or stall, even though no real disruption happened.

### [SEV 2] #14109: Unbounded Recursive Value::Display Formatting on Closure Serialization Failure

**Root Cause:** When closure serialization fails (e.g., due to depth limits or invalid data), the error handling path attempted to format the `Value` for logging/debugging purposes. The `Display` implementation for `Value` (in `values_impl.rs:4460`) delegates to the closure's Display implementation, which in turn formats the captured values. If a captured value contains another closure (or if the error itself triggers formatting), this creates an unbounded recursive call chain.

The lack of depth limits on closure chains meant that deeply nested closures could be constructed, and when serialization failed, the Display formatting would recursively expand without termination, leading to stack overflow or memory exhaustion.

**Fix:** Added `ClosureDepthCheck` timed feature (activated Feb 2-9, 2026) that enforces depth checking for captured values when packing closures, preventing deeply nested closure chains that could cause stack overflow during serialization or display formatting.

### [SEV 2] #14154: TypeTag Conversion Panic in hash_to

**Root Cause:** The `hash_to_internal` native function in `hash_to_structure.rs` performs a `type_to_type_tag` conversion on type arguments. Prior to the fix, if the type argument was excessively complex (deeply nested generic types), the conversion could either:
1. Panic due to hitting internal limits
2. Run indefinitely attempting to expand a complex type

The crypto algebra natives (`hash_to_internal`) did not gracefully handle type conversion failures, instead allowing panics to propagate up.

**Fix:** Added proper error handling with `E_TYPE_TO_TYPE_TAG_CONVERSION_FAILED` error code (see `hash_to_structure.rs:31`). The fix converts the panic into a safe Move abort with error code `0x0B_0063` (`std::error::internal(99)`), along with `FixCryptoAlgebraNativesResultHandling` timed feature to ensure consistent behavior.

### [SEV 2] #14156: Loss of Network Liveness due to Infinite Loop in Quorum Store Batch (Move VM Part)

**Root Cause:** The Move VM part of this issue relates to gas metering inconsistencies during transaction execution. Certain transaction patterns involving complex type instantiations or deeply nested structures could bypass proper gas charging, allowing loops to execute without consuming expected gas. This was exacerbated by missing or incorrect bounds in the production verifier config.

The gas metering logic for certain operations (particularly those involving type resolution and instantiation checking) had gaps that could be exploited to create transactions that ran much longer than their gas limit should have allowed.

**Fix:**
- Enabled `EnableStrictBoundsInProdConfig` (activated Feb 25-27, 2026) to add strict bounds for struct definitions (200→1100), struct variants (64→127), fields per struct (64), function definitions (1000), and basic blocks in scripts (1024).
- Enabled `RevisedBoundsInProdConfig` (activated Mar 3-5, 2026) to revise bounds that were found to be either too restrictive or needed adjustment after the initial strict bounds deployment.
- Note: The consensus/quorum store portion will be covered by Balaji separately.

---

## Impact

### Internal Teams
- **Move VM Team:** Emergency triage and fix development required rapid coordination
- **Security Team:** Involved in assessing exploitability and impact assessment
- **Node Operators:** Required coordinated deployment of fixes via cherry-pick process

### External Ecosystem
- **No observed network disruption:** All three vulnerabilities were identified and patched before any malicious exploitation
- **Potential impact if exploited:**
  - Node crashes due to stack overflow (#14109)
  - Node crashes due to unhandled panic (#14154)
  - Network stall due to validators unable to process transactions (#14156)

### Severity Justification
All three issues were classified as SEV-2 because:
- They could cause network-wide impact (crash or stall validators)
- They were not being actively exploited
- Fixes could be deployed through normal release process with coordinated timing

---

## Timeline

| Time | Event |
|------|-------|
| TBD | Initial report of #14109 |
| TBD | Initial report of #14154 |
| TBD | Initial report of #14156 |
| TBD | Root cause analysis completed |
| TBD | Fix development and review |
| 2026-02-02 22:00 PT | `ClosureDepthCheck` activated on testnet |
| 2026-02-02 22:00 PT | `FixCryptoAlgebraNativesResultHandling` activated on testnet |
| 2026-02-09 12:00 PT | Both features activated on mainnet |
| 2026-02-25 10:00 PT | `EnableStrictBoundsInProdConfig` activated on testnet |
| 2026-02-27 10:00 PT | `EnableStrictBoundsInProdConfig` activated on mainnet |
| 2026-03-03 21:00 PT | `RevisedBoundsInProdConfig` activated on testnet |
| 2026-03-05 10:00 PT | `RevisedBoundsInProdConfig` scheduled for mainnet |

---

## Learnings

### Action Items

1. **[P0] Add fuzz testing for closure serialization paths**
   - Owner: Move VM Team
   - Due: TBD
   - Add fuzz tests that exercise closure packing/unpacking with various depth levels

2. **[P0] Audit all native functions for panic paths**
   - Owner: Move VM Team
   - Due: TBD
   - Ensure all native functions return SafeNativeError instead of panicking

3. **[P1] Add depth limits documentation**
   - Owner: Move VM Team
   - Due: TBD
   - Document the depth limits and their rationale in the Move VM design docs

4. **[P1] Improve gas metering test coverage**
   - Owner: Execution Team
   - Due: TBD
   - Add tests specifically targeting gas consumption for complex type operations

5. **[P2] Consider adding runtime recursion guards to Display implementations**
   - Owner: Move VM Team
   - Due: TBD
   - Add recursion depth tracking to prevent infinite Display loops even if other guards fail

---

## What Went Well

1. **Early detection:** All three issues were identified through internal review/testing before any mainnet exploitation
2. **Coordinated deployment:** The timed feature mechanism allowed controlled rollout first to testnet, then mainnet
3. **Bundled fix approach:** Combining related fixes reduced deployment complexity and coordination overhead
4. **Cherry-pick process:** Despite some tricky issues during cherry-picking (see thread), the team successfully backported the fixes
5. **Clear communication:** Deployment coordination in #oncall-deployments kept all stakeholders informed

---

## What Didn't Go So Well

1. **Cherry-pick complexity:** The gas fix for #14156 had dependency issues during cherry-pick that required manual resolution
   - See: https://aptos-org.slack.com/archives/C0AAV414GKD/p1771379380468059

2. **Multiple timed features required:** The need for both `EnableStrictBoundsInProdConfig` and later `RevisedBoundsInProdConfig` indicates the initial bounds were not fully validated
   - The first set of bounds was too restrictive for some legitimate use cases
   - Required a follow-up deployment just days later

3. **Gap between testnet and mainnet activation:** The week-long gap between testnet and mainnet for some features left a window where mainnet was known to be vulnerable

4. **Documentation gaps:** The relationship between these three SEVs and how they could be exploited together was not immediately clear from initial reports

---

## References

- PR #18887: [vm] Fixing issues with bounds and recursion
- Timed Features: `types/src/on_chain_config/timed_features.rs`
- Closure Display Implementation: `third_party/move/move-vm/types/src/values/values_impl.rs:4460`
- Hash-to-structure natives: `aptos-move/framework/natives/src/cryptography/algebra/hash_to_structure.rs`
- Production config: `aptos-move/aptos-vm-environment/src/prod_configs.rs`
