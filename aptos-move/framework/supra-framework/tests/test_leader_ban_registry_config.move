#[test_only]
module supra_framework::test_leader_ban_registry_config {
    use std::bcs;
    use std::vector;
    use supra_framework::config_buffer;
    use supra_framework::leader_ban_registry_config;

    #[test(sender = @0xdead)]
    #[expected_failure(abort_code = 0x50003, location = supra_framework::system_addresses)]
    // signer is not valid
    fun test_signer(sender: &signer) {
        let ban_registry_param =
            leader_ban_registry_config::get_test_ban_registry_params_v0();
        let config_bytes = bcs::to_bytes(&ban_registry_param);
        leader_ban_registry_config::initialize(sender, config_bytes);
    }

    #[test(sender = @supra_framework)]
    #[
        expected_failure(
            abort_code = 0x10001, location = supra_framework::leader_ban_registry_config
        )
    ]
    // Invalid config bytes
    fun test_empty_config_value(sender: &signer) {
        let config_bytes = vector::empty();
        leader_ban_registry_config::initialize(sender, config_bytes);
    }

    #[test(sender = @supra_framework)]
    #[
        expected_failure(
            abort_code = 0x10003, location = supra_framework::leader_ban_registry_config
        )
    ]
    // Invalid version bytes
    fun test_invalid_config_value(sender: &signer) {
        let ban_registry_param =
            leader_ban_registry_config::get_test_ban_registry_params_v0();
        let config_bytes = bcs::to_bytes(&ban_registry_param);
        vector::push_back(&mut config_bytes, 0);
        leader_ban_registry_config::initialize(sender, config_bytes);
    }

    public fun init_ban_registry_params(sender: &signer) {
        let ban_registry_param =
            leader_ban_registry_config::get_test_ban_registry_params_v0();
        let config_bytes = bcs::to_bytes(&ban_registry_param);
        leader_ban_registry_config::initialize(sender, config_bytes);
    }

    #[test(sender = @supra_framework)]
    public fun test_init_config_value(sender: &signer) {
        init_ban_registry_params(sender);
        assert!(leader_ban_registry_config::check_ban_registry_params_exist(sender), 1);
        assert!(
            leader_ban_registry_config::check_ban_registry_params_v0_exist(sender), 2
        );
    }

    #[test(sender = @supra_framework)]
    #[
        expected_failure(
            abort_code = 0x80004, location = supra_framework::leader_ban_registry_config
        )
    ]
    // Already initialised
    fun test_init_config_value_reinit(sender: &signer) {
        init_ban_registry_params(sender);
        init_ban_registry_params(sender);
    }

    #[test(sender = @supra_framework)]
    fun test_on_epoch_change(sender: &signer) {
        config_buffer::initialize(sender);
        assert!(
            !leader_ban_registry_config::check_ban_registry_params_exist(sender), 1
        );
        let ban_registry_param =
            leader_ban_registry_config::get_test_ban_registry_params_v0();
        let config_bytes = bcs::to_bytes(&ban_registry_param);
        leader_ban_registry_config::set_for_next_epoch(sender, config_bytes, 0);
        leader_ban_registry_config::on_new_epoch(sender);
        assert!(leader_ban_registry_config::check_ban_registry_params_exist(sender), 1);
        assert!(
            leader_ban_registry_config::check_ban_registry_params_v0_exist(sender), 2
        );

        let updated_initial_e_denied = 3;
        let updated_max_e_denied = 8;
        let updated_minimum_u_proposers = 6;
        let updated_probation_elections = 2;

        let ban_registry_param =
            leader_ban_registry_config::get_custom_ban_registry_params_v0(
                updated_initial_e_denied,
                updated_max_e_denied,
                updated_minimum_u_proposers,
                updated_probation_elections
            );
        let config_bytes = bcs::to_bytes(&ban_registry_param);
        leader_ban_registry_config::set_for_next_epoch(sender, config_bytes, 0);
        leader_ban_registry_config::on_new_epoch(sender);
        assert!(
            leader_ban_registry_config::get_initial_elections_denied()
                == updated_initial_e_denied,
            3
        );
        assert!(
            leader_ban_registry_config::get_max_elections_denied()
                == updated_max_e_denied,
            4
        );
        assert!(
            leader_ban_registry_config::get_minimum_unbanned_proposers()
                == updated_minimum_u_proposers,
            5
        );
        assert!(
            leader_ban_registry_config::get_probation_elections()
                == updated_probation_elections,
            6
        );
    }
}
