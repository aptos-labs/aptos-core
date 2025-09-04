spec velor_framework::chain_id {
    /// <high-level-req>
    /// No.: 1
    /// Requirement: During genesis, the ChainId resource should be created and moved under the Velor framework account
    /// with the specified chain id.
    /// Criticality: Medium
    /// Implementation: The chain_id::initialize function is responsible for generating the ChainId resource and then
    /// storing it under the velor_framework account.
    /// Enforcement: Formally verified via [high-level-req-1](initialize).
    ///
    /// No.: 2
    /// Requirement: The chain id can only be fetched if the chain id resource exists under the Velor
    /// framework account.
    /// Criticality: Low
    /// Implementation: The chain_id::get function fetches the chain id by borrowing the ChainId
    /// resource from the velor_framework account.
    /// Enforcement: Formally verified via [high-level-req-2](get).
    /// </high-level-req>
    ///
    spec module {
        pragma verify = true;
        pragma aborts_if_is_strict;
    }

    spec initialize {
        use std::signer;
        let addr = signer::address_of(velor_framework);
        aborts_if addr != @velor_framework;
        aborts_if exists<ChainId>(@velor_framework);
        /// [high-level-req-1]
        ensures exists<ChainId>(addr);
        ensures global<ChainId>(addr).id == id;
    }

    spec get {
        /// [high-level-req-2]
        aborts_if !exists<ChainId>(@velor_framework);
    }
}
