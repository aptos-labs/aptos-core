module publisher::test {
    use std::signer;
    use std::signer::address_of;
    use aptos_std::smart_table;
    use aptos_std::smart_table::SmartTable;
    use aptos_std::table_with_length;
    use aptos_std::table_with_length::TableWithLength;

    struct Resource has key, store {}

    struct Item has store, drop {}

    struct Stack has key {
        stack: TableWithLength<u64, Item>,
    }

    struct Collection has key {
        collection: SmartTable<u64, Item>,
    }

    public entry fun store_resource_to(account: &signer) {
        move_to(account, Resource {});
    }

    public entry fun remove_resource_from(account: address) acquires Resource {
        assert!(exists<Resource>(account), 123);
        let Resource {} = move_from<Resource>(account);
    }

    public entry fun init_stack(account: &signer) {
        assert!(signer::address_of(account) == @publisher, 123);

        let stack = table_with_length::new<u64, Item>();
        let table_of_items = Stack { stack };
        move_to(account, table_of_items);
    }

    public entry fun stack_push(to_push: u64) acquires Stack {
        assert!(exists<Stack>(@publisher), 123);
        let stack = borrow_global_mut<Stack>(@publisher);

        let len = table_with_length::length(&mut stack.stack);
        let pushed = 0;
        while (pushed < to_push) {
            table_with_length::add(&mut stack.stack, len + pushed, Item {});
            pushed = pushed + 1;
        }
    }

    public entry fun stack_pop(to_pop: u64) acquires Stack {
        assert!(exists<Stack>(@publisher), 123);
        let stack = &mut borrow_global_mut<Stack>(@publisher).stack;

        let len = table_with_length::length(stack);
        assert!(len >= to_pop, 456);
        let popped = 0;
        while (popped < to_pop) {
            table_with_length::remove(stack, len - 1 - popped);
            popped = popped + 1;
        }
    }

    public entry fun store_1_pop_2(account: &signer) acquires Stack {
        store_resource_to(account);
        stack_pop(2);
    }

    public entry fun init_collection_of_1000(account: &signer) {
        let collection = smart_table::new_with_config(1024, 0, 2);
        let i = 0;
        while (i < 1000) {
            smart_table::add(&mut collection, i, Item {});
            i = i + 1;
        };
        move_to(account, Collection { collection });
    }

    public entry fun grow_collection(account: &signer, begin: u64, end: u64) acquires Collection {
        let addr = signer::address_of(account);
        let collection = &mut borrow_global_mut<Collection>(addr).collection;
        let i = begin;
        while (i < end) {
            smart_table::add(collection, i, Item {});
            i = i + 1;
        };
    }

    public entry fun destroy_collection(account: &signer) acquires Collection {
        let addr = address_of(account);
        let Collection { collection } = move_from<Collection>(addr);
        smart_table::destroy(collection);
    }
}
