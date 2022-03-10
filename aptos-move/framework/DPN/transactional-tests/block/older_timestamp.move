//# init --validators Vivian

//# block --proposer Vivian --time 100000000

//# run --signers DiemRoot DiemRoot --admin-script
script{
use DiemFramework::Block;
use DiemFramework::Timestamp;

fun main() {
    assert!(Block::get_current_block_height() == 1, 76);
    assert!(Timestamp::now_microseconds() == 100000000, 77);
}
}

//# block --proposer Vivian --time 90000000
