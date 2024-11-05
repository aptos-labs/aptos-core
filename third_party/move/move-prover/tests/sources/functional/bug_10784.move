module 0x42::m {

    struct RistrettoPoint has drop {
        handle: u64
    }

    struct CompressedRistretto has copy, store, drop {
        data: vector<u8>
    }

    native fun point_compress_internal(point: &RistrettoPoint): vector<u8>;

    spec fun spec_point_compress_internal(point: &RistrettoPoint): vector<u8>;

    spec point_compress_internal(point: &RistrettoPoint): vector<u8> {
        pragma opaque;
        ensures result == spec_point_compress_internal(point);
    }

    public fun point_compress(point: &RistrettoPoint): CompressedRistretto {
        CompressedRistretto {
            data: point_compress_internal(point)
        }
    }

}
