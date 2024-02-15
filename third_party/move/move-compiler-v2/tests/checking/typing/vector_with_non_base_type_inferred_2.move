module 0x42::Test {
    fun t() {
        // test invalid vector instatiation, inferred type
        vector[&0];
    }

    fun t_1() {
        // test invalid vector instatiation, inferred type
        vector[&mut 0];
    }

    fun t_2() {
        // test invalid vector instatiation, inferred type
        vector[()];
    }

    fun t_4() {
        // test invalid vector instatiation, inferred type
        vector[(0, false)];
    }

    fun t_5() {
        // tests valid subtyping join... although not important at the moment
        vector[&0, &mut 0];
    }

    fun t_6() {
        // tests valid subtyping join... although not important at the moment
        vector[&mut 0, &0];
    }

}
