//# publish
module 0x42::M {
    const B0: vector<bool> = vector[];
    const B1: vector<bool> = vector[true];
    const B2: vector<bool> = vector[true && false];
    const B3: vector<bool> = vector[true, true && false, true == true];

    const E0: bool = vector<u8>[] == vector[];
    const E1: bool = vector[0] == vector[1, 100];

    const E3: vector<vector<u8>> = vector[vector[1], vector[2]];

    fun foo() {
        assert!(vector[vector[1], vector[2]] == E3, 0);
    }
}

//#run 0x42::M::foo
