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
        String { bytes }
    }

    spec utf8(bytes: vector<u8>): String {
        pragma opaque;
        aborts_if !spec_internal_check_utf8(bytes);
        ensures result == spec_utf8(bytes);
    }

    spec module {
        fun spec_internal_check_utf8(v: vector<u8>): bool;

        fun spec_internal_is_char_boundary(v: vector<u8>, i: u64): bool;

        fun spec_internal_sub_string(v: vector<u8>, i: u64, j: u64): vector<u8>;

        fun spec_internal_index_of(v: vector<u8>, r: vector<u8>): u64;
    }

    spec index_of(self: &0x1::string::String, r: &0x1::string::String): u64 {
        pragma opaque = true;
        ensures [inferred] result == spec_internal_index_of(self.bytes, r.bytes);
        aborts_if [inferred] false;
    }

    spec append(self: &mut 0x1::string::String, r: 0x1::string::String) {
        pragma opaque = true;
        ensures [inferred] self
            == update_field(
                old(self),
                bytes,
                concat(old(self).bytes, r.bytes)
            );
        aborts_if [inferred] false;
    }

    spec insert(
        self: &mut 0x1::string::String, at: u64, o: 0x1::string::String
    ) {
        pragma opaque = true, aborts_if_is_partial = true;
        ensures [inferred] at <= len(old(self).bytes)
            && !spec_internal_is_char_boundary(old(self).bytes, at) ==>
            self == old(self);
        ensures [inferred] at > len(old(self).bytes) ==> self == old(self);
        aborts_if [inferred] at <= len(self.bytes)
            && (
                spec_internal_is_char_boundary(self.bytes, at)
                    && aborts_of<sub_string>(self, 0, at)
            );
        aborts_if [inferred] at <= len(self.bytes)
            && !spec_internal_is_char_boundary(self.bytes, at);
        aborts_if [inferred] at > len(self.bytes);
    }

    spec is_empty(self: &0x1::string::String): bool {
        use 0x1::vector;
        pragma opaque = true, aborts_if_is_partial = true;
        ensures [inferred] result == vector::is_empty<u8>(self.bytes);
    }

    spec length(self: &0x1::string::String): u64 {
        pragma opaque = true;
        ensures [inferred] result == len(self.bytes);
        aborts_if [inferred] false;
    }

    spec append_utf8(self: &mut 0x1::string::String, bytes: vector<u8>) {
        pragma opaque = true;
        ensures [inferred] ensures_of<append>(self, utf8(bytes), self);
        aborts_if [inferred] aborts_of<utf8>(bytes);
    }

    spec bytes(self: &0x1::string::String): &vector<u8> {
        pragma opaque = true;
        ensures [inferred] result == self.bytes;
        aborts_if [inferred] false;
    }

    spec into_bytes(self: 0x1::string::String): vector<u8> {
        pragma opaque = true;
        ensures [inferred] result == self.bytes;
        aborts_if [inferred] false;
    }

    spec sub_string(self: &0x1::string::String, i: u64, j: u64): 0x1::string::String {
        pragma opaque = true;
        ensures [inferred] j <= len(self.bytes)
            && (
                i <= j
                    && (
                        spec_internal_is_char_boundary(self.bytes, i)
                            && spec_internal_is_char_boundary(self.bytes, j)
                    )
            ) ==>
            result == String {
                bytes: spec_internal_sub_string(self.bytes, i, j)
            };
        aborts_if [inferred] j <= len(self.bytes)
            && (
                i <= j
                    && (
                        spec_internal_is_char_boundary(self.bytes, i)
                            && !spec_internal_is_char_boundary(self.bytes, j)
                    )
            );
        aborts_if [inferred] j <= len(self.bytes)
            && (i <= j && !spec_internal_is_char_boundary(self.bytes, i));
        aborts_if [inferred] j <= len(self.bytes) && i > j;
        aborts_if [inferred] j > len(self.bytes);
    }

    spec try_utf8(bytes: vector<u8>): 0x1::option::Option<0x1::string::String> {
        use 0x1::option;
        pragma opaque = true;
        ensures [inferred] spec_internal_check_utf8(bytes) ==>
            result == option::some<String>(String { bytes: bytes });
        ensures [inferred]!spec_internal_check_utf8(bytes) ==>
            result == option::none<String>();
        aborts_if [inferred] false;
    }
}
