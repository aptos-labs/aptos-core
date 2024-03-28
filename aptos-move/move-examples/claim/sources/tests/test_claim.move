#[test_only]
module claim::test_claim {
    use claim::claim_list;

    #[test(claim = @0xcafe)]
    fun test_claim(claim: &signer) {
        claim_list::init_for_test(claim);
        assert!(!claim_list::is_claimed(1), 0);
        claim_list::claim(1);
        assert!(claim_list::is_claimed(1), 1);
    }
}
