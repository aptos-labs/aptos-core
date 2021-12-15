//# init --validators Vivian

//# block --proposer Vivian --time 1000000

//# run --admin-script
//#     --signers DiemRoot
script{
use DiemFramework::DiemTimestamp;
use DiemFramework::DiemBlock;

fun main() {
    assert!(DiemBlock::get_current_block_height() == 1, 73);
    assert!(DiemTimestamp::now_microseconds() == 1000000, 76);
}
}

//# run --admin-script
//#     --signers DiemRoot
script{
use DiemFramework::DiemTimestamp;

fun main() {
    assert!(DiemTimestamp::now_microseconds() != 2000000, 77);
}
}

// TODO: this transaction looks weird. We should figure out what the intention is and decide
//       what to do with it.
//# run --admin-script
//#     --signers Vivian
script{
use DiemFramework::DiemTimestamp;

fun main(dr: signer, vv: signer) {
    DiemTimestamp::update_global_time(&vv, @Vivian, 20);
}
}
