spec std::string {
    spec internal_check_utf8(v: &vector<u8>): bool {
        pragma opaque;
        aborts_if [abstract] false;
        ensures [abstract] result == spec_internal_check_utf8(v);
    }

    spec internal_is_char_boundary(v: &vector<u8>, i: u64): bool {
        pragma opaque;
        aborts_if [abstract] false;
        ensures [abstract] result == spec_internal_is_char_boundary(v, i);
    }
    spec internal_sub_string(v: &vector<u8>, i: u64, j: u64): vector<u8> {
        pragma opaque;
        aborts_if [abstract] false;
        ensures [abstract] result == spec_internal_sub_string(v, i, j);
    }
    spec internal_index_of(v: &vector<u8>, r: &vector<u8>): u64 {
        pragma opaque;
        aborts_if [abstract] false;
        ensures [abstract] result == spec_internal_index_of(v, r);
    }
    spec fun spec_utf8(bytes: vector<u8>): String {
        String{bytes}
    }

    spec module {
        fun spec_internal_check_utf8(v: vector<u8>): bool;
        fun spec_internal_is_char_boundary(v: vector<u8>, i: u64): bool;
        fun spec_internal_sub_string(v: vector<u8>, i: u64, j: u64): vector<u8>;
        fun spec_internal_index_of(v: vector<u8>, r: vector<u8>): u64;
    }
}
