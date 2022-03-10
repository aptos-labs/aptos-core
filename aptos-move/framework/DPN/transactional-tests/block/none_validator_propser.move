//# init --parent-vasps Alice --validators Vivian

//# block --proposer Vivian --time 1000000

//# run --signers DiemRoot DiemRoot --admin-script
script{
use DiemFramework::Block;
use DiemFramework::Timestamp;

fun main() {
    assert!(Block::get_current_block_height() == 1, 77);
    assert!(Timestamp::now_microseconds() == 1000000, 78);
}
}

//# block --proposer Alice --time 1000000
