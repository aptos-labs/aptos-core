module publisher::test {
    public fun foo() {
        let i = 0;
        while (i < 2) {
            let j = 0;
            while (j < 2) {
                let k = 0;
                while (k < 2) {
                    let l = 0;
                    while (l < 2) {
                        let m = 0;
                        while (m < 2) {
                            m = m + 1;
                        };
                        l = l + 1;
                    };
                    k = k + 1;
                };
                j = j + 1;
            };
            i = i + 1;
        };
    }
}
