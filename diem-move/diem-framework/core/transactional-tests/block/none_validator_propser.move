//# init --parent-vasps Alice --validators Vivian

//# block --proposer Vivian --time 1000000

//# run --signers DiemRoot DiemRoot --admin-script
script{
use DiemFramework::DiemBlock;
use DiemFramework::DiemTimestamp;

fun main() {
    assert!(DiemBlock::get_current_block_height() == 1, 77);
    assert!(DiemTimestamp::now_microseconds() == 1000000, 78);
}
}

//# block --proposer Alice --time 1000000
