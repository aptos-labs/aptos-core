#[test_only]
module velor_framework::permissioned_signer_tests {
    use std::bcs;
    use std::features;
    use velor_framework::account::create_signer_for_test;
    use velor_framework::permissioned_signer;
    use velor_framework::timestamp;
    use std::option;
    use std::signer;

    struct OnePermission has copy, drop, store {}

    struct AddressPermission has copy, drop, store {
        addr: address
    }

    #[test(creator = @0xcafe)]
    fun test_permission_e2e(creator: &signer) {
        let velor_framework = create_signer_for_test(@0x1);
        timestamp::set_time_has_started_for_testing(&velor_framework);

        let perm_handle = permissioned_signer::create_permissioned_handle(creator);
        let perm_signer = permissioned_signer::signer_from_permissioned_handle(&perm_handle);

        assert!(permissioned_signer::is_permissioned_signer(&perm_signer), 1);
        assert!(!permissioned_signer::is_permissioned_signer(creator), 1);
        assert!(signer::address_of(&perm_signer) == signer::address_of(creator), 1);

        permissioned_signer::authorize_increase(creator, &perm_signer, 100, OnePermission {});
        assert!(
            permissioned_signer::capacity(&perm_signer, OnePermission {})
                == option::some(100),
            1
        );

        assert!(
            permissioned_signer::check_permission_consume(
                &perm_signer, 10, OnePermission {}
            ),
            1
        );
        assert!(
            permissioned_signer::capacity(&perm_signer, OnePermission {})
                == option::some(90),
            1
        );

        permissioned_signer::authorize_increase(
            creator,
            &perm_signer,
            5,
            AddressPermission { addr: @0x1 }
        );

        assert!(
            permissioned_signer::capacity(&perm_signer, AddressPermission { addr: @0x1 })
                == option::some(5),
            1
        );
        assert!(
            permissioned_signer::capacity(&perm_signer, AddressPermission { addr: @0x2 })
                == option::none(),
            1
        );

        // Not enough capacity, check permission should return false
        assert!(
            !permissioned_signer::check_permission_consume(
                &perm_signer, 10, AddressPermission { addr: @0x1 }
            ),
            1
        );

        assert!(
            permissioned_signer::check_permission_consume(
                &perm_signer, 5, AddressPermission { addr: @0x1 }
            ),
            1
        );

        // Remaining capacity is 0, should be viewed as non-exist.
        assert!(
            !permissioned_signer::check_permission_exists(
                &perm_signer, AddressPermission { addr: @0x1 }
            ),
            1
        );

        permissioned_signer::revoke_permission(&perm_signer, OnePermission {});
        assert!(
            permissioned_signer::capacity(&perm_signer, OnePermission {})
                == option::none(),
            1
        );

        permissioned_signer::destroy_permissioned_handle(perm_handle);
    }

    #[test(creator = @0xcafe)]
    fun test_storable_permission_e2e(creator: &signer) {
        let velor_framework = create_signer_for_test(@0x1);
        timestamp::set_time_has_started_for_testing(&velor_framework);

        let perm_handle =
            permissioned_signer::create_storable_permissioned_handle(creator, 60);
        let perm_signer =
            permissioned_signer::signer_from_storable_permissioned_handle(&perm_handle);

        assert!(permissioned_signer::is_permissioned_signer(&perm_signer), 1);
        assert!(!permissioned_signer::is_permissioned_signer(creator), 1);
        assert!(signer::address_of(&perm_signer) == signer::address_of(creator), 1);

        permissioned_signer::authorize_increase(creator, &perm_signer, 100, OnePermission {});
        assert!(
            permissioned_signer::capacity(&perm_signer, OnePermission {})
                == option::some(100),
            1
        );

        assert!(
            permissioned_signer::check_permission_consume(
                &perm_signer, 10, OnePermission {}
            ),
            1
        );
        assert!(
            permissioned_signer::capacity(&perm_signer, OnePermission {})
                == option::some(90),
            1
        );

        permissioned_signer::authorize_increase(
            creator,
            &perm_signer,
            5,
            AddressPermission { addr: @0x1 }
        );

        assert!(
            permissioned_signer::capacity(&perm_signer, AddressPermission { addr: @0x1 })
                == option::some(5),
            1
        );
        assert!(
            permissioned_signer::capacity(&perm_signer, AddressPermission { addr: @0x2 })
                == option::none(),
            1
        );

        // Not enough capacity, check permission should return false
        assert!(
            !permissioned_signer::check_permission_consume(
                &perm_signer, 10, AddressPermission { addr: @0x1 }
            ),
            1
        );

        permissioned_signer::revoke_permission(&perm_signer, OnePermission {});
        assert!(
            permissioned_signer::capacity(&perm_signer, OnePermission {})
                == option::none(),
            1
        );

        permissioned_signer::destroy_storable_permissioned_handle(perm_handle);
    }

