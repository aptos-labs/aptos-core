spec velor_framework::chain_status {
    /// <high-level-req>
    /// No.: 1
    /// Requirement: The end of genesis mark should persist throughout the entire life of the chain.
    /// Criticality: Medium
    /// Implementation: The Velor framework account should never drop the GenesisEndMarker resource.
    /// Enforcement: Audited that GenesisEndMarker is published at the end of genesis and never removed. Formally
    /// verified via [high-level-req-1](set_genesis_end) that GenesisEndMarker is published.
    ///
    /// No.: 2
    /// Requirement: The status of the chain should never be genesis and operating at the same time.
    /// Criticality: Low
    /// Implementation: The status of the chain is determined by the GenesisEndMarker resource.
    /// Enforcement: Formally verified via [high-level-req-2](global invariant).
    ///
    /// No.: 3
    /// Requirement: The status of the chain should only be changed once, from genesis to operating.
    /// Criticality: Low
    /// Implementation: Attempting to assign a resource type more than once will abort.
    /// Enforcement: Formally verified via [high-level-req-3](set_genesis_end).
    /// </high-level-req>
    ///
    spec module {
        pragma verify = true;
        pragma aborts_if_is_strict;
        /// [high-level-req-2]
        invariant is_genesis() == !is_operating();
    }

    spec set_genesis_end(velor_framework: &signer) {
        use std::signer;
        pragma verify = true;
        pragma delegate_invariants_to_caller;
        let addr = signer::address_of(velor_framework);
        aborts_if addr != @velor_framework;
        /// [high-level-req-3]
        aborts_if exists<GenesisEndMarker>(@velor_framework);
        /// [high-level-req-1]
        ensures global<GenesisEndMarker>(@velor_framework) == GenesisEndMarker {};
    }

    spec schema RequiresIsOperating {
        requires is_operating();
    }

    spec assert_operating {
        aborts_if !is_operating();
    }

    spec assert_genesis {
        aborts_if !is_genesis();
    }
}
