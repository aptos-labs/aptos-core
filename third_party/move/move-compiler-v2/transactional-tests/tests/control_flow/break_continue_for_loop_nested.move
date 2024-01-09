//# run
script {
    fun main() {
        let k = 0;
        let y = 0;
        for (j in 0..10) { //j:  0,  1,  2,  3,  4, 5, 6, 7, 8, 9
            y = y + 1;     //y:  1,  4, 13, 16,
            if (y >= 15) break;
            y = y + 2;     //y:  3,  6, 15,  -
            let x = 0;
            for (i in 0..10) { //i:  0,  1,  2,  3,  4, 5, 6, 7, 8, 9
                x = x + 1;     //x:  1,  4, 13, 16,
                if (x >= 15) break;
                x = x + 2;     //x:  3,  6, 15,  -
                if (i % 2 == 0) continue;
                x = x * 2;     //x:  -, 12,  -,  -
            };
            assert!(x == 16, x);
            k = k + x;     //k: 16, 32, 48,  -
            if (j % 2 == 0) continue;
            y = y * 2;     //y:  -, 12,  -,  -
        };
        // k: 48, y: 16
        let z = 3 * k + y;
        // z: 208
        assert!(z == 160, z);
    }
}
