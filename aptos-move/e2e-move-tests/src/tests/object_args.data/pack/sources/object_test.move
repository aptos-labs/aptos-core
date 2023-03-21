module 0xCAFE::test {
    use std::vector;
    use std::string::String;
    use std::option::Option;
    use std::fixed_point32::FixedPoint32;
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
    }

    public entry fun object_arg(msg: String, o: Object<ModuleData>) acquires ModuleData {
        let addr = aptos_std::object::object_address(&o);
        // guaranteed to exist
        borrow_global_mut<ModuleData>(addr).state = msg;
    }

    public entry fun object_vec(msg: String, objs: vector<Object<ModuleData>>) acquires ModuleData {
        vector::for_each(objs,|o| { borrow_global_mut<ModuleData>(object::object_address(&o)).state = msg; });
    }

    public entry fun pass_optional_fixedpoint(o: Object<ModuleData>, x: Option<FixedPoint32>) acquires ModuleData {
        let y = std::option::map(x, |e| std::fixed_point32::get_raw_value(e));
        let addr = aptos_std::object::object_address(&o);
        let s = std::vector::empty();
        if (std::option::is_none(&y)) {
            std::vector::append(&mut s,b"none");
        } else {
            let y = std::option::extract(&mut y);
            let ascii0 = 48;
            if (y == 0) {
                std::vector::push_back(&mut s, (ascii0 as u8));
            } else {
                while (y != 0) {
                    let digit = ((ascii0 + (y % 10)) as u8);
                    y = y / 10;
                    std::vector::push_back(&mut s, digit);
                }
            };
            std::vector::reverse(&mut s);
        };
        // guaranteed to exist
        borrow_global_mut<ModuleData>(addr).state = std::string::utf8(s);
    }
}
