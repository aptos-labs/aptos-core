module aptos_framework::ethereum {
    use std::vector;
    use aptos_std::aptos_hash::keccak256;

    /// Constants for ASCII character codes
    const ASCII_A: u8 = 0x41;
    const ASCII_Z: u8 = 0x5A;
    const ASCII_A_LOWERCASE: u8 = 0x61;
    const ASCII_F_LOWERCASE: u8 = 0x66;

    // Error codes

    const EINVALID_LENGTH: u64 = 1;

    /// Represents an Ethereum address within Aptos smart contracts.
    /// Provides structured handling, storage, and validation of Ethereum addresses.
    struct EthereumAddress has store, copy, drop {
        inner: vector<u8>,
    }

    /// Validates an Ethereum address against EIP-55 checksum rules and returns a new `EthereumAddress`.
    ///
    /// @param ethereum_address A 40-byte vector of unsigned 8-bit integers (hexadecimal format).
    /// @return A validated `EthereumAddress` struct.
    /// @abort If the address does not conform to EIP-55 standards.
    public fun ethereum_address(ethereum_address: vector<u8>): EthereumAddress {
        assert_eip55(&ethereum_address);
        EthereumAddress { inner: ethereum_address }
    }

    /// Returns a new `EthereumAddress` without EIP-55 validation.
    ///
    /// @param ethereum_address A 40-byte vector of unsigned 8-bit integers (hexadecimal format).
    /// @return A validated `EthereumAddress` struct.
    /// @abort If the address does not conform to EIP-55 standards.
    public fun ethereum_address_no_eip55(ethereum_address: vector<u8>): EthereumAddress {
        assert_40_char_hex(&ethereum_address);
        EthereumAddress { inner: ethereum_address }
    }

    /// Returns a new 20-byte `EthereumAddress` without EIP-55 validation.
    ///
    /// @param ethereum_address A 20-byte vector of unsigned 8-bit bytes.
    /// @return An `EthereumAddress` struct.
    /// @abort If the address does not conform to EIP-55 standards.
    public fun ethereum_address_20_bytes(ethereum_address: vector<u8>): EthereumAddress {
        assert!(vector::length(&ethereum_address) == 20, EINVALID_LENGTH);
        EthereumAddress { inner: ethereum_address }
    }

    /// Gets the inner vector of an `EthereumAddress`.
    ///
    /// @param ethereum_address A 40-byte vector of unsigned 8-bit integers (hexadecimal format).
    /// @return The vector<u8> inner value of the EthereumAddress
    public fun get_inner_ethereum_address(ethereum_address: EthereumAddress): vector<u8> {
        ethereum_address.inner
    }

    /// Converts uppercase ASCII characters in a vector to their lowercase equivalents.
    ///
    /// @param input A reference to a vector of ASCII characters.
    /// @return A new vector with lowercase equivalents of the input characters.
    /// @note Only affects ASCII letters; non-alphabetic characters are unchanged.
    public fun to_lowercase(input: &vector<u8>): vector<u8> {
        let lowercase_bytes = vector::empty();
        vector::enumerate_ref(input, |_i, element| {
            let lower_byte = if (*element >= ASCII_A && *element <= ASCII_Z) {
                *element + 32
            } else {
                *element
            };
            vector::push_back<u8>(&mut lowercase_bytes, lower_byte);
        });
        lowercase_bytes
    }

    #[test]
    fun test_to_lowercase() {
        let upper = b"TeST";
        let lower = b"test";
        assert!(to_lowercase(&upper) == lower, 0);
    }

    /// Converts an Ethereum address to EIP-55 checksummed format.
    ///
    /// @param ethereum_address A 40-character vector representing the Ethereum address in hexadecimal format.
    /// @return The EIP-55 checksummed version of the input address.
    /// @abort If the input address does not have exactly 40 characters.
    /// @note Assumes input address is valid and in lowercase hexadecimal format.
    public fun to_eip55_checksumed_address(ethereum_address: &vector<u8>): vector<u8> {
        assert!(vector::length(ethereum_address) == 40, 0);
        let lowercase = to_lowercase(ethereum_address);
        let hash = keccak256(lowercase);
        let output = vector::empty<u8>();

        for (index in 0..40) {
            let item = *vector::borrow(ethereum_address, index);
            if (item >= ASCII_A_LOWERCASE && item <= ASCII_F_LOWERCASE) {
                let hash_item = *vector::borrow(&hash, index / 2);
                if ((hash_item >> ((4 * (1 - (index % 2))) as u8)) & 0xF >= 8) {
                    vector::push_back(&mut output, item - 32);
                } else {
                    vector::push_back(&mut output, item);
                }
            } else {
                vector::push_back(&mut output, item);
            }
        };
        output
    }

    public fun get_inner(eth_address: &EthereumAddress): vector<u8> {
        eth_address.inner
    }

    /// Checks if an Ethereum address conforms to the EIP-55 checksum standard.
    ///
    /// @param ethereum_address A reference to a 40-character vector of an Ethereum address in hexadecimal format.
    /// @abort If the address does not match its EIP-55 checksummed version.
    /// @note Assumes the address is correctly formatted as a 40-character hexadecimal string.
    public fun assert_eip55(ethereum_address: &vector<u8>) {
        let eip55 = to_eip55_checksumed_address(ethereum_address);
        let len = vector::length(&eip55);
        for (index in 0..len) {
            assert!(vector::borrow(&eip55, index) == vector::borrow(ethereum_address, index), 0);
        };
    }

    /// Checks if an Ethereum address is a nonzero 40-character hexadecimal string.
    ///
    /// @param ethereum_address A reference to a vector of bytes representing the Ethereum address as characters.
    /// @abort If the address is not 40 characters long, contains invalid characters, or is all zeros.
    public fun assert_40_char_hex(ethereum_address: &vector<u8>) {
        let len = vector::length(ethereum_address);

        // Ensure the address is exactly 40 characters long
        assert!(len == 40, 1);

        // Ensure the address contains only valid hexadecimal characters
        let is_zero = true;
        for (index in 0..len) {
            let char = *vector::borrow(ethereum_address, index);

            // Check if the character is a valid hexadecimal character (0-9, a-f, A-F)
            assert!(
                (char >= 0x30 && char <= 0x39) || // '0' to '9'
                (char >= 0x41 && char <= 0x46) || // 'A' to 'F'
                (char >= 0x61 && char <= 0x66),  // 'a' to 'f'
                2
            );

            // Check if the address is nonzero
            if (char != 0x30) { // '0'
                is_zero = false;
            };
        };

        // Abort if the address is all zeros
        assert!(!is_zero, 3);
    }

    #[test_only]
    public fun eth_address_20_bytes(): vector<u8> {
        vector[0x32, 0xBe, 0x34, 0x3B, 0x94, 0xf8, 0x60, 0x12, 0x4d, 0xC4, 0xfE, 0xE2, 0x78, 0xFD, 0xCB, 0xD3, 0x8C, 0x10, 0x2D, 0x88]
}

    #[test_only]
    public fun valid_eip55(): vector<u8> {
        b"32Be343B94f860124dC4fEe278FDCBD38C102D88"
    }

    #[test_only]
    public fun invalid_eip55(): vector<u8> {
        b"32be343b94f860124dc4fee278fdcbd38c102d88"
    }

    #[test]
    fun test_valid_eip55_checksum() {
        assert_eip55(&valid_eip55());
    }

    #[test]
    #[expected_failure(abort_code = 0, location = Self)]
    fun test_invalid_eip55_checksum() {
        assert_eip55(&invalid_eip55());
    }

    #[test]
    #[expected_failure(abort_code = 0, location = Self)]
    fun test_simple_invalid_eip55_checksum() {
        assert_eip55(&b"0");
    }
}