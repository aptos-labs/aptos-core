/// Provides a common place for exporting `create_signer` across the Velor Framework.
///
/// To use create_signer, add the module below, such that:
/// `friend velor_framework::friend_wants_create_signer`
/// where `friend_wants_create_signer` is the module that needs `create_signer`.
///
/// Note, that this is only available within the Velor Framework.
///
/// This exists to make auditing straight forward and to limit the need to depend
/// on account to have access to this.
module velor_framework::create_signer {
    friend velor_framework::account;
    friend velor_framework::velor_account;
    friend velor_framework::coin;
    friend velor_framework::fungible_asset;
    friend velor_framework::genesis;
    friend velor_framework::account_abstraction;
    friend velor_framework::multisig_account;
    friend velor_framework::object;
    friend velor_framework::permissioned_signer;
    friend velor_framework::transaction_validation;

    public(friend) native fun create_signer(addr: address): signer;
}
