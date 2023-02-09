module aptos_framework::storage_deposit {
    use aptos_framework::event::{EventHandle};
    use aptos_framework::system_addresses;
    use std::error;
    use aptos_framework::account::new_event_handle;
    use std::vector;
    use aptos_framework::coin;
    use aptos_framework::aptos_coin::AptosCoin;
    use aptos_framework::coin::AggregatableCoin;

    /// Maximum possible coin supply.
    const MAX_U128: u128 = 340282366920938463463374607431768211455;

    const EGLOBAL_STORAGE_DEPOSIT: u64 = 0;

    struct GlobalStorageDeposit has key, store {
        deposit: AggregatableCoin<AptosCoin>,
        slot_deposit_event: EventHandle<SlotDepositEvent>,
        excess_bytes_penalty_event: EventHandle<ExcessBytesPenaltyEvent>,
        slot_refund_event: EventHandle<SlotRefundEvent>,
    }

    struct SlotDepositEvent has drop, store {
        payer: address,
        amount: u64,
    }

    struct ExcessBytesPenaltyEvent has drop, store {
        payer: address,
        amount: u64,
    }

    struct SlotRefundEvent has drop, store {
        payee: address,
        amount: u64,
    }

    struct DepositEntry has drop {
        account: address,
        amount: u64,
    }

    struct ChargeSchedule has drop {
        slot_charges: vector<DepositEntry>,
        slot_refunds: vector<DepositEntry>,
        excess_bytes_penalties: vector<DepositEntry>,
    }

    public entry fun initialize(aptos_framework: &signer) {
        system_addresses::assert_aptos_framework(aptos_framework);
        assert!(
            !exists<GlobalStorageDeposit>(@aptos_framework),
            error::already_exists(EGLOBAL_STORAGE_DEPOSIT)
        );

        let global_storage_deposit = GlobalStorageDeposit {
            // FIXME(aldenhu): needs the limit to be u128 (u64 implied)
            deposit: coin::initialize_aggregatable_coin<AptosCoin>(aptos_framework),
            slot_deposit_event: new_event_handle<SlotDepositEvent>(aptos_framework),
            excess_bytes_penalty_event: new_event_handle<ExcessBytesPenaltyEvent>(aptos_framework),
            slot_refund_event: new_event_handle<SlotRefundEvent>(aptos_framework),
        };

        move_to<GlobalStorageDeposit>(aptos_framework, global_storage_deposit);
    }

    public fun charge_and_refund(schedule: ChargeSchedule) acquires GlobalStorageDeposit {
        assert!(
            exists<GlobalStorageDeposit>(@aptos_framework),
            error::not_found(EGLOBAL_STORAGE_DEPOSIT)
        );
        let global_storage_deposit = borrow_global_mut<GlobalStorageDeposit>(@aptos_framework);

        let i = 0;
        let len = vector::length(&schedule.slot_charges);
        while (i <= len) {
            let entry = vector::borrow(&schedule.slot_charges, i);
            coin::collect_into_aggregatable_coin<AptosCoin>(entry.account, entry.amount, &mut global_storage_deposit.deposit);
            // FIXME(aldenhu): central events kills concurrency, probably need to augment Account with these events
            // TODO: emit event
            i = i + 1;
        };

        let i = 0;
        let len = vector::length(&schedule.slot_refunds);
        while (i <= len) {
            let entry = vector::borrow(&schedule.slot_charges, i);
            let coin = coin::extract_from_aggregatable_coin(&mut global_storage_deposit.deposit, entry.amount);
            coin::deposit(entry.account, coin);
            // FIXME(aldenhu): central events kills concurrency, probably need to augment Account with these events
            // TODO: emit event
            i = i + 1;
        };

        let i = 0;
        let len = vector::length(&schedule.excess_bytes_penalties);
        while (i <= len) {
            let entry = vector::borrow(&schedule.slot_charges, i);
            coin::collect_into_aggregatable_coin<AptosCoin>(entry.account, entry.amount, &mut global_storage_deposit.deposit);
            // FIXME(aldenhu): central events kills concurrency, probably need to augment Account with these events
            // TODO: emit event
            i = i + 1;
        };
    }
}
