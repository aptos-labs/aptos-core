module 0xcafe::read_helper {
    use std::signer;
    use aptos_std::table::{Self, Table};
    use aptos_framework::account;
    use aptos_framework::aptos_coin::AptosCoin;
    use aptos_framework::coin;

    #[resource_group(scope = global)]
    struct Group {}

    #[resource_group_member(group = 0xcafe::read_helper::Group)]
    struct InGroup has key { value: u64 }

    struct Plain has key { value: u64 }

    struct TableHolder has key { entries: Table<u64, u64> }

    /// Publishes the read targets under `account`: a plain resource, a resource-group member, and a
    /// one-entry table. Run as its own applied transaction so that a later block only reads them.
    public entry fun init(account: &signer) {
        move_to(account, Plain { value: 1 });
        move_to(account, InGroup { value: 2 });
        let entries = table::new<u64, u64>();
        table::add(&mut entries, 7, 70);
        move_to(account, TableHolder { entries });
    }

    /// Reads a plain (non-group) resource without writing it.
    public entry fun read_plain(target: address) acquires Plain {
        let _ = borrow_global<Plain>(target).value;
    }

    /// Reads a resource-group member. The recorded read key is the enclosing *group* key, not the
    /// member's own struct tag.
    public entry fun read_group_member(target: address) acquires InGroup {
        let _ = borrow_global<InGroup>(target).value;
    }

    /// `exists<T>` loads the slot to answer, so even an absent resource is recorded as a read.
    public entry fun check_exists(target: address) {
        let _ = exists<Plain>(target);
    }

    /// Reads a single table item without writing it.
    public entry fun read_table_item(target: address) acquires TableHolder {
        let holder = borrow_global<TableHolder>(target);
        let _ = *table::borrow(&holder.entries, 7);
    }

    /// Reads framework state without writing it: the account resource of `target`, plus
    /// `CoinInfo<AptosCoin>` and the table-backed coin-to-fungible-asset conversion map behind
    /// `coin::supply`.
    public entry fun read_only(target: address) {
        let _ = account::get_sequence_number(target);
        let _ = coin::supply<AptosCoin>();
    }

    /// Reads (via `exists`) and then mutates the caller's own `Plain`, so its slot is written.
    public entry fun write_plain(account: &signer) acquires Plain {
        let addr = signer::address_of(account);
        if (exists<Plain>(addr)) {
            let plain = borrow_global_mut<Plain>(addr);
            plain.value = plain.value + 1;
        } else {
            move_to(account, Plain { value: 1 });
        };
    }
}
