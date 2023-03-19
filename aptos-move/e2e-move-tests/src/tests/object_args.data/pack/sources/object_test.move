module 0xCAFE::test {
    use std::vector;
    use std::string::String;
    use aptos_std;
    use aptos_std::object::Object;

    struct ModuleData has key, store {
        state: String,
    }

    struct GenericModuleData<T: copy + store> has key, store {
        state: T,
    }

    public entry fun object_arg(sender: &signer, msg: String, o: Object<ModuleData>) acquires ModuleData {
        let addr = aptos_std::object::get_address(o);
        if (!exists<ModuleData>(addr)) {
            std::aptos_std::get_signer(o);
            move_to(, ModuleData{state: msg});
        } else {
            borrow_global_mut<ModuleData>(addr).state = msg;
        }
    }

    public entry fun object_vec(sender: &signer, msg: String, objs: vector<Object<ModuleData>>, i: u64) acquires ModuleData {
        let obj = *vector::borrow(&objs, i);
        let addr = aptos_std::object::get_address(obj);
        let addr = aptos_std::object::get_address(*vector::borrow(&obj, i));
        if (!exists<ModuleData>(addr)) {
            move_to(sender, ModuleData{state: msg});
        } else {
            borrow_global_mut<ModuleData>(addr).state = msg;
        }
    }

    public entry fun object_vec_vec(sender: &signer, msg: String, objs: vector<vector<Object<ModuleData>>>, i: u64, j: u64) acquires ModuleData {
        let obj = *vector::borrow(vector::borrow(&objs, i), j);
        let addr = aptos_std::object::get_address(obj);
        if (!exists<ModuleData>(addr)) {
            move_to(sender, ModuleData{state: msg});
        } else {
            borrow_global_mut<ModuleData>(addr).state = msg;
        }
    }
}