    #[test(creator = @0xcafe)]
    #[expected_failure(
        abort_code = 0x50005, location = velor_framework::permissioned_signer
    )]
    fun test_permission_expiration(creator: &signer) {
        let velor_framework = create_signer_for_test(@0x1);
        timestamp::set_time_has_started_for_testing(&velor_framework);

        let perm_handle =
            permissioned_signer::create_storable_permissioned_handle(creator, 60);
        let _perm_signer =
            permissioned_signer::signer_from_storable_permissioned_handle(&perm_handle);

        timestamp::fast_forward_seconds(60);
        let _perm_signer =
            permissioned_signer::signer_from_storable_permissioned_handle(&perm_handle);

        permissioned_signer::destroy_storable_permissioned_handle(perm_handle);
    }

    // invalid authorization
    // 1. master signer is a permissioned signer
    // 2. permissioned signer is a master signer
    // 3. permissioned and main signer address mismatch
    #[test(creator = @0xcafe)]
    #[expected_failure(
        abort_code = 0x50002, location = velor_framework::permissioned_signer
    )]
    fun test_auth_1(creator: &signer) {
        let velor_framework = create_signer_for_test(@0x1);
        timestamp::set_time_has_started_for_testing(&velor_framework);

        let perm_handle = permissioned_signer::create_permissioned_handle(creator);
        let perm_signer = permissioned_signer::signer_from_permissioned_handle(&perm_handle);

        permissioned_signer::authorize_increase(
            &perm_signer,
            &perm_signer,
            100,
            OnePermission {}
        );
        permissioned_signer::destroy_permissioned_handle(perm_handle);
    }

    #[test(creator = @0xcafe)]
    #[expected_failure(
        abort_code = 0x50002, location = velor_framework::permissioned_signer
    )]
    fun test_auth_2(creator: &signer) {
        permissioned_signer::authorize_increase(creator, creator, 100, OnePermission {});
    }

    #[test(creator = @0xcafe, creator2 = @0xbeef)]
    #[expected_failure(
        abort_code = 0x50002, location = velor_framework::permissioned_signer
    )]
    fun test_auth_3(creator: &signer, creator2: &signer) {
        let velor_framework = create_signer_for_test(@0x1);
        timestamp::set_time_has_started_for_testing(&velor_framework);

        let perm_handle = permissioned_signer::create_permissioned_handle(creator);
        let perm_signer = permissioned_signer::signer_from_permissioned_handle(&perm_handle);

        permissioned_signer::authorize_increase(creator2, &perm_signer, 100, OnePermission {});
        permissioned_signer::destroy_permissioned_handle(perm_handle);
    }

    // Accessing capacity on a master signer
    #[test(creator = @0xcafe)]
    fun test_invalid_capacity(creator: &signer) {
        assert!(
            permissioned_signer::capacity(creator, OnePermission {})
                == option::some(
                    115792089237316195423570985008687907853269984665640564039457584007913129639935
                ),
            1
        );
    }

    // Making sure master signer always have all permissions even when feature is disabled.
    #[test(creator = @velor_framework)]
    fun test_master_signer_permission(creator: &signer) {
        assert!(
            permissioned_signer::check_permission_exists(creator, OnePermission {}),
            1
        );

        // Disable the permissioned signer feature.
        features::change_feature_flags_for_testing(
            creator,
            vector[],
            vector[features::get_permissioned_signer_feature()]
        );

        // Master signer should still have permission after feature is disabled.
        assert!(
            permissioned_signer::check_permission_exists(creator, OnePermission {}),
            1
        );
    }

    // creating permission using a permissioned signer
    #[test(creator = @0xcafe)]
    #[expected_failure(
        abort_code = 0x50001, location = velor_framework::permissioned_signer
    )]
    fun test_invalid_creation(creator: &signer) {
        let velor_framework = create_signer_for_test(@0x1);
        timestamp::set_time_has_started_for_testing(&velor_framework);

        let perm_handle = permissioned_signer::create_permissioned_handle(creator);
        let perm_signer = permissioned_signer::signer_from_permissioned_handle(&perm_handle);

        let perm_handle_2 = permissioned_signer::create_permissioned_handle(&perm_signer);
        permissioned_signer::destroy_permissioned_handle(perm_handle);
        permissioned_signer::destroy_permissioned_handle(perm_handle_2);
    }

    #[test(creator = @0xcafe)]
    fun test_permission_revocation_success(creator: &signer) {
        let velor_framework = create_signer_for_test(@0x1);
        timestamp::set_time_has_started_for_testing(&velor_framework);

        let perm_handle =
            permissioned_signer::create_storable_permissioned_handle(creator, 60);
        let _perm_signer =
            permissioned_signer::signer_from_storable_permissioned_handle(&perm_handle);

        permissioned_signer::revoke_permission_storage_address(
            creator, permissioned_signer::permissions_storage_address(&perm_handle)
        );

        permissioned_signer::destroy_storable_permissioned_handle(perm_handle);
    }

    #[test(creator = @0xcafe)]
    fun test_permission_revocation_success_with_permissioned_signer(creator: &signer) {
        let velor_framework = create_signer_for_test(@0x1);
        timestamp::set_time_has_started_for_testing(&velor_framework);

        let perm_handle =
            permissioned_signer::create_storable_permissioned_handle(creator, 60);
        let perm_signer =
            permissioned_signer::signer_from_storable_permissioned_handle(&perm_handle);

        permissioned_signer::grant_revoke_permission(creator, &perm_signer);

        permissioned_signer::revoke_permission_storage_address(
            &perm_signer, permissioned_signer::permissions_storage_address(&perm_handle)
        );

        permissioned_signer::destroy_storable_permissioned_handle(perm_handle);
    }

    #[test(creator = @0xcafe)]
    #[expected_failure(
        abort_code = 0x50007, location = velor_framework::permissioned_signer
    )]
    fun test_permission_revocation_and_access(creator: &signer) {
        let velor_framework = create_signer_for_test(@0x1);
        timestamp::set_time_has_started_for_testing(&velor_framework);

        let perm_handle =
            permissioned_signer::create_storable_permissioned_handle(creator, 60);
        let _perm_signer =
            permissioned_signer::signer_from_storable_permissioned_handle(&perm_handle);

        permissioned_signer::revoke_permission_storage_address(
            creator, permissioned_signer::permissions_storage_address(&perm_handle)
        );
        let _perm_signer =
            permissioned_signer::signer_from_storable_permissioned_handle(&perm_handle);

        permissioned_signer::destroy_storable_permissioned_handle(perm_handle);
    }

    #[test(creator1 = @0xcafe, creator2 = @0xbafe)]
    #[expected_failure(
        abort_code = 0x50008, location = velor_framework::permissioned_signer
    )]
    fun test_permission_revoke_other(creator1: &signer, creator2: &signer) {
        let velor_framework = create_signer_for_test(@0x1);
        timestamp::set_time_has_started_for_testing(&velor_framework);

        let perm_handle_1 =
            permissioned_signer::create_storable_permissioned_handle(creator1, 60);

        let perm_handle_2 =
            permissioned_signer::create_storable_permissioned_handle(creator2, 60);

        permissioned_signer::revoke_permission_storage_address(
            creator1, permissioned_signer::permissions_storage_address(&perm_handle_2)
        );

        permissioned_signer::destroy_storable_permissioned_handle(perm_handle_1);
        permissioned_signer::destroy_storable_permissioned_handle(perm_handle_2);
    }

    #[test(creator = @0xcafe)]
    #[expected_failure(abort_code = 453, location = std::bcs)]
    fun test_permissioned_signer_serialization(creator: &signer) {
        let velor_framework = create_signer_for_test(@0x1);
        timestamp::set_time_has_started_for_testing(&velor_framework);

        let perm_handle =
            permissioned_signer::create_storable_permissioned_handle(creator, 60);
        let perm_signer =
            permissioned_signer::signer_from_storable_permissioned_handle(&perm_handle);

        bcs::to_bytes(&perm_signer);

        permissioned_signer::destroy_storable_permissioned_handle(perm_handle);
    }
}
