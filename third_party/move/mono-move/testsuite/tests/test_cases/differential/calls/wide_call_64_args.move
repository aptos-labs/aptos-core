// A call with 64 arguments. Every argument is distinct and the callee returns them in order,
// so a dropped or misplaced argument changes the result. The golden pins how the call lowers.

// RUN: publish --print(stackless)
module 0x42::wide_call {
    fun wide(
        a0: u64, a1: u64, a2: u64, a3: u64, a4: u64, a5: u64, a6: u64, a7: u64, a8: u64, a9: u64,
        a10: u64, a11: u64, a12: u64, a13: u64, a14: u64, a15: u64, a16: u64, a17: u64, a18: u64,
        a19: u64, a20: u64, a21: u64, a22: u64, a23: u64, a24: u64, a25: u64, a26: u64, a27: u64,
        a28: u64, a29: u64, a30: u64, a31: u64, a32: u64, a33: u64, a34: u64, a35: u64, a36: u64,
        a37: u64, a38: u64, a39: u64, a40: u64, a41: u64, a42: u64, a43: u64, a44: u64, a45: u64,
        a46: u64, a47: u64, a48: u64, a49: u64, a50: u64, a51: u64, a52: u64, a53: u64, a54: u64,
        a55: u64, a56: u64, a57: u64, a58: u64, a59: u64, a60: u64, a61: u64, a62: u64, a63: u64
    ): vector<u64> {
        vector[
            a0, a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11, a12, a13, a14, a15, a16, a17, a18,
            a19, a20, a21, a22, a23, a24, a25, a26, a27, a28, a29, a30, a31, a32, a33, a34, a35,
            a36, a37, a38, a39, a40, a41, a42, a43, a44, a45, a46, a47, a48, a49, a50, a51, a52,
            a53, a54, a55, a56, a57, a58, a59, a60, a61, a62, a63
        ]
    }

    public fun call_wide(base: u64): vector<u64> {
        wide(
            base, base + 1, base + 2, base + 3, base + 4, base + 5, base + 6, base + 7, base + 8,
            base + 9, base + 10, base + 11, base + 12, base + 13, base + 14, base + 15, base + 16,
            base + 17, base + 18, base + 19, base + 20, base + 21, base + 22, base + 23, base + 24,
            base + 25, base + 26, base + 27, base + 28, base + 29, base + 30, base + 31, base + 32,
            base + 33, base + 34, base + 35, base + 36, base + 37, base + 38, base + 39, base + 40,
            base + 41, base + 42, base + 43, base + 44, base + 45, base + 46, base + 47, base + 48,
            base + 49, base + 50, base + 51, base + 52, base + 53, base + 54, base + 55, base + 56,
            base + 57, base + 58, base + 59, base + 60, base + 61, base + 62, base + 63
        )
    }
}

// RUN: execute 0x42::wide_call::call_wide --args 1000
// CHECK: results: [1000, 1001, 1002, 1003, 1004, 1005, 1006, 1007, 1008, 1009, 1010, 1011, 1012, 1013, 1014, 1015, 1016, 1017, 1018, 1019, 1020, 1021, 1022, 1023, 1024, 1025, 1026, 1027, 1028, 1029, 1030, 1031, 1032, 1033, 1034, 1035, 1036, 1037, 1038, 1039, 1040, 1041, 1042, 1043, 1044, 1045, 1046, 1047, 1048, 1049, 1050, 1051, 1052, 1053, 1054, 1055, 1056, 1057, 1058, 1059, 1060, 1061, 1062, 1063]
