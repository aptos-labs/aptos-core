//# init
// TODO: The test harness will panic if the test starts with a comment.
//       Fix the bug and remove the dummy init command.

// Admin scripts should be signed using the key pair of the first signer. You technically are
// allowed to provide a different private key, but if you do so, the transaction will likely
// be rejected.
//
//# run --signers DiemRoot 0xAA
//#     --private-key 56a26140eb233750cd14fb168c3eb4bd0782b099cde626ec8aff7f3cceb6364f
//#     --admin-script
script {
    fun main() {}
}
