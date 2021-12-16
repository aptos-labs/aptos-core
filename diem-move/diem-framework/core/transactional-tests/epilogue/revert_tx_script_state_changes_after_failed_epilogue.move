//# init --parent-vasps Alice Bob Carol

// Give Alice some money...
//# run --type-args 0x1::XUS::XUS --signers DesignatedDealer --args @Alice 10000 x"" x""
//#     -- 0x1::PaymentScripts::peer_to_peer_with_metadata

// Give Bob some money...
//# run --type-args 0x1::XUS::XUS --signers DesignatedDealer --args @Bob 10000 x"" x""
//#     -- 0x1::PaymentScripts::peer_to_peer_with_metadata


// Transfer all of the Alice's funds to Carol. this script will execute successfully, but
// the epilogue will fail because Alice spent her gas deposit. The VM should revert the state
// changes and re-execute the epilogue. Alice will still be charged for the gas she used.

//# run --type-args 0x1::XUS::XUS
//#     --gas-price 1
//#     --signers Alice --args @Carol 10000 x"" x""
//#     -- 0x1::PaymentScripts::peer_to_peer_with_metadata


// Bob sends the same transaction script, with the amount set to 1000 instead of his full balance.
// This transaction should go through and bob will pay for the gas he used.

//# run --type-args 0x1::XUS::XUS
//#     --gas-price 1
//#     --signers Bob --args @Carol 1000 x"" x""
//#     -- 0x1::PaymentScripts::peer_to_peer_with_metadata


// Check that the following invariants holds:
// 1) Carol's balance has went up by 1000, receiving funds from Bob but not Alice.
// 2) Alice's balance is exactly 1000 greater than Bob's, indicating they consumed the same amount of gas.

//# run --admin-script --signers DiemRoot DiemRoot
script {
use DiemFramework::DiemAccount;
use DiemFramework::XUS::XUS;

fun main() {
    assert!(DiemAccount::balance<XUS>(@Carol) == 1000, 42);
    assert!(DiemAccount::balance<XUS>(@Alice) == DiemAccount::balance<XUS>(@Bob) + 1000, 43)
}
}
