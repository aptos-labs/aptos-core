//# init --parent-vasps Alice

//# run --type-args 0x1::XUS::XUS --signers DesignatedDealer --args @Alice 5000 x"" x""
//#     -- 0x1::PaymentScripts::peer_to_peer_with_metadata

//# publish
// Dummy module for testing...
module DiemRoot::Nop {
    public(script) fun nop() {}
}

//# run --signers Alice --gas-price 1 --gas-budget 5001
//#     -- 0x1::Nop::nop
