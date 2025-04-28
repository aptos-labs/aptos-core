module std::modules {
    enum S1 { Inner }
    enum S2<T> { One }

    fun main(s: S2<u8>) {
        // crashes the compiler
        s is S1;
        s is S2<u8>;
        s is S2<S1::Inner>;

    }
}
