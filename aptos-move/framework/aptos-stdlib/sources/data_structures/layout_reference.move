module aptos_framework::layout_reference {
    use aptos_framework::object;
    use aptos_std::table_with_length::{Self, TableWithLength};
    use std::mem;

    struct ObjectValue<T> has key {
        value: T,
    }

    enum LayoutReference<T> has store {
        Inline{ value: T },
        // ExternalObject{ addr: address },
        ExternalTable { table: TableWithLength<bool, T>},
    }

    public fun new_inline<T: store>(value: T): LayoutReference<T> {
        LayoutReference::Inline { value }
    }

    // public fun new_external_object<T: store>(value: T): LayoutReference<T> {
    //     LayoutReference::ExternalObject(store_at_new_address(value))
    // }

    public fun new_external_table<T: store>(value: T): LayoutReference<T> {
        let table = table_with_length::new();
        table.add(true, value);
        LayoutReference::ExternalTable { table }
    }

    public fun borrow<T: store>(self: &LayoutReference<T>): &T {
        match (self) {
            LayoutReference::Inline { value } => value,
            // LayoutReference::ExternalObject(addr) => &ObjectValue<T>[*addr].value,
            LayoutReference::ExternalTable { table } => table.borrow(true),
        }
    }

    public fun borrow_mut<T: store>(self: &mut LayoutReference<T>): &mut T {
        match (self) {
            LayoutReference::Inline { value } => value,
            // LayoutReference::ExternalObject(addr) => &mut ObjectValue<T>[*addr].value,
            LayoutReference::ExternalTable { table } => table.borrow_mut(true),
        }
    }

    public fun destroy<T: store>(self: LayoutReference<T>): T {
        match (self) {
            LayoutReference::Inline { value } => value,
            // LayoutReference::ExternalObject(addr) => {
            //     let ObjectValue { value } = move_from<ObjectValue<T>>(addr);
            //     value
            // },
            LayoutReference::ExternalTable { table } => {
                let value = table.remove(true);
                table.destroy_empty();
                value
            },
        }
    }

    public fun move_to_inline<T: store>(self: &mut LayoutReference<T>) {
        match (self) {
            LayoutReference::Inline { value: _ } => {},
            // LayoutReference::ExternalObject(addr) => {
            //     let ObjectValue { value } = move_from<ObjectValue<T>>(*addr);
            //     *self = LayoutReference::Inline(value);
            // },
            LayoutReference::ExternalTable { table } => {
                let value = table.remove(true);
                let LayoutReference::ExternalTable { table } = mem::replace(self, LayoutReference::Inline { value });
                table.destroy_empty();
            },
        }
    }

    // public fun move_to_external_object<T>(self: &mut LayoutReference<T>) {
    //     match (self) {
    //         LayoutReference::Inline(value) => {
    //             *self = LayoutReference::ExternalObject(store_at_new_address(value));
    //         },
    //         LayoutReference::ExternalObject(_) => {},
    //         LayoutReference::ExternalTable(table) => table.remove(true),
    //     }
    // }

    public fun move_to_external_table<T: store>(self: &mut LayoutReference<T>) {
        match (self) {
            LayoutReference::Inline { value: _ } => {
                let LayoutReference::Inline { value } = mem::replace(self, LayoutReference::ExternalTable { table: table_with_length::new() });
                self.table.add(true, value);
            },
            // LayoutReference::ExternalObject(_) => {},
            LayoutReference::ExternalTable { table: _ } => {},
        }
    }

    fun store_at_new_address<T: store>(value: T): address {
        let constructor_ref = object::create_object(@aptos_framework);
        let extend_ref = object::generate_extend_ref(&constructor_ref);
        object::delete(object::generate_delete_ref(&constructor_ref));
        move_to(&object::generate_signer_for_extending(&extend_ref), ObjectValue { value });
        object::address_from_extend_ref(&extend_ref)
    }
}
