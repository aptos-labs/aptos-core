module 0x815::m {

    enum S1<phantom T> { One }
    enum S2 { One }

    fun incorrect_generic(s: S1<u8>) {
        s is S1<u16>;
    }

    fun incorrect_enum(s : S1<u8>) {
        s is S2;
    }

    fun non_empty_arguments_for_empty_expected(s : S2) {
        s is S2<u8>;
    }

    enum S3<T> {Three{ inner : S1<T>}, Four{ inner : S1<u8>}}

    fun infer_nested_choice<G>(first: S3<G>,) {
        first.inner is One<G>|One<u8>;
    }
}
