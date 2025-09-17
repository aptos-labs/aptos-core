module test::hello_fuzzer {
    public entry fun hello(input: vector<u8>) {
        if (input[0] == 0x48 /* h */) {
            if (input[1] == 0x65 /* e */) {
                if (input[2] == 0x6c /* l */) {
                    if (input[3] == 0x6c /* l */) {
                        if (input[4] == 0x6f /* 0 */) {
                            abort 42;
                        }
                    }
                }
            }
        }
    }
}
