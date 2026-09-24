/// Fee and delivery bookkeeping for the executor that pays for delivery on the
/// destination chain, shaped after a LayerZero V2 executor.
///
/// Fees are debited saturatingly and counters only ever grow, so no mix branch
/// can drive this module into an abort however long a run goes.
module bench::relay_executor {
    use std::signer;
    use aptos_std::table::{Self, Table};

    /// Only the package address may fund a channel.
    const E_NOT_BENCH: u64 = 1;

    /// Charged per delivered byte until a channel sets its own rate.
    const DEFAULT_FEE_PER_BYTE: u64 = 1;

    struct ExecutorAccount has store, drop {
        balance: u128,
        fee_per_byte: u64,
        messages_sent: u64,
        messages_delivered: u64,
        messages_skipped: u64,
        bytes_delivered: u64,
    }

    struct Executor has key {
        accounts: Table<u64, ExecutorAccount>,
    }

    public fun initialize(admin: &signer) {
        assert!(signer::address_of(admin) == @bench, E_NOT_BENCH);
        if (!exists<Executor>(@bench)) {
            move_to(admin, Executor { accounts: table::new() });
        }
    }

    public entry fun fund(
        admin: &signer, channel_id: u64, amount: u128
    ) acquires Executor {
        assert!(signer::address_of(admin) == @bench, E_NOT_BENCH);
        prepay(channel_id, amount);
    }

    /// Unpermissioned top-up. Onboarding thousands of accounts through the
    /// admin would serialize them on the admin's sequence number.
    public fun prepay(channel_id: u64, amount: u128) acquires Executor {
        if (!ensure_account(channel_id)) return;
        let account = table::borrow_mut(
            &mut borrow_global_mut<Executor>(@bench).accounts, channel_id);
        account.balance = account.balance + amount;
    }

    public fun record_send(
        channel_id: u64, n_messages: u64, bytes: u64
    ) acquires Executor {
        if (!ensure_account(channel_id)) return;
        let account = table::borrow_mut(
            &mut borrow_global_mut<Executor>(@bench).accounts, channel_id);
        let fee = (bytes as u128) * (account.fee_per_byte as u128);
        account.balance =
            if (account.balance > fee) account.balance - fee else 0;
        account.messages_sent = account.messages_sent + n_messages;
    }

    public fun record_delivery(
        channel_id: u64, n_messages: u64, bytes: u64
    ) acquires Executor {
        if (!ensure_account(channel_id)) return;
        let account = table::borrow_mut(
            &mut borrow_global_mut<Executor>(@bench).accounts, channel_id);
        account.messages_delivered = account.messages_delivered + n_messages;
        account.bytes_delivered = account.bytes_delivered + bytes;
    }

    public fun record_skip(channel_id: u64, n_messages: u64) acquires Executor {
        if (!ensure_account(channel_id)) return;
        let account = table::borrow_mut(
            &mut borrow_global_mut<Executor>(@bench).accounts, channel_id);
        account.messages_skipped = account.messages_skipped + n_messages;
    }

    public fun set_fee_per_byte(channel_id: u64, fee: u64) acquires Executor {
        if (!ensure_account(channel_id)) return;
        table::borrow_mut(
            &mut borrow_global_mut<Executor>(@bench).accounts, channel_id
        ).fee_per_byte = fee;
    }

    /// Create the channel's account if the executor is up, reporting whether
    /// one is there to borrow afterwards.
    fun ensure_account(channel_id: u64): bool acquires Executor {
        if (!exists<Executor>(@bench)) return false;
        let accounts = &mut borrow_global_mut<Executor>(@bench).accounts;
        if (!table::contains(accounts, channel_id)) {
            table::add(
                accounts,
                channel_id,
                ExecutorAccount {
                    balance: 0,
                    fee_per_byte: DEFAULT_FEE_PER_BYTE,
                    messages_sent: 0,
                    messages_delivered: 0,
                    messages_skipped: 0,
                    bytes_delivered: 0,
                },
            );
        };
        true
    }

    fun has_account(channel_id: u64): bool acquires Executor {
        if (!exists<Executor>(@bench)) return false;
        table::contains(&borrow_global<Executor>(@bench).accounts, channel_id)
    }

    #[view]
    public fun balance_of(channel_id: u64): u128 acquires Executor {
        if (!has_account(channel_id)) return 0;
        table::borrow(
            &borrow_global<Executor>(@bench).accounts, channel_id).balance
    }

    #[view]
    public fun fee_per_byte_of(channel_id: u64): u64 acquires Executor {
        if (!has_account(channel_id)) return DEFAULT_FEE_PER_BYTE;
        table::borrow(
            &borrow_global<Executor>(@bench).accounts, channel_id).fee_per_byte
    }

    #[view]
    public fun messages_sent(channel_id: u64): u64 acquires Executor {
        if (!has_account(channel_id)) return 0;
        table::borrow(
            &borrow_global<Executor>(@bench).accounts, channel_id).messages_sent
    }

    #[view]
    public fun messages_delivered(channel_id: u64): u64 acquires Executor {
        if (!has_account(channel_id)) return 0;
        table::borrow(
            &borrow_global<Executor>(@bench).accounts, channel_id
        ).messages_delivered
    }

    #[view]
    public fun messages_skipped(channel_id: u64): u64 acquires Executor {
        if (!has_account(channel_id)) return 0;
        table::borrow(
            &borrow_global<Executor>(@bench).accounts, channel_id
        ).messages_skipped
    }

    #[view]
    public fun bytes_delivered(channel_id: u64): u64 acquires Executor {
        if (!has_account(channel_id)) return 0;
        table::borrow(
            &borrow_global<Executor>(@bench).accounts, channel_id
        ).bytes_delivered
    }
}
