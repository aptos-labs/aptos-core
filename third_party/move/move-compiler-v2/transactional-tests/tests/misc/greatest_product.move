//# publish
module 0xc0ffee::greatest_product {
    use std::vector;

    const THOUSAND_DIGIT_NUMBER: vector<u8> = vector[
        7, 3, 1, 6, 7, 1, 7, 6, 5, 3, 1, 3, 3, 8, 7, 1, 5, 9, 9, 5,
        9, 6, 9, 3, 2, 9, 8, 5, 3, 5, 7, 9, 3, 7, 2, 0, 0, 6, 3, 1,
        7, 0, 1, 8, 8, 6, 7, 9, 8, 0, 8, 2, 8, 0, 8, 5, 1, 3, 2, 8,
        7, 0, 3, 5, 0, 6, 6, 4, 8, 0, 7, 6, 7, 8, 8, 0, 1, 1, 6, 9,
        5, 2, 3, 6, 1, 5, 6, 3, 0, 3, 8, 8, 6, 2, 3, 8, 8, 1, 7, 7,
        4, 6, 2, 8, 2, 7, 9, 9, 4, 7, 5, 6, 4, 4, 7, 9, 9, 5, 8, 4,
        7, 3, 4, 6, 7, 9, 9, 5, 3, 7, 1, 7, 6, 5, 2, 3, 7, 9, 1, 9,
        9, 3, 5, 1, 9, 9, 7, 7, 8, 0, 0, 9, 8, 7, 6, 9, 7, 7, 7, 9,
        9, 6, 7, 6, 9, 5, 5, 4, 8, 3, 7, 5, 5, 1, 5, 5, 8, 4, 4, 4
    ];

    public fun find_greatest_product(): u64 {
        let max_product = 0;
        let i = 0;
        let length = vector::length(&THOUSAND_DIGIT_NUMBER);

        while (i <= length - 4) {
            let product = (*vector::borrow(&THOUSAND_DIGIT_NUMBER, i) as u64) *
                          (*vector::borrow(&THOUSAND_DIGIT_NUMBER, i + 1) as u64) *
                          (*vector::borrow(&THOUSAND_DIGIT_NUMBER, i + 2) as u64) *
                          (*vector::borrow(&THOUSAND_DIGIT_NUMBER, i + 3) as u64);

            if (product > max_product) {
                max_product = product;
            };

            i = i + 1;
        };

        max_product
    }

    public fun test_find_greatest_product() {
        assert!(find_greatest_product() == 3969, 0);
    }
}

//# run 0xc0ffee::greatest_product::test_find_greatest_product
