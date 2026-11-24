// ristretto255 natives that combine scalars with points, checked against small multiples of
// the basepoint B from the RFC 9496 test vectors (B[k] = k * B). The operands are distinct,
// so a swapped or misread operand gives a different multiple. In-place multiplication is
// observed by rereading the destination and a neighbouring point. Multi-scalar multiplication
// also runs with one term and with 200, the latter compared with B times the expected total.

// RUN: publish
module 0x1::ristretto255 {
    struct Scalar has copy, store, drop {
        data: vector<u8>
    }

    struct RistrettoPoint has drop {
        handle: u64
    }

    native fun scalar_from_u64_internal(num: u64): vector<u8>;

    native fun point_compress_internal(point: &RistrettoPoint): vector<u8>;

    native fun point_mul_internal(point: &RistrettoPoint, scalar: vector<u8>, in_place: bool): u64;

    native fun point_equals(left: &RistrettoPoint, right: &RistrettoPoint): bool;

    native fun basepoint_mul_internal(scalar: vector<u8>): u64;

    native fun basepoint_double_mul_internal(
        point_scalar: vector<u8>, some_point: &RistrettoPoint, basepoint_scalar: vector<u8>
    ): u64;

    native fun double_scalar_mul_internal(
        point1: u64, point2: u64, scalar1: vector<u8>, scalar2: vector<u8>
    ): u64;

    native fun multi_scalar_mul_internal<P, S>(points: &vector<P>, scalars: &vector<S>): u64;

    fun scalar(value: u64): vector<u8> {
        scalar_from_u64_internal(value)
    }

    fun times_b(multiple: u64): RistrettoPoint {
        RistrettoPoint { handle: basepoint_mul_internal(scalar(multiple)) }
    }

    // Multiplication stores 5B in the destination and leaves the neighbouring
    // point at B.
    public fun mul_in_place(): (bool, vector<u8>, vector<u8>) {
        let dest = times_b(1);
        let neighbour = times_b(1);
        let returned = point_mul_internal(&dest, scalar(5), true);
        (returned == dest.handle, point_compress_internal(&dest), point_compress_internal(&neighbour))
    }

    // The source keeps B and the product gets a new handle.
    public fun mul_out_of_place(): (bool, vector<u8>, vector<u8>) {
        let source = times_b(1);
        let product = RistrettoPoint { handle: point_mul_internal(&source, scalar(5), false) };
        (
            product.handle != source.handle,
            point_compress_internal(&source),
            point_compress_internal(&product)
        )
    }

    // 2 * (3B) + 5 * B == 11B.
    public fun basepoint_double_mul(): vector<u8> {
        let three_b = times_b(3);
        let result = RistrettoPoint {
            handle: basepoint_double_mul_internal(scalar(2), &three_b, scalar(5))
        };
        point_compress_internal(&result)
    }

    // 3 * (2B) + 1 * (3B) == 9B.
    public fun double_scalar_mul(): vector<u8> {
        let two_b = times_b(2);
        let three_b = times_b(3);
        let result = RistrettoPoint {
            handle: double_scalar_mul_internal(two_b.handle, three_b.handle, scalar(3), scalar(1))
        };
        point_compress_internal(&result)
    }

    // 1 * (5B) + 4 * (2B) == 13B.
    public fun multi_scalar_mul_two(): vector<u8> {
        let points = vector[times_b(5), times_b(2)];
        let scalars = vector[Scalar { data: scalar(1) }, Scalar { data: scalar(4) }];
        let result = RistrettoPoint {
            handle: multi_scalar_mul_internal<RistrettoPoint, Scalar>(&points, &scalars)
        };
        point_compress_internal(&result)
    }

    // 2 * (7B) == 14B.
    public fun multi_scalar_mul_one(): vector<u8> {
        let points = vector[times_b(7)];
        let scalars = vector[Scalar { data: scalar(2) }];
        let result = RistrettoPoint {
            handle: multi_scalar_mul_internal<RistrettoPoint, Scalar>(&points, &scalars)
        };
        point_compress_internal(&result)
    }

    // sum over i < count of (i + 2) * ((i + 1)B) == count(count + 1)(count + 2)/3 * B.
    public fun multi_scalar_mul_many(count: u64): bool {
        let points = vector[];
        let scalars = vector[];
        let index = 0;
        while (index < count) {
            points.push_back(times_b(index + 1));
            scalars.push_back(Scalar { data: scalar(index + 2) });
            index = index + 1;
        };
        let result = RistrettoPoint {
            handle: multi_scalar_mul_internal<RistrettoPoint, Scalar>(&points, &scalars)
        };
        let expected = times_b(count * (count + 1) * (count + 2) / 3);
        point_equals(&result, &expected)
    }
}

// RUN: execute 0x1::ristretto255::mul_in_place
// CHECK: results: true, 0xe882b131016b52c1d3337080187cf768423efccbb517bb495ab812c4160ff44e, 0xe2f2ae0a6abc4e71a884a961c500515f58e30b6aa582dd8db6a65945e08d2d76

// RUN: execute 0x1::ristretto255::mul_out_of_place
// CHECK: results: true, 0xe2f2ae0a6abc4e71a884a961c500515f58e30b6aa582dd8db6a65945e08d2d76, 0xe882b131016b52c1d3337080187cf768423efccbb517bb495ab812c4160ff44e

// RUN: execute 0x1::ristretto255::basepoint_double_mul
// CHECK: results: 0xbce83f8ba5dd2fa572864c24ba1810f9522bc6004afe95877ac73241cafdab42

// RUN: execute 0x1::ristretto255::double_scalar_mul
// CHECK: results: 0x02622ace8f7303a31cafc63f8fc48fdc16e1c8c8d234b2f0d6685282a9076031

// RUN: execute 0x1::ristretto255::multi_scalar_mul_two
// CHECK: results: 0xaa52e000df2e16f55fb1032fc33bc42742dad6bd5a8fc0be0167436c5948501f

// RUN: execute 0x1::ristretto255::multi_scalar_mul_one
// CHECK: results: 0x46376b80f409b29dc2b5f6f0c52591990896e5716f41477cd30085ab7f10301e

// RUN: execute 0x1::ristretto255::multi_scalar_mul_many --args 200
// CHECK: results: true
