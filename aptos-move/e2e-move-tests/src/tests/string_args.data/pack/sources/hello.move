module 0xCAFE::test {
    use std::signer;
    use std::string::String;

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
}
