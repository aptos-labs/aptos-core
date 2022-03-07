//# init --parent-vasps Alice Bob

// Dummy module for testing...
//# publish
module DiemRoot::Nop {
    public(script) fun nop() {}
}

// Give Alice some money to pay for transactions...
//# run --type-args 0x1::XUS::XUS --signers DesignatedDealer --args @Alice 100000 x"" x""
//#     -- 0x1::PaymentScripts::peer_to_peer_with_metadata

// Give Bob some money to pay for transactions...
//# run --type-args 0x1::XUS::XUS --signers DesignatedDealer --args @Bob 100000 x"" x""
//#     -- 0x1::PaymentScripts::peer_to_peer_with_metadata

// List of constants:
//      sliding nonce
//      global_memory_per_byte_cost
//      global_memory_per_byte_write_cost
//      min_transaction_gas_units
//      large_transaction_cutoff
//      intrinsic_gas_per_byte
//      maximum_number_of_gas_units
//      min_price_per_gas_unit
//      max_price_per_gas_unit
//      max_transaction_size_in_bytes
//      gas_unit_scaling_factor
//      default_account_size

// Wrong sender. Should fail.
//# run --args 0
//#            4
//#            9
//#            600
//#            600
//#            8
//#            4000000
//#            0
//#            10000
//#            4096
//#            1000
//#            800
//#     --signers TreasuryCompliance
//#     -- 0x1::SystemAdministrationScripts::set_gas_constants

// Min gas price greater than max gas price. Should fail.
//# run --args 0
//#            4
//#            9
//#            600
//#            600
//#            8
//#            4000000
//#            10
//#            9
//#            4096
//#            1000
//#            800
//#     --signers DiemRoot
//#     -- 0x1::SystemAdministrationScripts::set_gas_constants

//# run --signers Alice --gas-price 1 --gas-currency XUS -- 0xA550C18::Nop::nop

// Increase the min_transaction gas units and min_price_per_gas_unit
//# run --args 0
//#            4
//#            9
//#            6000
//#            600
//#            8
//#            4000000
//#            1
//#            10000
//#            4096
//#            1000
//#            800
//#     --signers DiemRoot
//#     -- 0x1::SystemAdministrationScripts::set_gas_constants

//# run --signers Bob --gas-price 1 --gas-currency XUS -- 0xA550C18::Nop::nop

//# run --admin-script --signers DiemRoot DiemRoot
script {
use DiemFramework::DiemAccount;
use DiemFramework::XUS::XUS;

fun main() {
    // Alice processed before the bump in min transaction gas units so should have more money left
    assert!(DiemAccount::balance<XUS>(@Bob) < DiemAccount::balance<XUS>(@Alice), 42);
}
}

// Can't process a transaction now with a gas price of zero since the lower bound was also changed.
//# run --signers Alice --gas-price 0 --gas-currency XUS -- 0xA550C18::Nop::nop
