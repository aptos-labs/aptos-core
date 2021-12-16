//# init --parent-vasps Bob

// Give Bob some money to pay for transactions...
//# run --type-args 0x1::XUS::XUS --signers DesignatedDealer --args @Bob 10000 x"" x""
//#     -- 0x1::PaymentScripts::peer_to_peer_with_metadata

//# publish
module DiemRoot::InfiniteLoop {
    public(script) fun run() { while (true) {} }
}

//# run --signers Bob --gas-budget 700 --gas-price 1 --gas-currency XUS
//#     -- 0xA550C18::InfiniteLoop::run

//# view --address Bob --resource 0x1::DiemAccount::Balance<0x1::XUS::XUS>

//# run --signers TreasuryCompliance --type-args 0x1::XUS::XUS --show-events
//#     -- 0x1::TreasuryComplianceScripts::burn_txn_fees

// No txn fee balance left to burn so this should fail.
//# run --signers TreasuryCompliance --type-args 0x1::XUS::XUS --show-events
//#     -- 0x1::TreasuryComplianceScripts::burn_txn_fees
