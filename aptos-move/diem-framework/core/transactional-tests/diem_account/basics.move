//# init --parent-vasps Test Alice Bob Caroll Doris=0x1::XDX::XDX
//#      --addresses Abby=0x751eb65a16f7f36411cb3990a6f08c58
//#      --private-keys Abby=1bbba5c7064e06e4fb757d14823f36f76bf7f97eb751cf11e8c8294a75aa159c

// Give Bob some money...
//# run --type-args 0x1::XUS::XUS --signers DesignatedDealer --args @Bob 100000 x"" x""
//#     -- 0x1::PaymentScripts::peer_to_peer_with_metadata

//# publish
module Test::Holder {
    use Std::Signer;

    struct Hold<T> has key {
        x: T
    }

    public fun hold<T: store>(account: &signer, x: T) {
        move_to(account, Hold<T>{x})
    }

    public fun get<T: store>(account: &signer): T
    acquires Hold {
        let Hold {x} = move_from<Hold<T>>(Signer::address_of(account));
        x
    }
}

//# run --admin-script --signers DiemRoot Alice
script {
    use DiemFramework::DiemAccount;
    fun main(_dr: signer, sender: signer) {
        DiemAccount::initialize(&sender, x"00000000000000000000000000000000");
    }
}

//# run --type-args 0x1::XUS::XUS --signers Bob --args @Bob 10 x"" x""
//#     -- 0x1::PaymentScripts::peer_to_peer_with_metadata

//# run --type-args 0x1::XUS::XUS --signers Bob --args @Abby 10 x"" x""
//#     -- 0x1::PaymentScripts::peer_to_peer_with_metadata

//# run --type-args 0x1::XDX::XDX --signers Bob --args @Abby 10 x"" x""
//#     -- 0x1::PaymentScripts::peer_to_peer_with_metadata

//# run --type-args 0x1::XUS::XUS --signers Bob --args @Doris 10 x"" x""
//#     -- 0x1::PaymentScripts::peer_to_peer_with_metadata

//# run --signers Bob --args x"123abc"
//#     -- 0x1::AccountAdministrationScripts::rotate_authentication_key

//# run --admin-script --signers DiemRoot Bob
script {
    use DiemFramework::DiemAccount;
    use Test::Holder;
    fun main(_dr: signer, account: signer) {
        Holder::hold(
            &account,
            DiemAccount::extract_key_rotation_capability(&account)
        );
        Holder::hold(
            &account,
            DiemAccount::extract_key_rotation_capability(&account)
        );
    }
}

//# run --admin-script --signers DiemRoot Caroll
script {
    use DiemFramework::DiemAccount;
    use Std::Signer;
    fun main(_dr: signer, sender: signer) {
        let cap = DiemAccount::extract_key_rotation_capability(&sender);
        assert!(
            *DiemAccount::key_rotation_capability_address(&cap) == Signer::address_of(&sender), 0
        );
        DiemAccount::restore_key_rotation_capability(cap);
        let with_cap = DiemAccount::extract_withdraw_capability(&sender);

        assert!(
            *DiemAccount::withdraw_capability_address(&with_cap) == Signer::address_of(&sender),
            0
        );
        DiemAccount::restore_withdraw_capability(with_cap);
    }
}

//# run --type-args 0x1::XUS::XUS --signers Bob --args @Alice 10000 x"" x""
//#     -- 0x1::PaymentScripts::peer_to_peer_with_metadata

//# run --admin-script --signers DiemRoot DiemRoot
script {
    use DiemFramework::DiemAccount;
    use DiemFramework::XUS::XUS;

    fun main() {
        assert!(DiemAccount::balance<XUS>(@Alice) == 10000, 60)
    }
}

//# run --signers TreasuryCompliance
//#     --type-args 0x1::XDX::XDX
//#     --args 0
//#            0x0
//#            x"00000000000000000000000000000000"
//#            b"xxx"
//#            true
//#     -- 0x1::AccountCreationScripts::create_parent_vasp_account

//# run --signers TreasuryCompliance
//#     --type-args 0x1::XDX::XDX
//#     --args 0
//#            @Abby
//#            x""
//#            b"abby"
//#            true
//#     -- 0x1::AccountCreationScripts::create_parent_vasp_account

//# run --admin-script --signers DiemRoot DiemRoot
script {
use DiemFramework::DiemAccount;
fun main() {
    DiemAccount::sequence_number(@0x1);
}
}

//# run --admin-script --signers DiemRoot DiemRoot
script {
use DiemFramework::DiemAccount;
fun main() {
    DiemAccount::authentication_key(@0x1);
}
}

//# run --admin-script --signers DiemRoot DiemRoot
script {
use DiemFramework::DiemAccount;
fun main() {
    DiemAccount::delegated_key_rotation_capability(@0x1);
}
}

//# run --admin-script --signers DiemRoot DiemRoot
script {
use DiemFramework::DiemAccount;
fun main() {
    DiemAccount::delegated_withdraw_capability(@0x1);
}
}
