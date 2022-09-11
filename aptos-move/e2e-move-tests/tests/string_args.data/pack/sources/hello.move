module 0xCAFE::test {
    use std::signer;
    use std::string::String;
    use std::vector;

    struct ModuleData has key, store {
        state: String,
    }

    public entry fun hi(sender: &signer, msg: String) acquires ModuleData {
        let addr = signer::address_of(sender);
        if (!exists<ModuleData>(addr)) {
            move_to(sender, ModuleData{state: msg})
        } else {
            borrow_global_mut<ModuleData>(addr).state = msg;
        }
    }

    public entry fun hi_vec(sender: &signer, msgs: vector<String>, i: u64) acquires ModuleData {
        let addr = signer::address_of(sender);
        if (!exists<ModuleData>(addr)) {
            move_to(sender, ModuleData{state: *vector::borrow(&msgs, i)})
        } else {
            borrow_global_mut<ModuleData>(addr).state = *vector::borrow(&msgs, i);
        }
    }

    public entry fun more_hi_vec(sender: &signer, msgs: vector<vector<String>>, i: u64, j: u64) acquires ModuleData {
        let addr = signer::address_of(sender);
        if (!exists<ModuleData>(addr)) {
            move_to(sender, ModuleData{state: *vector::borrow(vector::borrow(&msgs, i), j)})
        } else {
            borrow_global_mut<ModuleData>(addr).state = *vector::borrow(vector::borrow(&msgs, i), j);
        }
    }
}
