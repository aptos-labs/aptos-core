// RUN: publish --print(bytecode,stackless,micro-ops)
module 0x1::test {
    fun do_abort(code: u64) {
        abort code
    }
}

// RUN: execute 0x1::test::do_abort --args 42
// CHECK: aborted: code 42

// RUN: execute 0x1::test::do_abort --args 0
// CHECK: aborted: code 0
