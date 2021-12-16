//# init --parent-vasps Alice

//# publish
module DiemRoot::Test {
    public(script) fun nop() {}
}

// Should fail because the sequence number is too new.
//# run --signers Alice --sequence-number 5 -- 0xA550C18::Test::nop

// Running with 0 should succeed because sequence number wasn't bumped.
//# run --signers Alice --sequence-number 0 -- 0xA550C18::Test::nop
