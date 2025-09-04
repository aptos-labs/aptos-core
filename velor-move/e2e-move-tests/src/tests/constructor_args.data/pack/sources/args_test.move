module 0xCAFE::test {
    use std::vector;
    use std::string::String;
    use std::option::Option;
    use std::fixed_point32::FixedPoint32;
    use velor_std::fixed_point64::FixedPoint64;
    use velor_std::object::Object;
    use velor_framework::object::{create_object_from_account, generate_signer};
    use velor_framework::object;

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
        let addr = velor_std::object::object_address(&o);
        // guaranteed to exist
        borrow_global_mut<ModuleData>(addr).state = msg;
    }

    public entry fun object_vec(msg: String, objs: vector<Object<ModuleData>>) acquires ModuleData {
        vector::for_each(objs,|o| { borrow_global_mut<ModuleData>(object::object_address(&o)).state = msg; });
    }

    public entry fun pass_optional_fixedpoint32(o: Object<ModuleData>, x: Option<FixedPoint32>) acquires ModuleData {
        let y = std::option::map(x, |e| std::fixed_point32::get_raw_value(e));
        let addr = velor_std::object::object_address(&o);
        let s;
        if (std::option::is_none(&y)) {
            s = std::string::utf8(b"none");
        } else {
            s = convert((std::option::extract(&mut y) as u128));
        };
        // guaranteed to exist
        borrow_global_mut<ModuleData>(addr).state = s;
    }

    public entry fun pass_optional_vector_fixedpoint64(o: Object<ModuleData>, x: Option<vector<FixedPoint64>>, i: u64) acquires ModuleData {
        let addr = velor_std::object::object_address(&o);
        let s;
        if (std::option::is_none(&x)) {
            s = std::string::utf8(b"none");
        } else {
            let x = std::option::extract(&mut x);
            s = convert(velor_std::fixed_point64::get_raw_value(*std::vector::borrow(&x, i)));
        };

        // guaranteed to exist
        borrow_global_mut<ModuleData>(addr).state = s;
    }

    public entry fun pass_optional_vector_optional_string(o: Object<ModuleData>, x: Option<vector<Option<String>>>, i: u64) acquires ModuleData {
        let addr = velor_std::object::object_address(&o);
        let s;
        if (std::option::is_none(&x)) {
            s = std::string::utf8(b"empty top option");
        } else {
            let x = std::option::extract(&mut x);
            let x = std::vector::borrow_mut(&mut x, i);
            if (std::option::is_none(x)) {
                s = std::string::utf8(b"empty bottom option");
            } else {
                s = std::option::extract(x);
            }
        };

        // guaranteed to exist
        borrow_global_mut<ModuleData>(addr).state = s;
    }

    public entry fun pass_vector_optional_object(o: vector<Option<Object<ModuleData>>>, s: String, i: u64) acquires ModuleData {
        let o = std::vector::borrow_mut(&mut o, i);
        if (std::option::is_none(o)) {
            return
        } else {
            let o = std::option::extract(o);
            let addr = velor_std::object::object_address(&o);
            // guaranteed to exist
            borrow_global_mut<ModuleData>(addr).state = s;
        };
    }

    // Valuable data that should not be able to be fabricated by a malicious tx
    struct MyPrecious {
        value: u64,
    }

    public entry fun ensure_no_fabrication(my_precious: Option<MyPrecious>) {
        if (std::option::is_none(&my_precious)) {
            std::option::destroy_none(my_precious)
        } else {
            let MyPrecious { value : _ } = std::option::destroy_some(my_precious);
        }
    }

    public entry fun ensure_vector_vector_u8(o: Object<ModuleData>, _: vector<vector<u8>>) acquires ModuleData {
        let addr = velor_std::object::object_address(&o);
        // guaranteed to exist
        borrow_global_mut<ModuleData>(addr).state = std::string::utf8(b"vector<vector<u8>>");
    }

    fun convert(x: u128): String {
        let s = std::vector::empty();
        let ascii0 = 48;
        if (x == 0) {
            std::vector::push_back(&mut s, (ascii0 as u8));
        } else {
            while (x != 0) {
                let digit = ((ascii0 + (x % 10)) as u8);
                x = x / 10;
                std::vector::push_back(&mut s, digit);
            }
        };
        std::vector::reverse(&mut s);
        std::string::utf8(s)
    }

    #[view]
    public fun get_state<T: key>(o: Object<T>): String acquires ModuleData {
        let addr = velor_std::object::object_address(&o);
        borrow_global<ModuleData>(addr).state
    }
}
