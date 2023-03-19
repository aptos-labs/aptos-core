module 0xCAFE::test {
    use std::vector;
    use std::string::String;
    use aptos_std::object::Object;
    use aptos_framework::object::{create_object_from_account, generate_signer};
    use aptos_framework::object;

    struct ModuleData has key, store {
        state: String,
    }

    struct GenericModuleData<T: copy + store> has key, store {
        state: T,
    }

    public entry fun initialize(sender: &signer) {
        let cref = create_object_from_account(sender);
        let s = generate_signer(&cref);
        move_to(&s, ModuleData { state: std::string::utf8(b"") });
        std::debug::print(&std::signer::address_of(&s));
    }

    public entry fun object_arg(msg: String, o: Object<ModuleData>) acquires ModuleData {
        let addr = aptos_std::object::object_address(&o);
        // guaranteed to exist
        borrow_global_mut<ModuleData>(addr).state = msg;
    }

    public entry fun object_vec(msg: String, objs: vector<Object<ModuleData>>) acquires ModuleData {
        vector::for_each(objs,|o| { borrow_global_mut<ModuleData>(object::object_address(&o)).state = msg; });
    }
}
