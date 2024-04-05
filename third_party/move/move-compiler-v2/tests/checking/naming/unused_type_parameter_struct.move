module 0x42::test {
	struct S0<T> {}

	struct S1<T1, T2> {}

	struct S2<T1, phantom T2> {
		f: S3<T2>,
	}

	struct S3<phantom T> {}
}
