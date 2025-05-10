module 0x815::m {

    enum S1<phantom T> { One }

    fun infer_phantom(first: S1<u8>) {
        first is One;
        first is S1::One;
        first is One<u8>;
        first is S1::One<u8>;
    }

    enum S2<T> {Two{inner: S1<T>}}
    enum Unused<phantom T> { Two }

    fun infer_nested<G>(first: S2<u8>, second : S2<G>) {
        first is Two;
        second is Two;
        first is S2::Two;
        second is S2::Two<G>;
        first.inner is One;
        second.inner is One;
        first.inner is One<u8>;
        second.inner is One<G>;
    }

    enum S3<T> {Three{ inner : S1<T>}, Four{ inner : S1<u8>}}
    enum S4<T> {Five{ inner : S1<T>}}

    fun infer_nested_choice<G>(first: S3<G>, second: S4<G>) {
        first is Three|Four;
        second.inner is One<G>;
    }
}
