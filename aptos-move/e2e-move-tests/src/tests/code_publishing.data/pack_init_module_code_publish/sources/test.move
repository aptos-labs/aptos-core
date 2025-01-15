module 0xcafe::test {
    use aptos_framework::code;

    fun init_module(s: &signer) {
        // The following metadata and code corresponds to an immutable package called `Package` with compatibility
        // checks. Code:
        //   module 0xcafe::m {
        //       public fun f() {}
        //   }
        let metadata: vector<u8> = vector[7, 80, 97, 99, 107, 97, 103, 101, 1, 0, 0, 0, 0, 0, 0, 0, 0, 64, 68, 56, 49, 69, 55, 68, 70, 69, 70, 54, 51, 52, 66, 50, 56, 56, 49, 69, 48, 48, 51, 69, 67, 70, 49, 54, 66, 54, 66, 69, 53, 53, 66, 69, 57, 49, 54, 54, 55, 53, 65, 65, 66, 66, 50, 67, 57, 52, 70, 55, 56, 52, 54, 67, 56, 70, 57, 55, 68, 49, 50, 57, 54, 65, 107, 31, 139, 8, 0, 0, 0, 0, 0, 2, 255, 37, 138, 203, 9, 128, 48, 16, 68, 239, 91, 133, 164, 0, 177, 1, 123, 240, 30, 68, 214, 236, 32, 193, 124, 150, 68, 5, 187, 55, 65, 230, 52, 239, 61, 171, 236, 78, 62, 176, 82, 226, 136, 97, 30, 204, 242, 3, 67, 15, 74, 245, 57, 117, 54, 141, 109, 134, 110, 61, 10, 11, 54, 205, 193, 187, 183, 11, 151, 163, 242, 229, 247, 208, 122, 203, 34, 5, 181, 162, 174, 68, 86, 160, 72, 130, 228, 124, 255, 31, 84, 65, 171, 55, 103, 0, 0, 0, 1, 1, 109, 0, 0, 0, 0, 0];
        let code: vector<vector<u8>> = vector[vector[161, 28, 235, 11, 7, 0, 0, 10, 6, 1, 0, 2, 3, 2, 6, 5, 8, 1, 7, 9, 4, 8, 13, 32, 12, 45, 7, 0, 0, 0, 1, 0, 0, 0, 1, 0, 1, 109, 1, 102, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 202, 254, 0, 1, 0, 0, 0, 1, 2, 0]];

        code::publish_package_txn(s, metadata, code)
    }
}
