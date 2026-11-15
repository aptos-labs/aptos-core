/// Move 2 reference-transparent fields and vector/global index notation.
module 0x42::move2_index {
    struct Resource has store, key {
        value: u64,
        values: vector<u64>,
    }

    public fun field(self: &Resource): u64 {
        self.value
    }

    public fun borrow_field(self: &Resource): &u64 {
        &self.value
    }

    public fun borrow_mut_field(self: &mut Resource): &mut u64 {
        &mut self.value
    }

    public fun vector_value(self: &Resource, index: u64): u64 {
        self.values[index]
    }

    public fun vector_borrow(self: &Resource, index: u64): &u64 {
        &self.values[index]
    }

    public fun vector_borrow_mut(self: &mut Resource, index: u64): &mut u64 {
        &mut self.values[index]
    }

    public fun vector_write(self: &mut Resource, index: u64, value: u64) {
        self.values[index] = value;
    }

    public fun storage_field(address: address): u64 acquires Resource {
        Resource[address].value
    }

    public fun storage_borrow(address: address): &Resource acquires Resource {
        &Resource[address]
    }

    public fun storage_borrow_mut(address: address): &mut Resource acquires Resource {
        &mut Resource[address]
    }

    public fun storage_borrow_field(address: address): &u64 acquires Resource {
        &Resource[address].value
    }

    public fun storage_borrow_mut_field(address: address): &mut u64 acquires Resource {
        &mut Resource[address].value
    }

    public fun storage_write(address: address, value: u64) acquires Resource {
        Resource[address].value = value;
    }

    public fun storage_write_resource(address: address, value: Resource) acquires Resource {
        Resource[address] = value;
    }
}
