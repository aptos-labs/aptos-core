#[test_only]
module DiemFramework::BlockTests {
    use DiemFramework::Genesis;
    use DiemFramework::Block;

    // TODO: the error code doesn't seem correct, juding by the name of the test.
    #[test(tc = @TreasuryCompliance, dr = @DiemRoot, account = @0x100)]
    #[expected_failure(abort_code = 1)]
    fun invalid_initialization_address(account: signer, tc: signer, dr: signer) {
        Genesis::setup(&dr, &tc);
        Block::initialize_block_metadata(&account);
    }
}
