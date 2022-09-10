module aptos_std::type_info {
    use std::bcs;
    use std::string;
    use std::vector;

    struct TypeInfo has copy, drop, store {
        account_address: address,
        module_name: vector<u8>,
        struct_name: vector<u8>,
    }

    public fun account_address(type_info: &TypeInfo): address {
        type_info.account_address
    }

    public fun module_name(type_info: &TypeInfo): vector<u8> {
        type_info.module_name
    }

    public fun struct_name(type_info: &TypeInfo): vector<u8> {
        type_info.struct_name
    }

    public native fun type_of<T>(): TypeInfo;
    public native fun type_name<T>(): string::String;

    /// Return the size, in bytes, of a `T`, provided an immutable
    /// reference to a quasi-null instance of fixed-size `T`,
    /// `fixed_size_type_null_ref`.
    ///
    /// Analogous to `sizeof()` in C.
    ///
    /// Ideally this function would be implemented as a native function
    /// of the form `public native fun size_of<T>(): u64;`, such that
    /// callers do not need to concoct quasi-null instances, e.g.
    /// `false` for `T` as a `bool`, `0` for `T` as a `u8`, or `@0x0`
    /// for `T` as an `address`.
    ///
    /// Does not actually enfore that `T` is a fixed-size type, which
    /// would require determining whether or not vectors are nested
    /// within, for example.
    ///
    /// See `test_size_of()` for an analysis of common types and nesting
    /// patterns, as well as `test_size_of_vectors()` for an analysis of
    /// vector base size dynamism.
    public fun size_of<T>(
        fixed_size_type_null_ref: &T
    ): u64 {
        // Return vector length of vectorized bytes representation
        vector::length(&bcs::to_bytes(fixed_size_type_null_ref))
    }

    #[test]
    fun test() {
        let type_info = type_of<TypeInfo>();
        assert!(account_address(&type_info) == @aptos_std, 0);
        assert!(module_name(&type_info) == b"type_info", 1);
        assert!(struct_name(&type_info) == b"TypeInfo", 2);
    }

    #[test]
    fun test_type_name() {
        use aptos_std::table::Table;

        assert!(type_name<bool>() == string::utf8(b"bool"), 0);
        assert!(type_name<u8>() == string::utf8(b"u8"), 1);
        assert!(type_name<u64>() == string::utf8(b"u64"), 2);
        assert!(type_name<u128>() == string::utf8(b"u128"), 3);
        assert!(type_name<address>() == string::utf8(b"address"), 4);
        assert!(type_name<signer>() == string::utf8(b"signer"), 5);

        // vector
        assert!(type_name<vector<u8>>() == string::utf8(b"vector<u8>"), 6);
        assert!(type_name<vector<vector<u8>>>() == string::utf8(b"vector<vector<u8>>"), 7);
        assert!(type_name<vector<vector<TypeInfo>>>() == string::utf8(b"vector<vector<0x1::type_info::TypeInfo>>"), 8);


        // struct
        assert!(type_name<TypeInfo>() == string::utf8(b"0x1::type_info::TypeInfo"), 9);
        assert!(type_name<
            Table<
                TypeInfo,
                Table<u8, vector<TypeInfo>>
            >
        >() == string::utf8(b"0x1::table::Table<0x1::type_info::TypeInfo, 0x1::table::Table<u8, vector<0x1::type_info::TypeInfo>>>"), 10);
    }

    #[test_only]
    struct CustomType has drop {}

    #[test_only]
    struct SimpleStruct has copy, drop {
        field: u8
    }

    #[test_only]
    struct ComplexStruct<T> has copy, drop {
        field_1: bool,
        field_2: u8,
        field_3: u64,
        field_4: u128,
        field_5: SimpleStruct,
        field_6: T
    }

    #[test_only]
    struct TwoBools has drop {
        bool_1: bool,
        bool_2: bool
    }

    #[test_only]
    use std::option;

    #[test(account = @0x0)]
    /// Ensure valid returns across native types and nesting schemas
    fun test_size_of(
        account: &signer
    ) {
        assert!(size_of(&false) == 1, 0); // Bool takes 1 byte
        assert!(size_of<u8>(&0) == 1, 0); // u8 takes 1 byte
        assert!(size_of<u64>(&0) == 8, 0); // u64 takes 8 bytes
        assert!(size_of<u128>(&0) == 16, 0); // u128 takes 16 bytes
        // Address is stored as a u256
        assert!(size_of(&@0x0) == 32, 0);
        assert!(size_of(account) == 32, 0); // Signer is an address
        // Assert custom type without fields has size 1
        assert!(size_of(&CustomType{}) == 1, 0);
        // Declare a simple struct with a 1-byte field
        let simple_struct = SimpleStruct{field: 0};
        // Assert size is indicated as 1 byte
        assert!(size_of(&simple_struct) == 1, 0);
        let complex_struct = ComplexStruct<u128>{
            field_1: false,
            field_2: 0,
            field_3: 0,
            field_4: 0,
            field_5: simple_struct,
            field_6: 0
        }; // Declare a complex struct with another nested inside
        // Assert size is bytewise sum of components
        assert!(size_of(&complex_struct) == (1 + 1 + 8 + 16 + 1 + 16), 0);
        // Declare a struct with two boolean values
        let two_bools = TwoBools{bool_1: false, bool_2: false};
        // Assert size is two bytes
        assert!(size_of(&two_bools) == 2, 0);
        // Declare an empty vector of element type u64
        let empty_vector_u64 = vector::empty<u64>();
        // Declare an empty vector of element type u128
        let empty_vector_u128 = vector::empty<u128>();
        // Assert size is 1 byte regardless of underlying element type
        assert!(size_of(&empty_vector_u64) == 1, 0);
        // Assert size is 1 byte regardless of underlying element type
        assert!(size_of(&empty_vector_u128) == 1, 0);
        // Declare a bool in a vector
        let bool_vector = vector::singleton(false);
        // Push back another bool
        vector::push_back(&mut bool_vector, false);
        // Assert size is 3 bytes (1 per element, 1 for the base vector)
        assert!(size_of(&bool_vector) == 3, 0);
        // Get a some option, which is implemented as a vector
        let u64_option = option::some(0);
        // Assert size is 9 bytes (8 per element, 1 for the base vector)
        assert!(size_of(&u64_option) == 9, 0);
        option::extract(&mut u64_option); // Remove the value inside
        // Assert size reduces to 1 byte
        assert!(size_of(&u64_option) == 1, 0);
    }

    #[test]
    /// Verify returns for base vector size at different lengths, with
    /// different underlying fixed-size elements.
    ///
    /// For a vector of length n containing fixed-size elements, the
    /// size of the vector is b + n * s bytes, where s is the size of an
    /// element in bytes, and b is a "base size" in bytes that varies
    /// with n. Per below, b is established as follows:
    /// * b = 1, n < 128
    /// * b = 2, 128 <= n < 16384
    /// * b = 3, 16384 <= n < ?
    /// ...
    fun test_size_of_vectors() {
        let vector_u64 = vector::empty<u64>(); // Declare empty vector
        let null_element = 0; // Declare a null element
        let element_size = size_of(&null_element); // Get element size
        // Vector size is 1 byte when length is 0
        assert!(size_of(&vector_u64) == 1, 0);
        let i = 0; // Declare loop counter
        while (i < 127) { // For 127 iterations
            // Add an element
            vector::push_back(&mut vector_u64, null_element);
            i = i + 1; // Increment counter
        };
        // Vector base size is still 1 byte (127 elements)
        assert!(size_of(&vector_u64) - element_size * i == 1, 0);
        // Add a 128th element
        vector::push_back(&mut vector_u64, null_element);
        i = i + 1; // Increment counter
        // Vector base size is now 2 bytes
        assert!(size_of(&vector_u64) - element_size * i == 2, 0);
        // Repeat until (2 ^ 16 / 4 - 1) elements in vector
        while (i < 16383) {
            // Add an element
            vector::push_back(&mut vector_u64, null_element);
            i = i + 1; // Increment counter
        };
        // Vector base size is still 2 bytes
        assert!(size_of(&vector_u64) - element_size * i == 2, 0);
        // Add 16384th element
        vector::push_back(&mut vector_u64, null_element);
        i = i + 1; // Increment counter
        // Vector base size is now 3 bytes
        assert!(size_of(&vector_u64) - element_size * i == 3, 0);
        // Repeat for custom struct
        let vector_complex = vector::empty<ComplexStruct<address>>();
        // Declare a null element
        let null_element = ComplexStruct{
            field_1: false,
            field_2: 0,
            field_3: 0,
            field_4: 0,
            field_5: SimpleStruct{field: 0},
            field_6: @0x0
        };
        element_size = size_of(&null_element); // Get element size
        // Vector size is 1 byte when length is 0
        assert!(size_of(&vector_complex) == 1, 0);
        i = 0; // Re-initialize loop counter
        while (i < 127) { // For 127 iterations
            // Add an element
            vector::push_back(&mut vector_complex, copy null_element);
            i = i + 1; // Increment counter
        };
        // Vector base size is still 1 byte (127 elements)
        assert!(size_of(&vector_complex) - element_size * i == 1, 0);
        // Add a 128th element
        vector::push_back(&mut vector_complex, null_element);
        i = i + 1; // Increment counter
        // Vector base size is now 2 bytes
        assert!(size_of(&vector_complex) - element_size * i == 2, 0);
        // Repeat until (2 ^ 16 / 4 - 1) elements in vector
        while (i < 16383) {
            // Add an element
            vector::push_back(&mut vector_complex, copy null_element);
            i = i + 1; // Increment counter
        };
        // Vector base size is still 2 bytes
        assert!(size_of(&vector_complex) - element_size * i == 2, 0);
        // Add 16384th element
        vector::push_back(&mut vector_complex, null_element);
        i = i + 1; // Increment counter
        // Vector base size is now 3 bytes
        assert!(size_of(&vector_complex) - element_size * i == 3, 0);
    }

}
