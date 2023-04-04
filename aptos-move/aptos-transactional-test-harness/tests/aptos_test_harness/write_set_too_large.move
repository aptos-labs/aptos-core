//# init --addresses Alice=0xf75daa73fc071f93593335eb9033da804777eb94491650dd3f095ce6f778acb6
//#      --private-keys Alice=56a26140eb233750cd14fb168c3eb4bd0782b099cde626ec8aff7f3cceb6364f
//#      --initial-coins 1000000000000000

//# publish
module Alice::Module {
    use std::vector;
    use std::string::{Self, String};

    struct ModuleData has key, store {
        data: vector<String>,
    }

    public entry fun write_large_data(account: &signer) {
        let data: vector<String> = vector::empty();
        let str = string::utf8(b"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef");
        let cnt: u64 = 1024 * 8;
        while (cnt > 0) {
            vector::push_back(&mut data, str);
            cnt = cnt - 1;
        };

        move_to<ModuleData>(account, ModuleData {
            data,
        });
    }
}

//# run --signers Alice --show-events --gas-budget 2000000 -- Alice::Module::write_large_data
