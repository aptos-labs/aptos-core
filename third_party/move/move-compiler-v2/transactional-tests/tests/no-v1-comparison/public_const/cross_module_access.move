//# publish
module 0x42::consts {
    // Byte-string constant
    public const BYTES: vector<u8> = b"hello world";

    // Hex-byte constant
    public const HEX_BYTES: vector<u8> = x"cafecafe";

    // Address constant
    public const ADDR: address = @0x42;

    // Expression constant: (1 << 1) * (1 << 2) * (1 << 3) = 2 * 4 * 8 = 64
    public const SHIFTY: u8 = {
        (1 << 1) * (1 << 2) * (1 << 3)
    };

    // Package-scoped constant (not accessible cross-module)
    package const PKG_VALUE: u64 = 100;

    // Private constant (not accessible cross-module)
    const PRIV_VALUE: u64 = 999;
}

//# publish
module 0x42::consumer {
    use 0x42::consts;

    public fun check_bytes(): bool {
        consts::BYTES == b"hello world"
    }

    public fun check_hex_bytes(): bool {
        consts::HEX_BYTES == x"cafecafe"
    }

    public fun check_addr(): bool {
        consts::ADDR == @0x42
    }

    public fun check_shifty(): bool {
        consts::SHIFTY == 64u8
    }
}

//# run
script {
    use 0x42::consumer;
    fun main() {
        assert!(consumer::check_bytes(), 1);
        assert!(consumer::check_hex_bytes(), 2);
        assert!(consumer::check_addr(), 3);
        assert!(consumer::check_shifty(), 4);
    }
}
