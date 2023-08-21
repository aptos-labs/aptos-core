module 0xCAFE::test {
    use std::signer;
    use std::string;
    use std::string::String;
    use std::vector;

    struct ModuleData<T> has key, store {
        state: T,
    }

    public entry fun hi(sender: &signer, msg: String) acquires ModuleData {
        let addr = signer::address_of(sender);
        if (!exists<ModuleData<String>>(addr)) {
            move_to(sender, ModuleData<String>{state: msg});
        } else {
            borrow_global_mut<ModuleData<String>>(addr).state = msg;
        }
    }

    public entry fun str_vec(sender: &signer, msgs: vector<String>, i: u64) acquires ModuleData {
        find_hello_in_msgs(&msgs);
        let addr = signer::address_of(sender);
        if (!exists<ModuleData<String>>(addr)) {
            move_to(sender, ModuleData<String>{state: *vector::borrow(&msgs, i)});
        } else {
            borrow_global_mut<ModuleData<String>>(addr).state = *vector::borrow(&msgs, i);
        }
    }

    public entry fun str_vec_vec(sender: &signer, msgs: vector<vector<String>>, i: u64, j: u64) acquires ModuleData {
        find_hello_in_msgs_of_msgs(&msgs);
        let addr = signer::address_of(sender);
        if (!exists<ModuleData<String>>(addr)) {
            move_to(sender, ModuleData<String>{state: *vector::borrow(vector::borrow(&msgs, i), j)});
        } else {
            borrow_global_mut<ModuleData<String>>(addr).state = *vector::borrow(vector::borrow(&msgs, i), j);
        }
    }

    public entry fun multi_vec(
        sender: &signer,
        addresses: vector<vector<address>>,
        msgs: vector<vector<String>>,
        vec1: vector<u64>,
        vec2: vector<u64>,
        i: u64,
        j: u64,
    ) acquires ModuleData {
        assert!(vector::length(&addresses) > 0, 30);
        assert!(vector::length(&msgs) > 0, 31);
        assert!(vector::length(&vec1) >= 0, 32);
        assert!(vector::length(&vec2) >= 0, 33);

        find_hello_in_msgs_of_msgs(&msgs);

        let addr = signer::address_of(sender);
        let msg = *vector::borrow(vector::borrow(&msgs, i), j);
        if (!exists<ModuleData<String>>(addr)) {
            move_to(sender, ModuleData<String>{state: msg});
        } else {
            borrow_global_mut<ModuleData<String>>(addr).state = msg;
        }
    }

    public entry fun non_generic_call(sender: &signer, msg: String) acquires ModuleData {
        let addr = signer::address_of(sender);
        if (!exists<ModuleData<String>>(addr)) {
            move_to<ModuleData<String>>(sender, ModuleData<String> { state: msg });
        } else {
            borrow_global_mut<ModuleData<String>>(addr).state = msg;
        }
    }

    public entry fun generic_call<T: copy + drop + store>(sender: &signer, msg: T) acquires ModuleData {
        let addr = signer::address_of(sender);
        if (!exists<ModuleData<T>>(addr)) {
            move_to<ModuleData<T>>(sender, ModuleData { state: msg });
        } else {
            borrow_global_mut<ModuleData<T>>(addr).state = msg;
        }
    }

    public entry fun generic_multi_vec<T: copy + drop + store, W: copy + drop + store>(
        sender: &signer,
        w_ies: vector<vector<W>>,
        t_ies: vector<vector<T>>,
        vec1: vector<u8>,
        vec2: vector<u64>,
        val1: W,
        val2: T,
        i: u64,
        j: u64,
    ) acquires ModuleData {
        assert!(vector::length(&w_ies) > 0, 30);
        assert!(vector::length(&t_ies) > 0, 31);
        assert!(vector::length(&vec1) >= 0, 32);
        assert!(vector::length(&vec2) >= 0, 33);

        let addr = signer::address_of(sender);
        let v1 = *vector::borrow(vector::borrow(&w_ies, i), j);
        let v2 = *vector::borrow(vector::borrow(&t_ies, i), j);
        let check = (&v1 == &val1) || (&v2 == &val2);
        if (check) {
            if (!exists<ModuleData<T>>(addr)) {
                move_to<ModuleData<T>>(sender, ModuleData { state: v2 });
            } else {
                borrow_global_mut<ModuleData<T>>(addr).state = v2;
            }
        } else {
            if (!exists<ModuleData<T>>(addr)) {
                move_to<ModuleData<T>>(sender, ModuleData { state: v2 });
            } else {
                borrow_global_mut<ModuleData<T>>(addr).state = v2;
            }
        };
    }

    entry fun nothing() {
    }

    fun find_hello_in_msgs_of_msgs(msgs: &vector<vector<String>>) {
        let outer_len = vector::length(msgs);
        while (outer_len > 0) {
            let inner_vec = vector::borrow(msgs, outer_len - 1);
            find_hello_in_msgs(inner_vec);
            outer_len = outer_len - 1;
        };
    }

    fun find_hello_in_msgs(msgs: &vector<String>) {
        let hello = string::utf8(b"hello");
        let len = vector::length(msgs);
        while (len > 0) {
            let str_elem = vector::borrow(msgs, len - 1);
            let idx = string::index_of(str_elem, &hello);
            let str_len = string::length(str_elem);
            assert!(idx < str_len, 50);
            len = len - 1;
        };
    }
}
