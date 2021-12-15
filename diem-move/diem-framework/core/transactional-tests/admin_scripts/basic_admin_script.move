//# init --parent-vasps Bob

//# run --admin-script --signers DiemRoot Bob
script {
use Std::Signer;
fun main(dr: signer, bob: signer) {
    assert!(Signer::address_of(&dr) == @DiemRoot, 0);
    assert!(Signer::address_of(&bob) == @Bob, 1);
}
}

//# run --admin-script --signers TreasuryCompliance Bob
// Should be rejected because the first signer is not DiemRoot.
script {
use Std::Signer;
fun main(dr: signer, bob: signer) {
    assert!(Signer::address_of(&dr) == @TreasuryCompliance, 0);
    assert!(Signer::address_of(&bob) == @Bob, 1);
}
}

//# run --admin-script --signers DiemRoot Bob
//#     --private-key 7ae3a1e5fbf7da8d5aa8923c2ef8e00ba2b8d5ae6dd47d1b559c5de19e772833
// Should be rejected due to incorrect signing key.
script {
use Std::Signer;
fun main(dr: signer, bob: signer) {
    assert!(Signer::address_of(&dr) == @DiemRoot, 0);
    assert!(Signer::address_of(&bob) == @Bob, 1);
}
}
