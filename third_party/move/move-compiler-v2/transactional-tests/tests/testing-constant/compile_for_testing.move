//# publish --print-bytecode
module 0x66::m {
    fun run() {
        if (__COMPILE_FOR_TESTING__) abort 1 else abort 2
    }
}

//#run 0x66::m::run
