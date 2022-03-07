//# init --parent-vasps Alice Bob

// Give Alice some money...
//
//# run --type-args 0x1::XUS::XUS --signers DesignatedDealer --args @Alice 1000 x"" x""
//#     -- 0x1::PaymentScripts::peer_to_peer_with_metadata

// Alice sends some money to Bob using a multi agent script.
//
//# run --signers Alice
//#     --secondary-signers Bob
//#     --type-args 0x1::XUS::XUS
//#     --args 10 x""
//#     -- 0x1::PaymentScripts::peer_to_peer_by_signers
