//# init --parent-vasps Alice

// Send a transaction with metadata and make sure we see it in the PaymentReceivedEvent.
//
//# run --type-args 0x1::XUS::XUS --signers DesignatedDealer --args @Alice 1000000 x"deadbeef" x""
//#     --show-events
//#     -- 0x1::PaymentScripts::peer_to_peer_with_metadata
