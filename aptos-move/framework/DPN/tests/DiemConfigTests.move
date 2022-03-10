#[test_only]
module DiemFramework::OnChainConfigTests {
    use DiemFramework::Reconfiguration;
    use DiemFramework::Genesis;

    #[test(account = @0x1)]
    #[expected_failure(abort_code = 2)]
    fun init_before_genesis(account: signer) {
        Reconfiguration::initialize(&account);
    }

    #[test(account = @0x2, tc = @TreasuryCompliance, dr = @DiemRoot)]
    #[expected_failure(abort_code = 1)]
    fun invalid_address_init(account: signer, tc: signer, dr: signer) {
        Genesis::setup(&dr, &tc);
        Reconfiguration::initialize(&account);
    }

    #[test(tc = @TreasuryCompliance, dr = @DiemRoot)]
    #[expected_failure(abort_code = 261)]
    fun invalid_get(tc: signer, dr: signer) {
        Genesis::setup(&dr, &tc);
        Reconfiguration::get<u64>();
    }

    #[test(account = @0x1, tc = @TreasuryCompliance, dr = @DiemRoot)]
    #[expected_failure(abort_code = 516)]
    fun invalid_set(account: signer, tc: signer, dr: signer) {
        Genesis::setup(&dr, &tc);
        Reconfiguration::set_for_testing(&account, 0);
    }

    #[test(account = @0x1, tc = @TreasuryCompliance, dr = @DiemRoot)]
    #[expected_failure(abort_code = 2)]
    fun invalid_publish(account: signer, tc: signer, dr: signer) {
        Genesis::setup(&dr, &tc);
        Reconfiguration::publish_new_config_for_testing(&account, 0);
    }
}
