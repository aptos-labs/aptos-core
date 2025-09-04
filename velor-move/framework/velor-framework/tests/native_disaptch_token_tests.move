#[test_only]
module velor_framework::native_dispatch_token_tests {
    use velor_framework::fungible_asset;
    use 0xcafe::native_dispatch_token;

    #[test(creator = @0xcafe)]
    #[expected_failure(abort_code=0x10019, location=velor_framework::fungible_asset)]
    fun test_native_dispatch_token(
        creator: &signer,
    ) {
        let (creator_ref, _) = fungible_asset::create_test_token(creator);
        fungible_asset::init_test_metadata(&creator_ref);

        native_dispatch_token::initialize(creator, &creator_ref);
    }
}
