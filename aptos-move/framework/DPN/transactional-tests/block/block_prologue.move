//# init --validators Vivian

//# block --proposer Vivian --time 1000000

//# run --admin-script
//#     --signers DiemRoot DiemRoot
script{
use DiemFramework::Timestamp;
use DiemFramework::Block;

fun main() {
    assert!(Block::get_current_block_height() == 1, 73);
    assert!(Timestamp::now_microseconds() == 1000000, 76);
}
}

//# run --admin-script
//#     --signers DiemRoot DiemRoot
script{
use DiemFramework::Timestamp;

fun main() {
    assert!(Timestamp::now_microseconds() != 2000000, 77);
}
}

// TODO: this transaction looks weird. We should figure out what the intention is and decide
//       what to do with it.
//# run --admin-script
//#     --signers DiemRoot Vivian
script{
use DiemFramework::Timestamp;

fun main(dr: signer, vv: signer) {
    Timestamp::update_global_time(&vv, @Vivian, 20);
}
}
