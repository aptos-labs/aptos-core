//# init --parent-vasps Alice

// Check that the initial sequence number is 0.
//# run --admin-script --signers DiemRoot DiemRoot
script {
use DiemFramework::DiemAccount;

fun main() {
    assert!(DiemAccount::sequence_number(@Alice) == 0, 72);
}
}

// Bump the sequence number twice.
//# publish
module Alice::M1 {}


//# publish
module Alice::M2 {}

// Check that the initial sequence number is 2.
//# run --admin-script --signers DiemRoot DiemRoot
script {
use DiemFramework::DiemAccount;

fun main() {
    assert!(DiemAccount::sequence_number(@Alice) == 2, 72);
}
}


//# publish --sequence-number 1
module Alice::M3 {}
// Should fail since the sequence number is too old.


//# publish --sequence-number 3
module Alice::M4 {}
// Should fail since the sequence number is too new.


//# publish --sequence-number 2
module Alice::M5 {}
// Should succeed.
