// `==` and `!=` on two signers; the golden pins how the comparison lowers. Each unequal pair
// differs in one address byte only (byte 31, 0, 8 or 16 from the left, one in each 8-byte
// quarter), so a comparison that skips any part of the address reports the pair equal.

// RUN: publish --print(stackless,micro-ops)
module 0x42::signer_eq {
    public fun same(left: signer, right: signer): bool { left == right }
    public fun differ(left: signer, right: signer): bool { left != right }
}

// RUN: execute 0x42::signer_eq::same --args 0x42, 0x42
// CHECK: results: true

// RUN: execute 0x42::signer_eq::same --args 0x42, 0x43
// CHECK: results: false

// RUN: execute 0x42::signer_eq::same --args 0x42, 0x1000000000000000000000000000000000000000000000000000000000000042
// CHECK: results: false

// RUN: execute 0x42::signer_eq::same --args 0x42, 0x0000000000000000100000000000000000000000000000000000000000000042
// CHECK: results: false

// RUN: execute 0x42::signer_eq::same --args 0x42, 0x0000000000000000000000000000000010000000000000000000000000000042
// CHECK: results: false

// RUN: execute 0x42::signer_eq::differ --args 0x42, 0x42
// CHECK: results: false

// RUN: execute 0x42::signer_eq::differ --args 0x42, 0x43
// CHECK: results: true

// RUN: execute 0x42::signer_eq::differ --args 0x42, 0x1000000000000000000000000000000000000000000000000000000000000042
// CHECK: results: true

// RUN: execute 0x42::signer_eq::differ --args 0x42, 0x0000000000000000100000000000000000000000000000000000000000000042
// CHECK: results: true

// RUN: execute 0x42::signer_eq::differ --args 0x42, 0x0000000000000000000000000000000010000000000000000000000000000042
// CHECK: results: true
