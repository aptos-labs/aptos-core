spec velor_framework::create_signer {
    use std::signer;

    /// <high-level-req>
    /// No.: 1
    /// Requirement: Obtaining a signer for an arbitrary account should only be available within the Velor Framework.
    /// Criticality: Critical
    /// Implementation: The create_signer::create_signer function only allows friend modules to retrieve the signer for
    /// an arbitrarily address.
    /// Enforcement: Enforced through function visibility.
    ///
    /// No.: 2
    /// Requirement: The account owner should have the ability to create a signer for their account.
    /// Criticality: Medium
    /// Implementation: Before an Account resource is created, a signer is created for the specified new_address, and
    /// later, the Account resource is assigned to this signer.
    /// Enforcement: Enforced by the [https://github.com/velor-chain/velor-core/blob/main/third_party/move/move-vm/types/src/values/values_impl.rs#L1129](move vm).
    ///
    /// No.: 3
    /// Requirement: An account should only be able to create a signer for another account if that account has granted it
    /// signing capabilities.
    /// Criticality: Critical
    /// Implementation: The Account resource holds a signer_capability_offer field which allows the owner to share the
    /// signer capability with other accounts.
    /// Enforcement: Formally verified via [account::high-level-spec-3](AccountContainsAddr).
    ///
    /// No.: 4
    /// Requirement: A signer should be returned for addresses that are not registered as accounts.
    /// Criticality: Low
    /// Implementation: The signer is just a struct that wraps an address, allows for non-accounts to have a signer.
    /// Enforcement: Formally verified via [0x1_create_signer_create_signer](create_signer).
    /// </high-level-req>
    ///
    spec module {
        pragma verify = true;
        pragma aborts_if_is_strict;
    }

    /// Convert address to singer and return.
    spec create_signer(addr: address): signer {
        pragma opaque;
        aborts_if [abstract] false;
        ensures [abstract] signer::address_of(result) == addr;
        ensures [abstract] result == spec_create_signer(addr);
    }

    spec fun spec_create_signer(addr: address): signer;
}
