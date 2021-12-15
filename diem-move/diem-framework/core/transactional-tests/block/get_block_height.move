//# init --validators Vivian

//# run --admin-script --signers DiemRoot DiemRoot
script{
use DiemFramework::DiemBlock;
fun main() {
    // check that the height of the initial block is zero
    assert!(DiemBlock::get_current_block_height() == 0, 77);
}
}

//# block --proposer Vivian --time 100000000

//# run --admin-script --signers DiemRoot DiemRoot
script{
use DiemFramework::DiemBlock;
use DiemFramework::DiemTimestamp;

fun main() {
    assert!(DiemBlock::get_current_block_height() == 1, 76);
    assert!(DiemTimestamp::now_microseconds() == 100000000, 80);
}
}

//# block --proposer Vivian --time 101000000

//# run --admin-script --signers DiemRoot DiemRoot
script{
use DiemFramework::DiemBlock;
use DiemFramework::DiemTimestamp;

fun main() {
    assert!(DiemBlock::get_current_block_height() == 2, 76);
    assert!(DiemTimestamp::now_microseconds() == 101000000, 80);
}
}
