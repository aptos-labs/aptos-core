spec aptos_framework::aptos_account {
    /// <high-level-req>
    /// No.: 1
    /// Requirement: During the creation of an Aptos account the following rules should hold: (1) the authentication key
    /// should be 32 bytes in length, (2) an Aptos account should not already exist for that authentication key, and (3)
    /// the address of the authentication key should not be equal to a reserved address (0x0, 0x1, or 0x3).
    /// Criticality: Critical
    /// Implementation: The authentication key which is passed in as an argument to create_account should satisfy all
    /// necessary conditions.
    /// Enforcement: Formally verified via [high-level-req-1](CreateAccountAbortsIf).
    ///
    /// No.: 2
    /// Requirement: After creating an Aptos account, the account should become registered to receive AptosCoin.
    /// Criticality: Critical
    /// Implementation: The create_account function creates a new account for the particular address and registers
    /// AptosCoin.
    /// Enforcement: Formally verified via [high-level-req-2](create_account).
    ///
    /// No.: 3
    /// Requirement: An account may receive a direct transfer of coins they have not registered for if and only if the
    /// transfer of arbitrary coins is enabled. By default the option should always set to be enabled for an account.
    /// Criticality: Low
    /// Implementation: Transfers of a coin to an account that has not yet registered for that coin should abort
    /// if and only if the allow_arbitrary_coin_transfers flag is explicitly set to false.
    /// Enforcement: Formally verified via [high-level-req-3](can_receive_direct_coin_transfers).
    ///
    /// No.: 4
    /// Requirement: Setting direct coin transfers may only occur if and only if a direct transfer config is associated
    /// with the provided account address.
    /// Criticality: Low
    /// Implementation: The set_allow_direct_coin_transfers function ensures the DirectTransferConfig structure exists
    /// for the signer.
    /// Enforcement: Formally verified via [high-level-req-4](set_allow_direct_coin_transfers).
    ///
    /// No.: 5
    /// Requirement: The transfer function should ensure an account is created for the provided destination if one does not
    /// exist; then, register AptosCoin for that account if a particular is unregistered before transferring the amount.
    /// Criticality: Critical
    /// Implementation: The transfer function checks if the recipient account exists. If the account does not exist,
    /// the function creates one and registers the account to AptosCoin if not registered.
    /// Enforcement: Formally verified via [high-level-req-5](transfer).
    ///
    /// No.: 6
    /// Requirement: Creating an account for the provided destination and registering it for that particular CoinType
    /// should be the only way to enable depositing coins, provided the account does not already exist.
    /// Criticality: Critical
    /// Implementation: The deposit_coins function verifies if the recipient account exists. If the account does not
    /// exist, the function creates one and ensures that the account becomes registered for the specified CointType.
    /// Enforcement: Formally verified via [high-level-req-6](deposit_coins).
    ///
    /// No.: 7
    /// Requirement: When performing a batch transfer of Aptos Coin and/or a batch transfer of a custom coin type, it
    /// should ensure that the vector containing destination addresses and the vector containing the corresponding
    /// amounts are equal in length.
    /// Criticality: Low
    /// Implementation: The batch_transfer and batch_transfer_coins functions verify that the length of the recipient
    /// addresses vector matches the length of the amount vector through an assertion.
    /// Enforcement: Formally verified via [high-level-req-7](batch_transfer_coins).
    /// </high-level-req>
    ///
    spec module {
        pragma aborts_if_is_strict;
    }

    /// Check if the bytes of the auth_key is 32.
    /// The Account does not exist under the auth_key before creating the account.
    /// Limit the address of auth_key is not @vm_reserved / @aptos_framework / @aptos_toke.
    spec create_account(auth_key: address) {
        use 0x1::account;
        use 0x1::fungible_asset;
        use 0x1::object;
        use 0x1::primary_fungible_store;
        pragma opaque = true, aborts_if_is_partial = false;
        let store_address = object::spec_create_user_derived_object_address(
            auth_key,
            primary_fungible_store::DeriveRefPod[@0xa].metadata_derive_ref.self
        );
        modifies account::Account[auth_key];
        modifies object::ObjectCore[store_address];
        modifies fungible_asset::FungibleStore[store_address];
        modifies fungible_asset::ConcurrentFungibleBalance[store_address];
        modifies object::Untransferable[store_address];
        /// [high-level-req-1]
        include CreateAccountAbortsIf;
        ensures exists<account::Account>(auth_key);
        aborts_if {
            let account_signer = ..S1 |~ result_of<account::create_account>(auth_key);
            S1 |~ aborts_of<register_apt>(account_signer)
        };
    }

    spec schema CreateAccountAbortsIf {
        auth_key: address;
        aborts_if exists<account::Account>(auth_key);
        // aborts_if length_judgment(auth_key);
        aborts_if auth_key == @vm_reserved
            || auth_key == @aptos_framework
            || auth_key == @aptos_token;
    }

    spec fun length_judgment(auth_key: address): bool {
        use std::bcs;

        let authentication_key = bcs::to_bytes(auth_key);
        len(authentication_key) != 32
    }

    spec transfer(source: &signer, to: address, amount: u64) {
        pragma opaque = true;
        // TODO(fa_migration)
        pragma verify = false;
        let account_addr_source = signer::address_of(source);

        include CreateAccountTransferAbortsIf;
        include GuidAbortsIf<AptosCoin>;
        include WithdrawAbortsIf<AptosCoin> { from: source };
        include TransferEnsures<AptosCoin>;

        aborts_if exists<coin::CoinStore<AptosCoin>>(to)
            && global<coin::CoinStore<AptosCoin>>(to).frozen;
        /// [high-level-req-5]
        ensures exists<aptos_framework::account::Account>(to);
        ensures exists<coin::CoinStore<AptosCoin>>(to);
    }

    spec assert_account_exists(addr: address) {
        pragma opaque = true;
        aborts_if !account::spec_exists_at(addr);
    }

    /// Check if the address existed.
    /// Check if the AptosCoin under the address existed.
    spec assert_account_is_registered_for_apt(addr: address) {
        aborts_if !account::spec_exists_at(addr);
        aborts_if !exists<coin::CoinInfo<AptosCoin>>(@aptos_framework);
    }

    spec set_allow_direct_coin_transfers(account: &signer, allow: bool) {
        use 0x1::signer;
        use 0x1::event;
        use 0x1::account;
        let addr = signer::address_of(account);
        include !exists<DirectTransferConfig>(addr) ==>
            account::NewEventHandleAbortsIf { account };
        modifies global<DirectTransferConfig>(addr);
        modifies global<account::Account>(addr);
        ensures exists<DirectTransferConfig>(addr);
        ensures global<DirectTransferConfig>(addr).allow_arbitrary_coin_transfers
            == allow;
        pragma opaque = true, aborts_if_is_partial = true;
        ensures [inferred] old(
            exists<DirectTransferConfig>(signer::address_of(account))
        ) && old(DirectTransferConfig[signer::address_of(account)]).allow_arbitrary_coin_transfers
            != allow ==>
            (
                S4.. |~ ensures_of<event::emit<DirectCoinTransferConfigUpdated>> (
                    DirectCoinTransferConfigUpdated {
                        account: signer::address_of(account),
                        new_allow_direct_transfers: allow
                    }
                )
            );
        ensures [inferred] old(
            exists<DirectTransferConfig>(signer::address_of(account))
        ) && old(DirectTransferConfig[signer::address_of(account)]).allow_arbitrary_coin_transfers
            != allow ==>
            {
                let a = signer::address_of(account);
                let b =
                    update_field(
                        old(DirectTransferConfig[signer::address_of(account)]),
                        allow_arbitrary_coin_transfers,
                        allow
                    );
                ..S4 |~ update<DirectTransferConfig>(a, b)
            };
        aborts_if [inferred]!exists<DirectTransferConfig>(signer::address_of(account))
            && aborts_of<account::new_event_handle<DirectCoinTransferConfigUpdatedEvent>> (account);
        aborts_if [inferred]({
            let a =
                S4 |~ aborts_of<event::emit<DirectCoinTransferConfigUpdated>> (
                    DirectCoinTransferConfigUpdated {
                        account: signer::address_of(account),
                        new_allow_direct_transfers: allow
                    }
                );
            exists<DirectTransferConfig>(signer::address_of(account))
                && (
                    DirectTransferConfig[signer::address_of(account)].allow_arbitrary_coin_transfers
                    != allow
                        && a
                )
        });
    }

    spec batch_transfer(
        source: &signer, recipients: vector<address>, amounts: vector<u64>
    ) {
        //TODO: Can't verify the loop invariant in enumerate
        pragma verify = false;
        let account_addr_source = signer::address_of(source);
        let coin_store_source = global<coin::CoinStore<AptosCoin>>(account_addr_source);
        let balance_source = coin_store_source.coin.value;

        // requires forall i in 0..len(recipients):
        //     recipients[i] != account_addr_source;
        // requires exists i in 0..len(recipients):
        //     amounts[i] > 0;

        // create account properties
        aborts_if len(recipients) != len(amounts);
        aborts_if exists i in 0..len(recipients):
            !account::spec_exists_at(recipients[i]) && length_judgment(recipients[i]);
        aborts_if exists i in 0..len(recipients):
            !account::spec_exists_at(recipients[i])
                && (
                    recipients[i] == @vm_reserved
                        || recipients[i] == @aptos_framework
                        || recipients[i] == @aptos_token
                );
        ensures forall i in 0..len(recipients):
            (
                !account::spec_exists_at(recipients[i]) ==>
                    !length_judgment(recipients[i])
            )
                && (
                    !account::spec_exists_at(recipients[i]) ==>
                        (
                            recipients[i] != @vm_reserved
                                && recipients[i] != @aptos_framework
                                && recipients[i] != @aptos_token
                        )
                );

        // coin::withdraw properties
        aborts_if exists i in 0..len(recipients):
            !exists<coin::CoinStore<AptosCoin>>(account_addr_source);
        aborts_if exists i in 0..len(recipients): coin_store_source.frozen;
        aborts_if exists i in 0..len(recipients):
            global<coin::CoinStore<AptosCoin>>(account_addr_source).coin.value
                < amounts[i];

        // deposit properties
        aborts_if exists i in 0..len(recipients):
            exists<coin::CoinStore<AptosCoin>>(recipients[i])
                && global<coin::CoinStore<AptosCoin>>(recipients[i]).frozen;

        // guid properties
        aborts_if exists i in 0..len(recipients):
            account::spec_exists_at(recipients[i])
                && !exists<coin::CoinStore<AptosCoin>>(recipients[i])
                && global<account::Account>(recipients[i]).guid_creation_num + 2
                    >= account::MAX_GUID_CREATION_NUM;
        aborts_if exists i in 0..len(recipients):
            account::spec_exists_at(recipients[i])
                && !exists<coin::CoinStore<AptosCoin>>(recipients[i])
                && global<account::Account>(recipients[i]).guid_creation_num + 2
                    > MAX_U64;
        pragma opaque = true, aborts_if_is_partial = true;
        ensures [inferred = sathard] len(recipients) == len(amounts) ==>
            (
                forall x: u64:
                    x < len(recipients) ==>
                        ensures_of<transfer>(source, recipients[x], amounts[x])
            );
        aborts_if [inferred = sathard] len(recipients) != len(amounts);
    }

    spec can_receive_direct_coin_transfers(account: address): bool {
        pragma opaque = true;
        aborts_if false;
        /// [high-level-req-3]
        ensures result
            == (
                !exists<DirectTransferConfig>(account)
                    || global<DirectTransferConfig>(account).allow_arbitrary_coin_transfers
            );
    }

    spec batch_transfer_coins<CoinType>(
        from: &signer, recipients: vector<address>, amounts: vector<u64>
    ) {
        //TODO: Can't verify the loop invariant in enumerate
        //use aptos_std::type_info;
        pragma verify = false;
        let account_addr_source = signer::address_of(from);
        let coin_store_source = global<coin::CoinStore<CoinType>>(account_addr_source);
        let balance_source = coin_store_source.coin.value;

        // requires forall i in 0..len(recipients):
        //     recipients[i] != account_addr_source;
        //
        // requires exists i in 0..len(recipients):
        //     amounts[i] > 0;

        /// [high-level-req-7]
        aborts_if len(recipients) != len(amounts);

        //create account properties
        aborts_if exists i in 0..len(recipients):
            !account::spec_exists_at(recipients[i]) && length_judgment(recipients[i]);
        aborts_if exists i in 0..len(recipients):
            !account::spec_exists_at(recipients[i])
                && (
                    recipients[i] == @vm_reserved
                        || recipients[i] == @aptos_framework
                        || recipients[i] == @aptos_token
                );
        ensures forall i in 0..len(recipients):
            (
                !account::spec_exists_at(recipients[i]) ==>
                    !length_judgment(recipients[i])
            )
                && (
                    !account::spec_exists_at(recipients[i]) ==>
                        (
                            recipients[i] != @vm_reserved
                                && recipients[i] != @aptos_framework
                                && recipients[i] != @aptos_token
                        )
                );

        // coin::withdraw properties
        aborts_if exists i in 0..len(recipients):
            !exists<coin::CoinStore<CoinType>>(account_addr_source);
        aborts_if exists i in 0..len(recipients): coin_store_source.frozen;
        aborts_if exists i in 0..len(recipients):
            global<coin::CoinStore<CoinType>>(account_addr_source).coin.value
                < amounts[i];

        // deposit properties
        aborts_if exists i in 0..len(recipients):
            exists<coin::CoinStore<CoinType>>(recipients[i])
                && global<coin::CoinStore<CoinType>>(recipients[i]).frozen;

        // guid properties
        aborts_if exists i in 0..len(recipients):
            account::spec_exists_at(recipients[i])
                && !exists<coin::CoinStore<CoinType>>(recipients[i])
                && global<account::Account>(recipients[i]).guid_creation_num + 2
                    >= account::MAX_GUID_CREATION_NUM;
        aborts_if exists i in 0..len(recipients):
            account::spec_exists_at(recipients[i])
                && !exists<coin::CoinStore<CoinType>>(recipients[i])
                && global<account::Account>(recipients[i]).guid_creation_num + 2
                    > MAX_U64;

    }

    spec deposit_coins<CoinType>(to: address, coins: Coin<CoinType>) {
        use 0x1::fungible_asset;
        use 0x1::object;
        use 0x1::primary_fungible_store;
        use 0x1::type_info;
        pragma opaque = true;
        pragma aborts_if_is_partial = false;
        // Verification remains disabled only while the composed account-creation
        // and coin-to-FA migration proof is being discharged. The contract keeps
        // both branch effects and the complete typed footprint explicit.
        pragma verify = false;
        let metadata_addr = coin::spec_paired_metadata_address<CoinType>();
        let coin_store_addr = object::spec_create_user_derived_object_address(
            to, metadata_addr
        );
        let apt_store_addr = object::spec_create_user_derived_object_address(
            to,
            primary_fungible_store::DeriveRefPod[@0xa].metadata_derive_ref.self
        );
        let coin_info_addr = type_info::type_of<CoinType>().account_address;
        modifies account::Account[to];
        modifies coin::CoinInfo<CoinType>[coin_info_addr];
        modifies coin::CoinConversionMap[@aptos_framework];
        modifies coin::PairedCoinType[metadata_addr];
        modifies coin::PairedFungibleAssetRefs[metadata_addr];
        modifies object::ObjectCore[metadata_addr];
        modifies fungible_asset::Metadata[metadata_addr];
        modifies fungible_asset::Supply[metadata_addr];
        modifies fungible_asset::ConcurrentSupply[metadata_addr];
        modifies primary_fungible_store::DeriveRefPod[metadata_addr];
        modifies object::ObjectCore[apt_store_addr];
        modifies fungible_asset::FungibleStore[apt_store_addr];
        modifies fungible_asset::ConcurrentFungibleBalance[apt_store_addr];
        modifies object::Untransferable[apt_store_addr];
        modifies object::ObjectCore[coin_store_addr];
        modifies fungible_asset::FungibleStore[coin_store_addr];
        modifies fungible_asset::ConcurrentFungibleBalance[coin_store_addr];
        modifies object::Untransferable[coin_store_addr];
        aborts_if !result_of<account::exists_at>(to) && aborts_of<create_account>(to);
        aborts_if aborts_of<coin::is_account_registered<CoinType>> (to);
        aborts_if aborts_of<coin::deposit<CoinType>> (to, coins);
        ensures exists<account::Account>(to);
        ensures !old(account::spec_exists_at(to)) ==>
            ensures_of<create_account>(to);
        ensures ensures_of<coin::deposit<CoinType>> (to, coins);
    }

    spec deposit_fungible_assets(to: address, fa: FungibleAsset) {
        use 0x1::account;
        use 0x1::primary_fungible_store;
        pragma verify = false;
        pragma opaque = true, aborts_if_is_partial = true;
        ensures [inferred] S1.. |~(ensures_of<primary_fungible_store::deposit>(to, fa));
        ensures [inferred](..S1 |~ !result_of<account::exists_at>(to)) ==>
            (S1.. |~ ensures_of<create_account>(to));
    }

    spec mint_to_fungible_store_for_gas(
        ref: &MintRef, account: address, amount: u64
    ) {
        use 0x1::object;
        use 0x1::fungible_asset;
        use 0x1::primary_fungible_store;
        pragma verify = false;
        pragma opaque = true;
        ensures [inferred](
            ..S2 |~ amount != 0 && result_of<fungible_asset::store_exists>(
                object::spec_create_user_derived_object_address(account, @0xa)
            )
        ) ==>
            {
                let a = S2..S4 |~ result_of<fungible_asset::mint>(ref, amount);
                S4.. |~ ensures_of<fungible_asset::unchecked_deposit_with_no_events>(
                    object::spec_create_user_derived_object_address(account, @0xa), a
                )
            };
        ensures [inferred](
            ..S2 |~ amount != 0 && !result_of<fungible_asset::store_exists>(
                object::spec_create_user_derived_object_address(account, @0xa)
            )
        ) ==>
            {
                let a = S2..S4 |~ result_of<fungible_asset::mint>(ref, amount);
                let b =
                    S2.. |~ result_of<primary_fungible_store::create_primary_store<fungible_asset::Metadata
                        >> (
                        account,
                        object::address_to_object<fungible_asset::Metadata>(@0xa)
                    );
                S4.. |~ ensures_of<fungible_asset::unchecked_deposit_with_no_events>(
                    object::object_address<fungible_asset::FungibleStore>(b), a
                )
            };
        aborts_if [inferred]({
            let a =
                ..S2 |~ result_of<fungible_asset::store_exists>(
                    object::spec_create_user_derived_object_address(account, @0xa)
                );
            let b = {
                let c = S2..S4 |~ result_of<fungible_asset::mint>(ref, amount);
                S4 |~ aborts_of<fungible_asset::unchecked_deposit_with_no_events>(
                    object::spec_create_user_derived_object_address(account, @0xa), c
                )
            };
            amount != 0 && (a && b)
        });
        aborts_if [inferred]({
            let a =
                ..S2 |~ result_of<fungible_asset::store_exists>(
                    object::spec_create_user_derived_object_address(account, @0xa)
                );
            let b = {
                let c = S2..S4 |~ result_of<fungible_asset::mint>(ref, amount);
                let d =
                    S2.. |~ result_of<primary_fungible_store::create_primary_store<fungible_asset::Metadata
                        >> (
                        account,
                        object::address_to_object<fungible_asset::Metadata>(@0xa)
                    );
                S4 |~ aborts_of<fungible_asset::unchecked_deposit_with_no_events>(
                    object::object_address<fungible_asset::FungibleStore>(d), c
                )
            };
            amount != 0 && (!a && b)
        });
        aborts_if [inferred]({
            let a =
                ..S2 |~ result_of<fungible_asset::store_exists>(
                    object::spec_create_user_derived_object_address(account, @0xa)
                );
            amount != 0
                && (
                    !a
                        && aborts_of<object::address_to_object<fungible_asset::Metadata>> (@0xa)
                )
        });
    }

    spec transfer_fungible_assets(
        from: &signer, metadata: Object<Metadata>, to: address, amount: u64
    ) {
        use 0x1::fungible_asset;
        use 0x1::primary_fungible_store;
        pragma verify = false;
        pragma opaque = true, aborts_if_is_partial = true;
        ensures [inferred = sathard]({
            let a =
                ..S1 |~ result_of<primary_fungible_store::withdraw<fungible_asset::Metadata
                    >> (from, metadata, amount);
            S1.. |~ ensures_of<deposit_fungible_assets>(to, a)
        });
    }

    spec batch_transfer_fungible_assets(
        from: &signer,
        metadata: Object<Metadata>,
        recipients: vector<address>,
        amounts: vector<u64>
    ) {
        pragma verify = false;
        pragma opaque = true, aborts_if_is_partial = true;
        ensures [inferred = sathard] len(recipients) == len(amounts) ==>
            (
                forall x: u64:
                    x < len(recipients) ==>
                        ensures_of<transfer_fungible_assets>(
                            from, metadata, recipients[x], amounts[x]
                        )
            );
        aborts_if [inferred = sathard] len(recipients) != len(amounts);
    }

    spec transfer_coins<CoinType>(from: &signer, to: address, amount: u64) {
        pragma opaque = true;
        // TODO(fa_migration)
        pragma verify = false;
        let account_addr_source = signer::address_of(from);

        include CreateAccountTransferAbortsIf;
        include WithdrawAbortsIf<CoinType>;
        include GuidAbortsIf<CoinType>;
        include RegistCoinAbortsIf<CoinType>;
        include TransferEnsures<CoinType>;

        aborts_if exists<coin::CoinStore<CoinType>>(to)
            && global<coin::CoinStore<CoinType>>(to).frozen;
        ensures exists<aptos_framework::account::Account>(to);
        ensures exists<aptos_framework::coin::CoinStore<CoinType>>(to);
    }

    spec register_apt(account_signer: &signer) {
        use 0x1::signer;
        use 0x1::object;
        use 0x1::fungible_asset;
        use 0x1::primary_fungible_store;
        pragma opaque = true, aborts_if_is_partial = false;
        let store_address = object::spec_create_user_derived_object_address(
            signer::address_of(account_signer),
            primary_fungible_store::DeriveRefPod[@0xa].metadata_derive_ref.self
        );
        modifies object::ObjectCore[store_address];
        modifies fungible_asset::FungibleStore[store_address];
        modifies fungible_asset::ConcurrentFungibleBalance[store_address];
        modifies object::Untransferable[store_address];
        ensures [inferred](
            ..S2 |~ !result_of<fungible_asset::store_exists>(
                object::spec_create_user_derived_object_address(
                    signer::address_of(account_signer), @0xa
                )
            )
        ) ==>
            {
                let a =
                    S2.. |~ result_of<primary_fungible_store::create_primary_store<fungible_asset::Metadata
                        >> (
                        signer::address_of(account_signer),
                        object::address_to_object<fungible_asset::Metadata>(@0xa)
                    );
                S2.. |~ ensures_of<primary_fungible_store::create_primary_store<fungible_asset::Metadata
                    >> (
                    signer::address_of(account_signer),
                    object::address_to_object<fungible_asset::Metadata>(@0xa),
                    a
                )
            };
        aborts_if [inferred]({
            let a =
                ..S2 |~ result_of<fungible_asset::store_exists>(
                    object::spec_create_user_derived_object_address(
                        signer::address_of(account_signer), @0xa
                    )
                );
            !a && aborts_of<object::address_to_object<fungible_asset::Metadata>> (@0xa)
        });
        aborts_if !result_of<fungible_asset::store_exists>(
            object::spec_create_user_derived_object_address(
                signer::address_of(account_signer), @0xa
            )
        )
            && !aborts_of<object::address_to_object<fungible_asset::Metadata>> (@0xa)
            && aborts_of<primary_fungible_store::create_primary_store<fungible_asset::Metadata
                >> (
                signer::address_of(account_signer),
                object::address_to_object<fungible_asset::Metadata>(@0xa)
            );
    }

    spec fungible_transfer_only(source: &signer, to: address, amount: u64) {
        use 0x1::signer;
        use 0x1::object;
        use 0x1::fungible_asset;
        use 0x1::primary_fungible_store;
        // TODO: temporary mockup.
        pragma verify = false;
        pragma opaque = true, aborts_if_is_partial = false;
        ensures [inferred](
            ..S4 |~ result_of<fungible_asset::store_exists>(
                object::spec_create_user_derived_object_address(
                    signer::address_of(source), @0xa
                )
            )
        ) && (
            S4..S5 |~ result_of<fungible_asset::store_exists>(
                object::spec_create_user_derived_object_address(to, @0xa)
            )
        ) ==>
            {
                let a =
                    S5..S7 |~ result_of<fungible_asset::unchecked_withdraw>(
                        object::spec_create_user_derived_object_address(
                            signer::address_of(source), @0xa
                        ),
                        amount
                    );
                S7.. |~ ensures_of<fungible_asset::unchecked_deposit>(
                    object::spec_create_user_derived_object_address(to, @0xa), a
                )
            };
        ensures [inferred](
            ..S4 |~ result_of<fungible_asset::store_exists>(
                object::spec_create_user_derived_object_address(
                    signer::address_of(source), @0xa
                )
            )
        ) && (
            S4..S5 |~ !result_of<fungible_asset::store_exists>(
                object::spec_create_user_derived_object_address(to, @0xa)
            )
        ) ==>
            {
                let a =
                    S5.. |~ result_of<primary_fungible_store::create_primary_store<fungible_asset::Metadata
                        >> (to, object::address_to_object<fungible_asset::Metadata>(@0xa));
                let b =
                    S5..S7 |~ result_of<fungible_asset::unchecked_withdraw>(
                        object::spec_create_user_derived_object_address(
                            signer::address_of(source), @0xa
                        ),
                        amount
                    );
                S7.. |~ ensures_of<fungible_asset::unchecked_deposit>(
                    object::object_address<fungible_asset::FungibleStore>(a), b
                )
            };
        ensures [inferred](
            ..S4 |~ !result_of<fungible_asset::store_exists>(
                object::spec_create_user_derived_object_address(
                    signer::address_of(source), @0xa
                )
            )
        ) && (
            S4..S5 |~ result_of<fungible_asset::store_exists>(
                object::spec_create_user_derived_object_address(to, @0xa)
            )
        ) ==>
            {
                let a = {
                    let b =
                        S4.. |~ result_of<primary_fungible_store::create_primary_store<fungible_asset::Metadata
                            >> (
                            signer::address_of(source),
                            object::address_to_object<fungible_asset::Metadata>(@0xa)
                        );
                    S5..S7 |~ result_of<fungible_asset::unchecked_withdraw>(
                        object::object_address<fungible_asset::FungibleStore>(b), amount
                    )
                };
                S7.. |~ ensures_of<fungible_asset::unchecked_deposit>(
                    object::spec_create_user_derived_object_address(to, @0xa), a
                )
            };
        ensures [inferred](
            ..S4 |~ !result_of<fungible_asset::store_exists>(
                object::spec_create_user_derived_object_address(
                    signer::address_of(source), @0xa
                )
            )
        ) && (
            S4..S5 |~ !result_of<fungible_asset::store_exists>(
                object::spec_create_user_derived_object_address(to, @0xa)
            )
        ) ==>
            {
                let a =
                    S5.. |~ result_of<primary_fungible_store::create_primary_store<fungible_asset::Metadata
                        >> (to, object::address_to_object<fungible_asset::Metadata>(@0xa));
                let b = {
                    let c =
                        S4.. |~ result_of<primary_fungible_store::create_primary_store<fungible_asset::Metadata
                            >> (
                            signer::address_of(source),
                            object::address_to_object<fungible_asset::Metadata>(@0xa)
                        );
                    S5..S7 |~ result_of<fungible_asset::unchecked_withdraw>(
                        object::object_address<fungible_asset::FungibleStore>(c), amount
                    )
                };
                S7.. |~ ensures_of<fungible_asset::unchecked_deposit>(
                    object::object_address<fungible_asset::FungibleStore>(a), b
                )
            };
        aborts_if [inferred]({
            let a =
                S4..S5 |~ result_of<fungible_asset::store_exists>(
                    object::spec_create_user_derived_object_address(to, @0xa)
                );
            !a && aborts_of<object::address_to_object<fungible_asset::Metadata>> (@0xa)
        });
        aborts_if [inferred](
            ..S4 |~ result_of<fungible_asset::store_exists>(
                object::spec_create_user_derived_object_address(
                    signer::address_of(source), @0xa
                )
            )
        )
            && (
                (
                    S4..S5 |~ result_of<fungible_asset::store_exists>(
                        object::spec_create_user_derived_object_address(to, @0xa)
                    )
                )
                    && {
                        let a =
                            S5..S7 |~ result_of<fungible_asset::unchecked_withdraw>(
                                object::spec_create_user_derived_object_address(
                                    signer::address_of(source), @0xa
                                ),
                                amount
                            );
                        S7 |~ aborts_of<fungible_asset::unchecked_deposit>(
                            object::spec_create_user_derived_object_address(to, @0xa), a
                        )
                    }
            );
        aborts_if [inferred](
            ..S4 |~ result_of<fungible_asset::store_exists>(
                object::spec_create_user_derived_object_address(
                    signer::address_of(source), @0xa
                )
            )
        ) && (
            (
                S4..S5 |~ !result_of<fungible_asset::store_exists>(
                    object::spec_create_user_derived_object_address(to, @0xa)
                )
            ) && {
                let a =
                    S5.. |~ result_of<primary_fungible_store::create_primary_store<fungible_asset::Metadata
                        >> (to, object::address_to_object<fungible_asset::Metadata>(@0xa));
                let b =
                    S5..S7 |~ result_of<fungible_asset::unchecked_withdraw>(
                        object::spec_create_user_derived_object_address(
                            signer::address_of(source), @0xa
                        ),
                        amount
                    );
                S7 |~ aborts_of<fungible_asset::unchecked_deposit>(
                    object::object_address<fungible_asset::FungibleStore>(a), b
                )
            }
        );
        aborts_if [inferred]({
            let a =
                S4..S5 |~ result_of<fungible_asset::store_exists>(
                    object::spec_create_user_derived_object_address(to, @0xa)
                );
            let b =
                ..S4 |~ result_of<fungible_asset::store_exists>(
                    object::spec_create_user_derived_object_address(
                        signer::address_of(source), @0xa
                    )
                );
            let c = {
                let d = {
                    let e =
                        S4.. |~ result_of<primary_fungible_store::create_primary_store<fungible_asset::Metadata
                            >> (
                            signer::address_of(source),
                            object::address_to_object<fungible_asset::Metadata>(@0xa)
                        );
                    S5..S7 |~ result_of<fungible_asset::unchecked_withdraw>(
                        object::object_address<fungible_asset::FungibleStore>(e), amount
                    )
                };
                S7 |~ aborts_of<fungible_asset::unchecked_deposit>(
                    object::spec_create_user_derived_object_address(to, @0xa), d
                )
            };
            !b && (a && c)
        });
        aborts_if [inferred]({
            let a =
                S4..S5 |~ result_of<fungible_asset::store_exists>(
                    object::spec_create_user_derived_object_address(to, @0xa)
                );
            let b =
                ..S4 |~ result_of<fungible_asset::store_exists>(
                    object::spec_create_user_derived_object_address(
                        signer::address_of(source), @0xa
                    )
                );
            let c = {
                let d =
                    S5.. |~ result_of<primary_fungible_store::create_primary_store<fungible_asset::Metadata
                        >> (to, object::address_to_object<fungible_asset::Metadata>(@0xa));
                let e = {
                    let f =
                        S4.. |~ result_of<primary_fungible_store::create_primary_store<fungible_asset::Metadata
                            >> (
                            signer::address_of(source),
                            object::address_to_object<fungible_asset::Metadata>(@0xa)
                        );
                    S5..S7 |~ result_of<fungible_asset::unchecked_withdraw>(
                        object::object_address<fungible_asset::FungibleStore>(f), amount
                    )
                };
                S7 |~ aborts_of<fungible_asset::unchecked_deposit>(
                    object::object_address<fungible_asset::FungibleStore>(d), e
                )
            };
            !b && (!a && c)
        });
        aborts_if [inferred]({
            let a =
                ..S4 |~ result_of<fungible_asset::store_exists>(
                    object::spec_create_user_derived_object_address(
                        signer::address_of(source), @0xa
                    )
                );
            !a && aborts_of<object::address_to_object<fungible_asset::Metadata>> (@0xa)
        });
    }

    spec is_fungible_balance_at_least(account: address, amount: u64): bool {
        use 0x1::object;
        use 0x1::fungible_asset;
        // TODO: temporary mockup.
        pragma verify = false;
        pragma opaque = true;
        ensures [inferred] result
            == result_of<fungible_asset::is_address_balance_at_least>(
                object::spec_create_user_derived_object_address(account, @0xa), amount
            );
        aborts_if [inferred] false;
    }

    spec burn_from_fungible_store_for_gas(
        ref: &BurnRef, account: address, amount: u64
    ) {
        use 0x1::object;
        use 0x1::fungible_asset;
        pragma opaque = true, aborts_if_is_partial = false;
        let store_address = object::spec_create_user_derived_object_address(
            account, @0xa
        );
        let metadata_address = object::object_address<fungible_asset::Metadata>(
            ref.metadata
        );
        modifies fungible_asset::FungibleStore[store_address];
        modifies fungible_asset::ConcurrentFungibleBalance[store_address];
        modifies fungible_asset::Supply[metadata_address];
        modifies fungible_asset::ConcurrentSupply[metadata_address];
        ensures [inferred] amount != 0 ==>
            ensures_of<fungible_asset::address_burn_from_for_gas>(
                ref,
                object::spec_create_user_derived_object_address(account, @0xa),
                amount
            );
        aborts_if amount != 0
            && aborts_of<fungible_asset::address_burn_from_for_gas>(
                ref,
                object::spec_create_user_derived_object_address(account, @0xa),
                amount
            );
    }

    spec schema CreateAccountTransferAbortsIf {
        to: address;
        aborts_if !account::spec_exists_at(to) && length_judgment(to);
        aborts_if !account::spec_exists_at(to)
            && (to == @vm_reserved
                || to == @aptos_framework
                || to == @aptos_token);
    }

    spec schema WithdrawAbortsIf<CoinType> {
        from: &signer;
        amount: u64;
        let account_addr_source = signer::address_of(from);
        let coin_store_source = global<coin::CoinStore<CoinType>>(account_addr_source);
        let balance_source = coin_store_source.coin.value;
        aborts_if !exists<coin::CoinStore<CoinType>>(account_addr_source);
        aborts_if coin_store_source.frozen;
        aborts_if balance_source < amount;
    }

    spec schema GuidAbortsIf<CoinType> {
        to: address;
        let acc = global<account::Account>(to);
        aborts_if account::spec_exists_at(to)
            && !exists<coin::CoinStore<CoinType>>(to)
            && acc.guid_creation_num + 2 >= account::MAX_GUID_CREATION_NUM;
        aborts_if account::spec_exists_at(to)
            && !exists<coin::CoinStore<CoinType>>(to)
            && acc.guid_creation_num + 2 > MAX_U64;
    }

    spec schema RegistCoinAbortsIf<CoinType> {
        use aptos_std::type_info;
        to: address;
        // TODO(fa_migration)
        // aborts_if !coin::spec_is_account_registered<CoinType>(to) && !type_info::spec_is_struct<CoinType>();
        aborts_if exists<aptos_framework::account::Account>(to);
        aborts_if type_info::type_of<CoinType>() != type_info::type_of<AptosCoin>();
    }

    spec schema TransferEnsures<CoinType> {
        to: address;
        account_addr_source: address;
        amount: u64;

        let if_exist_account = exists<account::Account>(to);
        let if_exist_coin = exists<coin::CoinStore<CoinType>>(to);
        let coin_store_to = global<coin::CoinStore<CoinType>>(to);
        let coin_store_source = global<coin::CoinStore<CoinType>>(account_addr_source);
        let post p_coin_store_to = global<coin::CoinStore<CoinType>>(to);
        let post p_coin_store_source = global<coin::CoinStore<CoinType>>(
            account_addr_source
        );
        ensures coin_store_source.coin.value - amount == p_coin_store_source.coin.value;
        ensures if_exist_account && if_exist_coin ==>
            coin_store_to.coin.value + amount == p_coin_store_to.coin.value;
    }
}
