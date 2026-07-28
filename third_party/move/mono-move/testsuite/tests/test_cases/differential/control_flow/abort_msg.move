// RUN: publish
module 0x1::test {
    fun abort_with_message() {
        abort b"boom"
    }
}

// RUN: execute 0x1::test::abort_with_message
// CHECK: aborted: code 14566554180833181696 (boom) in 0x1::test
