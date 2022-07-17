//# init --addresses Alice=0xf75daa73fc071f93593335eb9033da804777eb94491650dd3f095ce6f778acb6
//#      --private-keys Alice=56a26140eb233750cd14fb168c3eb4bd0782b099cde626ec8aff7f3cceb6364f

//# publish
module Alice::HelloWorld {
    use AptosFramework::Signer;
    use AptosFramework::Coin;
    use AptosFramework::TestCoin::TestCoin;
    use AptosFramework::ASCII::{Self, String};

    struct ModuleData has key, store {
        global_counter: u64,
        state: String,
    }

    fun init_module(sender: &signer) {
        move_to(
            sender,
            ModuleData { global_counter: 0, state: ASCII::string(b"init") }
        );
    }

    public fun foo(addr: address): u64 {
        Coin::balance<TestCoin>(addr)
    }

    public(script) fun hi(sender: &signer, msg: String) acquires ModuleData {
        borrow_global_mut<ModuleData>(Signer::address_of(sender)).state = msg;
    }
}


//# run --signers Alice --args x"68656C6C6F20776F726C64" --show-events -- Alice::HelloWorld::hi

//# view --address Alice --resource Alice::HelloWorld::ModuleData