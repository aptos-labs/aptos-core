//# init

// Creating an account of any type at the reserved address 0x0 or core module address 0x1 should fail

//# run --admin-script --signers DiemRoot TreasuryCompliance
script {
use DiemFramework::DiemAccount;
use DiemFramework::XUS::XUS;
fun main(_dr: signer, account: signer) {
    DiemAccount::create_parent_vasp_account<XUS>(
        &account, @0x0, x"00000000000000000000000000000000", x"", false);
}
}

//# run --admin-script --signers DiemRoot DiemRoot
script {
use DiemFramework::DiemAccount;
use DiemFramework::XUS::XUS;
fun main(_dr: signer, account: signer) {
    DiemAccount::create_parent_vasp_account<XUS>(
        &account, @0x0, x"00000000000000000000000000000000", x"", false);
}
}

//# run --admin-script --signers DiemRoot TreasuryCompliance
script {
use DiemFramework::DiemAccount;
use DiemFramework::XDX::XDX;
fun main(_dr: signer, account: signer) {
    DiemAccount::create_parent_vasp_account<XDX>(
        &account, @0x1, x"00000000000000000000000000000000", x"", false);
}
}

//# run --admin-script --signers DiemRoot DiemRoot
script {
use DiemFramework::DiemAccount;
use DiemFramework::XDX::XDX;
fun main(_dr: signer, account: signer) {
    DiemAccount::create_parent_vasp_account<XDX>(
        &account, @0x1, x"00000000000000000000000000000000", x"", false);
}
}
