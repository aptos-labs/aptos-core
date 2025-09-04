//# init --addresses Alice=0xf75daa73fc071f93593335eb9033da804777eb94491650dd3f095ce6f778acb6
//#      --private-keys Alice=56a26140eb233750cd14fb168c3eb4bd0782b099cde626ec8aff7f3cceb6364f

//# run --signers Alice --show-events
script {
    use std::vector;

    // should fail
    // Checks that default integers will always be u64 otherwise existing impls might fail
    // We're going above u64 max
    fun main() {
        let i = 1;
        let j = 1;
        while (j < 65) {
            i = 2 * i;
            j = j + 1;
        };
	let v = vector<u64>[1, 2, 3];
	assert!(*vector::borrow<u64>(&v, 5) == 5, 0);
    }
}
