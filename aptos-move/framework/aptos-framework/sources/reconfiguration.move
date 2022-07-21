/// Publishes configuration information for validators, and issues reconfiguration events
/// to synchronize configuration changes for the validators.
module aptos_framework::reconfiguration {
    use std::error;
    use aptos_std::event;
    use std::signer;
    use std::guid;
    use aptos_framework::system_addresses;
    use aptos_framework::timestamp;
    use aptos_framework::stake;

    friend aptos_framework::aptos_governance;
    friend aptos_framework::block;
    // TODO: migrate all to callback in block prologue
    friend aptos_framework::consensus_config;
    friend aptos_framework::version;
    friend aptos_framework::vm_config;
    friend aptos_framework::transaction_publishing_option;

    /// Event that signals consensus to start a new epoch,
    /// with new configuration information. This is also called a
    /// "reconfiguration event"
    struct NewEpochEvent has drop, store {
        epoch: u64,
    }

    /// Holds information about state of reconfiguration
    struct Configuration has key {
        /// Epoch number
        epoch: u64,
        /// Time of last reconfiguration. Only changes on reconfiguration events.
        last_reconfiguration_time: u64,
        /// Event handle for reconfiguration events
        events: event::EventHandle<NewEpochEvent>,
    }

    /// Reconfiguration disabled if this resource occurs under LibraRoot.
    struct DisableReconfiguration has key {}

    /// The `Configuration` resource is in an invalid state
    const ECONFIGURATION: u64 = 0;
    /// A `Reconfiguration` resource is in an invalid state
    const ECONFIG: u64 = 1;
    /// A `ModifyConfigCapability` is in a different state than was expected
    const EMODIFY_CAPABILITY: u64 = 2;
    /// An invalid block time was encountered.
    const EINVALID_BLOCK_TIME: u64 = 3;
    /// An invalid block time was encountered.
    const EINVALID_GUID_FOR_EVENT: u64 = 4;
    /// The largest possible u64 value
    const MAX_U64: u64 = 18446744073709551615;

    /// Publishes `Configuration` resource. Can only be invoked by aptos framework account, and only a single time in Genesis.
    public fun initialize(
        account: &signer,
    ) {
        timestamp::assert_genesis();
        system_addresses::assert_aptos_framework(account);

        assert!(!exists<Configuration>(@aptos_framework), error::already_exists(ECONFIGURATION));
        // assert it matches `new_epoch_event_key()`, otherwise the event can't be recognized
        assert!(guid::get_next_creation_num(signer::address_of(account)) == 2, error::invalid_state(EINVALID_GUID_FOR_EVENT));
        move_to<Configuration>(
            account,
            Configuration {
                epoch: 0,
                last_reconfiguration_time: 0,
                events: event::new_event_handle<NewEpochEvent>(account),
            }
        );
    }

    /// Private function to temporarily halt reconfiguration.
    /// This function should only be used for offline WriteSet generation purpose and should never be invoked on chain.
    fun disable_reconfiguration(account: &signer) {
        system_addresses::assert_aptos_framework(account);
        assert!(reconfiguration_enabled(), error::invalid_state(ECONFIGURATION));
        move_to(account, DisableReconfiguration {} )
    }

    /// Private function to resume reconfiguration.
    /// This function should only be used for offline WriteSet generation purpose and should never be invoked on chain.
    fun enable_reconfiguration(account: &signer) acquires DisableReconfiguration {
        system_addresses::assert_aptos_framework(account);

        assert!(!reconfiguration_enabled(), error::invalid_state(ECONFIGURATION));
        DisableReconfiguration {} = move_from<DisableReconfiguration>(signer::address_of(account));
    }

    fun reconfiguration_enabled(): bool {
        !exists<DisableReconfiguration>(@aptos_framework)
    }

    /// Force an epoch change.
    public entry fun force_reconfigure(account: &signer) acquires Configuration {
        system_addresses::assert_aptos_framework(account);
        reconfigure();
    }

    /// Signal validators to start using new configuration. Must be called from friend config modules.
    public(friend) fun reconfigure() acquires Configuration {
        stake::on_new_epoch();
        reconfigure_();
    }

    public fun last_reconfiguration_time(): u64 acquires Configuration {
        borrow_global<Configuration>(@aptos_framework).last_reconfiguration_time
    }

    /// Private function to do reconfiguration.  Updates reconfiguration status resource
    /// `Configuration` and emits a `NewEpochEvent`
    fun reconfigure_() acquires Configuration {
        // Do not do anything if genesis has not finished.
        if (timestamp::is_genesis() || timestamp::now_microseconds() == 0 || !reconfiguration_enabled()) {
            return ()
        };

        let config_ref = borrow_global_mut<Configuration>(@aptos_framework);
        let current_time = timestamp::now_microseconds();

        // Do not do anything if a reconfiguration event is already emitted within this transaction.
        //
        // This is OK because:
        // - The time changes in every non-empty block
        // - A block automatically ends after a transaction that emits a reconfiguration event, which is guaranteed by
        //   VM spec that all transactions comming after a reconfiguration transaction will be returned as Retry
        //   status.
        // - Each transaction must emit at most one reconfiguration event
        //
        // Thus, this check ensures that a transaction that does multiple "reconfiguration required" actions emits only
        // one reconfiguration event.
        //
        if (current_time == config_ref.last_reconfiguration_time) {
            return
        };

        assert!(current_time > config_ref.last_reconfiguration_time, error::invalid_state(EINVALID_BLOCK_TIME));
        config_ref.last_reconfiguration_time = current_time;
        config_ref.epoch = config_ref.epoch + 1;

        event::emit_event<NewEpochEvent>(
            &mut config_ref.events,
            NewEpochEvent {
                epoch: config_ref.epoch,
            },
        );
    }

    /// Emit a `NewEpochEvent` event. This function will be invoked by genesis directly to generate the very first
    /// reconfiguration event.
    fun emit_genesis_reconfiguration_event() acquires Configuration {
        assert!(exists<Configuration>(@aptos_framework), error::not_found(ECONFIGURATION));
        let config_ref = borrow_global_mut<Configuration>(@aptos_framework);
        assert!(config_ref.epoch == 0 && config_ref.last_reconfiguration_time == 0, error::invalid_state(ECONFIGURATION));
        config_ref.epoch = 1;

        event::emit_event<NewEpochEvent>(
            &mut config_ref.events,
            NewEpochEvent {
                epoch: config_ref.epoch,
            },
        );
    }
}
