#[test_only]
module aptos_framework::native_dispatch_token_tests {
    use aptos_framework::fungible_asset;
    use aptos_framework::native_dispatch_token;

    #[test(creator = @aptos_framework)]
    #[expected_failure(abort_code=0x10019, location=aptos_framework::fungible_asset)]
    fun test_native_dispatch_token(
        creator: &signer,
    ) {
        let (creator_ref, _) = fungible_asset::create_test_token(creator);
        fungible_asset::init_test_metadata(&creator_ref);

        native_dispatch_token::initialize(creator, &creator_ref);
    }
}
