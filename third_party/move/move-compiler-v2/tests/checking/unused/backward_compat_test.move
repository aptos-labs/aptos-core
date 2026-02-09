// Test to verify backward compatibility of struct_usage changes
// This ensures that structs used in global storage operations
// are properly tracked for package visibility and dependency analysis
module 0x42::provider {
    public struct Resource has key {
        value: u64
    }

    public fun create(): Resource {
        Resource { value: 42 }
    }
}

module 0x42::consumer {
    use 0x42::provider;

    // This function uses provider::Resource via exists<T>
    // The struct_usage analysis should detect this usage
    public fun check_exists(addr: address): bool {
        exists<provider::Resource>(addr)
    }

    // This function uses provider::Resource via borrow_global
    public fun read_value(addr: address): u64 acquires provider::Resource {
        borrow_global<provider::Resource>(addr).value
    }

    // This function uses provider::Resource via move_from
    public fun take_resource(addr: address): provider::Resource acquires provider::Resource {
        move_from<provider::Resource>(addr)
    }
}
