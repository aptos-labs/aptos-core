module poc::string_generic {

    use std::string;

	public entry fun foo() {
        let s = string::utf8(b"Hello, world!");
        string::insert(&mut s, 0, string::utf8(b"ABCD"));
        let s2 = string::sub_string(&s, 0, 10);
        let _s3 = string::index_of(&s2, &string::utf8(b"world"));
	}
}
