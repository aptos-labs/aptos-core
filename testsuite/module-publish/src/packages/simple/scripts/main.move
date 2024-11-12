script {
    // Note: this constant can be replaced in compiled script to make it hash to a different value.
    const SENDER: address = @0x1;

    fun main(_sender: &signer) {
        // The idea is to to ensure that this script takes some time to be deserialized and verified, but the actual
        // execution time is small (no-op).
        let count = 23;
        while (count > 0) {
            count = count - 1;
        };

        let a;
        let b = 0;
        let c;
        let d = 0;
        while (count > 0) {
            count = count - 1;

            a = b + 1;
            c = d + 1;
            b = a + 1;
            d = b - a;
            b = c + 1;
            a = b - c;
            b = a + 1;

            // can never be true
            if (a > b && b > c && c > d && d > a) {
                count = count + 1;
            }
        };

        let count = 5;
        let vec = std::vector::empty<u64>();
        let i = 0;
        while (i < 20) {
            std::vector::push_back(&mut vec, i);
            i = i + 1;
        };

        let sum: u64 = 0;

        while (count > 0) {
            let val = std::bcs::to_bytes(&vec);
            sum = sum + ((*std::vector::borrow(&val, 0)) as u64);
            count = count - 1;
        };

        if (count == 1000000) {
            while (true) {}
        }
    }
}
