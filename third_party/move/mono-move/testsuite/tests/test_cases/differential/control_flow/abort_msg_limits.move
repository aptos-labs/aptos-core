// Abort message edge cases: empty (literal, never-pushed, drained), exactly 1024 bytes,
// 1025 bytes and more (ABORT_MESSAGE_LIMIT_EXCEEDED), invalid and truncated UTF-8
// (INVALID_ABORT_MESSAGE), and a multi-byte character.

// RUN: publish
module 0x1::abort_msg_limits {
    use std::vector;

    fun bytes(count: u64, byte: u8): vector<u8> {
        let msg = vector::empty<u8>();
        let index = 0;
        while (index < count) { vector::push_back(&mut msg, byte); index = index + 1; };
        msg
    }

    fun empty_literal() { abort b"" }

    fun empty_runtime() { abort vector::empty<u8>() }

    fun drained() {
        let msg = b"x";
        vector::pop_back(&mut msg);
        abort msg
    }

    fun sized(count: u64) { abort bytes(count, 0x61) }

    fun invalid_utf8() { abort x"ff" }

    fun truncated_utf8() { abort x"e282" }

    fun multibyte() { abort x"e282ac" }

}

// RUN: execute 0x1::abort_msg_limits::empty_literal
// CHECK: aborted: code 14566554180833181696 () in 0x1::abort_msg_limits

// RUN: execute 0x1::abort_msg_limits::empty_runtime
// CHECK: aborted: code 14566554180833181696 () in 0x1::abort_msg_limits

// RUN: execute 0x1::abort_msg_limits::drained
// CHECK: aborted: code 14566554180833181696 () in 0x1::abort_msg_limits

// RUN: execute 0x1::abort_msg_limits::sized --args 1024
// CHECK-SUBSTR: aborted: code 14566554180833181696 (aaaaaaaa
// CHECK-SUBSTR: aaaaaaaa) in 0x1::abort_msg_limits

// RUN: execute 0x1::abort_msg_limits::sized --args 1025
// CHECK-V1-SUBSTR: ABORT_MESSAGE_LIMIT_EXCEEDED
// CHECK-V2-SUBSTR: AbortMsg: message size 1025 exceeds maximum 1024
// CHECK-ERROR-PARITY

// RUN: execute 0x1::abort_msg_limits::sized --args 5000
// CHECK-V1-SUBSTR: ABORT_MESSAGE_LIMIT_EXCEEDED
// CHECK-V2-SUBSTR: AbortMsg: message size 5000 exceeds maximum 1024
// CHECK-ERROR-PARITY

// RUN: execute 0x1::abort_msg_limits::invalid_utf8
// CHECK-V1-SUBSTR: INVALID_ABORT_MESSAGE
// CHECK-V2-SUBSTR: AbortMsg: message is not valid UTF-8
// CHECK-ERROR-PARITY

// RUN: execute 0x1::abort_msg_limits::truncated_utf8
// CHECK-V1-SUBSTR: INVALID_ABORT_MESSAGE
// CHECK-V2-SUBSTR: AbortMsg: message is not valid UTF-8
// CHECK-ERROR-PARITY

// RUN: execute 0x1::abort_msg_limits::multibyte
// CHECK: aborted: code 14566554180833181696 (€) in 0x1::abort_msg_limits
