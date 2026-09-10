// Differential test for the test-only unit_test native.

// RUN: publish
module 0x1::unit_test {
    use std::signer;
    use std::vector;

    public fun count(n: u64): u64 {
        let signers = create_signers_for_testing(n);
        vector::length(&signers)
    }

    public fun equals(): bool {
        let signers = create_signers_for_testing(4);
        let a = *signer::borrow_address(vector::borrow(&signers, 0)) == @0x0;
        let b = *signer::borrow_address(vector::borrow(&signers, 1)) == @0x0100000000000000000000000000000000000000000000000000000000000000;
        let c = *signer::borrow_address(vector::borrow(&signers, 2)) == @0x0200000000000000000000000000000000000000000000000000000000000000;
        let d = *signer::borrow_address(vector::borrow(&signers, 3)) == @0x0300000000000000000000000000000000000000000000000000000000000000;
        a && b && c && d
    }

    public fun distinct(): bool {
        let signers = create_signers_for_testing(3);
        let a = *signer::borrow_address(vector::borrow(&signers, 0));
        let b = *signer::borrow_address(vector::borrow(&signers, 1));
        let c = *signer::borrow_address(vector::borrow(&signers, 2));
        a != b && a != c && b != c
    }

    native fun create_signers_for_testing(num_signers: u64): vector<signer>;
}

// RUN: execute 0x1::unit_test::count --args 0
// CHECK: results: 0

// RUN: execute 0x1::unit_test::count --args 3
// CHECK: results: 3

// RUN: execute 0x1::unit_test::equals
// CHECK: results: true

// RUN: execute 0x1::unit_test::distinct
// CHECK: results: true
