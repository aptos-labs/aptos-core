//# init --parent-vasps Test Alice

//# run --admin-script --signers DiemRoot Alice
script {
use DiemFramework::AccountLimits;
use DiemFramework::XUS;
use Std::Signer;
fun main(dr_account: signer, vasp: signer) {
    let dr_account = &dr_account;
    let vasp = &vasp;
    AccountLimits::publish_unrestricted_limits_for_testing<XUS::XUS>(vasp);
    AccountLimits::publish_window<XUS::XUS>(
        dr_account,
        vasp,
        Signer::address_of(vasp)
    );
}
}

// Give Alice some money...
//# run --type-args 0x1::XUS::XUS --signers DesignatedDealer --args @Alice 1000000 x"" x""
//#     -- 0x1::PaymentScripts::peer_to_peer_with_metadata


//# run --admin-script --signers DiemRoot TreasuryCompliance
script {
use DiemFramework::AccountLimits;
use DiemFramework::XUS;

fun main(_dr: signer, account: signer) {
    let account = &account;
    AccountLimits::update_limits_definition<XUS::XUS>(
        account,
        @Alice,
        0,
        100,
        0,
        0,
    );
}
}

//# publish
module DiemRoot::Test {
    public(script) fun nop() {}
}

// XXX/FIXME: invalid gas currency for account if it doesn't hold it is bad

//# run --signers Alice --gas-price 150 --gas-budget 700 --gas-currency XDX
//#     -- 0xA550C18::Test::nop

// XXX/FIXME
