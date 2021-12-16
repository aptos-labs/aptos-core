//# init --parent-vasps Alice

//# publish
module DiemRoot::Test {
    public(script) fun nop() {}
}

// Bump sequence number to 1.
//# run --signers Alice --sequence-number 0 -- 0xA550C18::Test::nop

// Should fail because the sequence number is too old.
//# run --signers Alice --sequence-number 0 -- 0xA550C18::Test::nop

// Running with 1 should succeed because sequence number wasn't bumped.
//# run --signers Alice --sequence-number 1 -- 0xA550C18::Test::nop
