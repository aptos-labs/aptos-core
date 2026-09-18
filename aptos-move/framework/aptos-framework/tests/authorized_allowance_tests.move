#[test_only]
/// Tests for `authorized_allowance` against a fungible asset whose withdraw hook takes more out of
/// the store than it hands over, and which therefore could otherwise debit a signer by more than the
/// amount they authorized.
///
/// These also exercise the intended off-chain flow: the payload bytes come from the
/// `allowance_signing_message` view and are signed as opaque bytes, exactly as a wallet would.
module aptos_framework::authorized_allowance_tests {
    use aptos_framework::account;
    use aptos_framework::authorized_allowance;
    use aptos_framework::chain_id;
    use aptos_framework::deflation_token;
    use aptos_framework::fungible_asset::{Self, Metadata, TestToken};
    use aptos_framework::object::Object;
    use aptos_framework::primary_fungible_store;
    use aptos_framework::timestamp;
    use aptos_std::ed25519;
    use std::features;
    use std::signer;

    const CHAIN_ID: u8 = 4;
    const ED25519_SCHEME: u8 = 0;
    const RECIPIENT: address = @0xface;
    const EXPIRATION: u64 = 1000;
    const NONCE: u64 = 1;

    /// Mints 100 units of a deflationary asset to an account whose authentication key is the returned
    /// key pair. Every withdraw from that asset also burns 10% of the requested amount from the store.
    fun setup(creator: &signer): (ed25519::SecretKey, vector<u8>, address, Object<Metadata>) {
        features::change_feature_flags_for_testing(
            creator, vector[features::get_function_value_dispatch_feature()], vector[],
        );
        timestamp::set_time_has_started_for_testing(creator);
        chain_id::initialize_for_test(creator, CHAIN_ID);

        let (secret_key, public_key) = ed25519::generate_keys();
        let public_key_bytes = ed25519::validated_public_key_to_bytes(&public_key);
        let sender = account::create_account_from_ed25519_public_key(public_key_bytes);
        let sender_address = signer::address_of(&sender);

        let (creator_ref, token) = fungible_asset::create_test_token(creator);
        let (mint_ref, _transfer_ref, _burn_ref) =
            primary_fungible_store::init_test_metadata_with_primary_store_enabled(&creator_ref);
        let metadata = token.convert<TestToken, Metadata>();
        deflation_token::initialize(creator, &creator_ref);
        primary_fungible_store::mint(&mint_ref, sender_address, 100);

        (secret_key, public_key_bytes, sender_address, metadata)
    }

    /// Signs an allowance the way a wallet would: over the bytes the chain publishes for it.
    fun sign(
        secret_key: &ed25519::SecretKey,
        sender: address,
        metadata: Object<Metadata>,
        amount: u64,
    ): vector<u8> {
        let message = authorized_allowance::allowance_signing_message(
            sender, RECIPIENT, metadata, amount, EXPIRATION, NONCE,
        );
        ed25519::signature_to_bytes(&ed25519::sign_arbitrary_bytes(secret_key, message))
    }

    #[test(creator = @aptos_framework)]
    /// An allowance of 55 covers a transfer of 50 plus the 5 the withdraw hook burns, and is left
    /// exactly exhausted. The sender loses 55 in total, which is what they signed for.
    fun test_withdrawal_fee_is_charged_to_the_allowance(creator: &signer) {
        let (secret_key, public_key, sender, metadata) = setup(creator);
        let signature = sign(&secret_key, sender, metadata, 55);

        authorized_allowance::redeem_with_allowance(
            sender, RECIPIENT, metadata, 55, EXPIRATION, NONCE, ED25519_SCHEME, public_key,
            signature, 50,
        );

        assert!(primary_fungible_store::balance(sender, metadata) == 45, 1);
        assert!(primary_fungible_store::balance(RECIPIENT, metadata) == 50, 2);
        assert!(authorized_allowance::redeemed_amount(sender, NONCE) == 55, 3);
    }

    #[test(creator = @aptos_framework)]
    #[expected_failure(abort_code = 0x10003, location = aptos_framework::authorized_allowance)]
    /// Redeeming an allowance of 50 in full would debit 55 once the withdraw hook takes its cut. The
    /// redemption is rejected rather than letting the sender lose more than they signed for.
    fun test_debit_beyond_the_signed_amount_is_rejected(creator: &signer) {
        let (secret_key, public_key, sender, metadata) = setup(creator);
        let signature = sign(&secret_key, sender, metadata, 50);

        authorized_allowance::redeem_with_allowance(
            sender, RECIPIENT, metadata, 50, EXPIRATION, NONCE, ED25519_SCHEME, public_key,
            signature, 50,
        );
    }

    #[test(creator = @aptos_framework)]
    #[expected_failure(abort_code = 0x10003, location = aptos_framework::authorized_allowance)]
    /// Splitting a redemption into several pulls pays the withdrawal fee again each time. A later pull
    /// whose transfer still fits in the allowance is rejected once its fee no longer does.
    fun test_fee_on_a_later_pull_cannot_overrun_the_allowance(creator: &signer) {
        let (secret_key, public_key, sender, metadata) = setup(creator);
        let signature = sign(&secret_key, sender, metadata, 45);

        // A pull of 20 debits 22, using up 22 of the 45.
        authorized_allowance::redeem_with_allowance(
            sender, RECIPIENT, metadata, 45, EXPIRATION, NONCE, ED25519_SCHEME, public_key,
            signature, 20,
        );
        assert!(primary_fungible_store::balance(sender, metadata) == 78, 1);
        assert!(authorized_allowance::redeemed_amount(sender, NONCE) == 22, 2);

        // Transferring 22 more would fit in the 23 left, but the 2 it burns on the way out would not.
        authorized_allowance::redeem_with_allowance(
            sender, RECIPIENT, metadata, 45, EXPIRATION, NONCE, ED25519_SCHEME, public_key,
            signature, 22,
        );
    }
}
