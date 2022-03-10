//# init --validators Vivian --parent-vasps Alice

// Changing the publishing option from Open to CustomScript
// Step 1: Make sure we can publish module Alice::Foo at the beginning
//# publish
module Alice::Foo {
    public fun foo() {}
}


// Initially, any script is allowed.
//# run --admin-script --signers DiemRoot DiemRoot
script {
use DiemFramework::TransactionPublishingOption;
fun main(dr: signer, _dr2: signer) {
    assert!(TransactionPublishingOption::is_script_allowed(&dr, &x""), 100);
}
}

// Turning off open scripts is a privileged operation.
//# run --admin-script --signers DiemRoot Vivian
script {
use DiemFramework::TransactionPublishingOption;
fun main(_dr: signer, vv: signer) {
    TransactionPublishingOption::set_open_script(&vv);
}
}

// TODO: double check on `TransactionPublishingOption::set_open_script`.
//     - The name seems confusing.
//     - Shall we send more transactions to test out its effects?


//# block --proposer Vivian --time 2

// Step 2: Change option to CustomModule
//# run --admin-script --signers DiemRoot DiemRoot
script {
use DiemFramework::TransactionPublishingOption;

fun main(dr: signer, _dr2: signer) {
    TransactionPublishingOption::set_open_module(&dr, false)
}
}

//# publish
module Alice::Bar {
    public fun bar() {}
}
