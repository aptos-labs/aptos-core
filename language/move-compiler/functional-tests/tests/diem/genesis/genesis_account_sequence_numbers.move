script {
use DiemFramework::DiemAccount;

fun main() {
  // check that the sequence number of the Association account (which sent the genesis txn) has not been
  // incremented...
  assert!(DiemAccount::sequence_number(@DiemRoot) == 0, 66);
}
}
