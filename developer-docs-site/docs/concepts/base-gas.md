---
title: "How Base Gas Works"
id: "base-gas"
---

# How Base Gas Works

Aptos transactions by default charge a base gas fee, regardless of market conditions.
For each transaction, this "base gas" amount is based on three conditions:

1. Instructions.
2. Storage.
3. Payload.

The more function calls, branching conditional statements, etc. that a transaction requires, the more instruction gas it will cost.
Likewise, the more reads from and writes into global storage that a transaction requires, the more storage gas it will cost.
Finally, the more bytes in a transaction payload, the more it will cost.

## Instruction gas

Instruction gas parameters are defined at [`instr.rs`] and include the following instruction types:

### No-operation

| Parameter | Meaning        |
|-----------|----------------|
| `nop`     | A no-operation |

### Control flow

| Parameter  | Meaning                          |
|------------|----------------------------------|
| `ret`      | Return                           |
| `abort`    | Abort                            |
| `br_true`  | Execute conditional true branch  |
| `br_false` | Execute conditional false branch |
| `branch`   | Branch                           |

### Stack

| Parameter           | Meaning                          |
|---------------------|----------------------------------|
| `pop`               | Pop from stack                   |
| `ld_u8`             | Load a `u8`                      |
| `ld_u64`            | Load a `u64`                     |
| `ld_u128`           | Load a `u128`                    |
| `ld_true`           | Load a `true`                    |
| `ld_false`          | Load a `false`                   |
| `ld_const_base`     | Base cost to load a constant     |
| `ld_const_per_byte` | Per-byte cost to load a constant |

### Local scope

| Parameter                   | Meaning                  |
|-----------------------------|--------------------------|
| `imm_borrow_loc`            | Immutably borrow         |
| `mut_borrow_loc`            | Mutably borrow           |
| `imm_borrow_field`          | Immutably borrow a field |
| `mut_borrow_field`          | Mutably borrow a field   |
| `imm_borrow_field_generic`  |                          |
| `mut_borrow_field_generic`  |                          |
| `copy_loc_base`             | Base cost to copy        |
| `copy_loc_per_abs_val_unit` |                          |
| `move_loc_base`             | Move                     |
| `st_loc_base`               |                          |

### Calling

| Parameter                 | Meaning                       |
|---------------------------|-------------------------------|
| `call_base`               | Base cost for a function call |
| `call_per_arg`            | Cost per function argument    |
| `call_generic_base`       |                               |
| `call_generic_per_ty_arg` | Cost per type argument        |
| `call_generic_per_arg`    |                               |

### Structs

| Parameter                  | Meaning                              |
|----------------------------|--------------------------------------|
| `pack_base`                | Base cost to pack a `struct`         |
| `pack_per_field`           | Cost to pack a `struct`, per field   |
| `pack_generic_base`        |                                      |
| `pack_generic_per_field`   |                                      |
| `unpack_base`              | Base cost to unpack a `struct`       |
| `unpack_per_field`         | Cost to unpack a `struct`, per field |
| `unpack_generic_base`      |                                      |
| `unpack_generic_per_field` |                                      |

### References

| Parameter                   | Meaning                            |
|-----------------------------|------------------------------------|
| `read_ref_base`             | Base cost to read from a reference |
| `read_ref_per_abs_val_unit` |                                    |
| `write_ref_base`            | Base cost to write to a reference  |
| `freeze_ref`                | Freeze a reference                 |

### Casting

| Parameter   | Meaning          |
|-------------|------------------|
| `cast_u8`   | Cast to a `u8`   |
| `cast_u64`  | Cast to a `u64`  |
| `cast_u128` | Cast to a `u128` |

### Arithmetic

| Parameter | Meaning  |
|-----------|----------|
| `add`     | Add      |
| `sub`     | Subtract |
| `mul`     | Multiply |
| `mod_`    | Modulo   |
| `div`     | Divide   |


###  Bitwise

| Parameter | Meaning           |
|-----------|-------------------|
| `bit_or`  | `OR`: `\|`        |
| `bit_and` | `AND`: `&`        |
| `xor`     | `XOR`: `^`        |
| `shl`     | Shift left: `<<`  |
| `shr`     | Shift right: `>>` |

###  Boolean

| Parameter | Meaning      |
|-----------|--------------|
| `or`      | `OR`: `\|\|` |
| `and`     | `AND`: `&&`  |
| `not`     | `NOT`: `!`   |


###  Comparison

| Parameter              | Meaning                        |
|------------------------|--------------------------------|
| `lt`                   | Less than: `<`                 |
| `gt`                   | Greater than: `>`              |
| `le`                   | Less than or equal to: `<=`    |
| `ge`                   | Greater than or equal to: `>=` |
| `eq_base`              | Base equality cost: `==`       |
| `eq_per_abs_val_unit`  |                                |
| `neq_base`             | Base not equal cost: `!=`      |
| `neq_per_abs_val_unit` |                                |

### Global storage

| Parameter                        | Meaning                                               |
|----------------------------------|-------------------------------------------------------|
| `imm_borrow_global_base`         | Base cost to immutably borrow: `borrow_global<T>()`   |
| `imm_borrow_global_generic_base` |                                                       |
| `mut_borrow_global_base`         | Base cost to mutably borrow: `borrow_global_mut<T>()` |
| `mut_borrow_global_generic_base` |                                                       |
| `exists_base`                    | Base cost to check existence: `exists<T>()`           |
| `exists_generic_base`            |                                                       |
| `move_from_base`                 | Base cost to move from: `move_from<T>()`              |
| `move_from_generic_base`         |                                                       |
| `move_to_base`                   | Base cost to move to: `move_to<T>()`                  |
| `move_to_generic_base`           |                                                       |

### Vectors

| Parameter                      | Meaning                                  |
|--------------------------------|------------------------------------------|
| `vec_len_base`                 | Length of a vector                       |
| `vec_imm_borrow_base`          | Immutably borrow an element              |
| `vec_mut_borrow_base`          | Mutably borrow an element                |
| `vec_push_back_base`           | Push back                                |
| `vec_pop_back_base`            | Pop from the back                        |
| `vec_swap_base`                | Swap elements                            |
| `vec_pack_base`                | Base cost to pack a vector               |
| `vec_pack_per_elem`            | Cost to pack a vector per element        |
| `vec_unpack_base`              | Base cost to unpack a vector             |
| `vec_unpack_per_expected_elem` | Base cost to unpack a vector per element |

## Storage gas

Storage gas is defined in [`storage_gas.move`], which is accompanied by a comprehensive and internally-linked docgen file at [`storage_gas.md`].


In short:

1. In [`initialize()`], [`base_8192_exponential_curve()`] is used to generate an exponential curve whereby per-item and per-byte costs increase rapidly as utilization approaches an upper bound.
2. Parameters are reconfigured each epoch via [`on_reconfig()`], based on item-wise and byte-wise utilization ratios.
3. Reconfigured parameters are stored in [`StorageGas`], which contains the following fields:

| Field             | Meaning                                     |
|-------------------|---------------------------------------------|
| `per_item_read`   | Cost to read an item from global storage    |
| `per_item_create` | Cost to create an item in global storage    |
| `per_item_write`  | Cost to overwrite an item in global storage |
| `per_byte_read`   | Cost to read a byte from global storage     |
| `per_byte_create` | Cost to create a byte in global storage     |
| `per_byte_write`  | Cost to overwrite a byte in global storage  |


Here, an "item" is either a resource having the `key` attribute, or an entry in a table, and notably, per-byte costs are assessed on the *entire* size of an item.
As stated in [`storage_gas.md`], for example, if an operation mutates a `u8` field in a resource that has 5 other `u128` fields, the per-byte gas write cost will account for $(5 * 128) / 8 + 1 = 81$ bytes.

### Vectors

Byte-wise fees are similarly assessed on vectors, which consume $\sum_{i = 0}^{n - 1} e_i + b(n)$ bytes, where

* $n$ is the number of elements in the vector,
* $e_i$ is the size of element $i$,
* and $b(n)$ is a "base size" which is a function of $n$.

See [#4540] for more information on vector base size, which is typically just 1 byte in practice, such that a vector of 100 `u8` elements accounts for $100 + 1 = 101$ bytes.
Hence per the item-wise read methodology described above, reading the last element of such a vector is treated as a 101-byte read.

## Payload gas

Payload gas is defined in [`transaction.rs`], which incorporates storage gas with several payload-associated parameters:

| Parameter                       | Meaning                                                                                |
|---------------------------------|----------------------------------------------------------------------------------------|
| `min_transaction_gas_units`     | Minimum amount of gas for a transaction, charged at start of execution                 |
| `large_transaction_cutoff`      | Size, in bytes, above which transactions will be charged an additional amount per byte |
| `intrinsic_gas_per_byte`        | Units of gas charged per byte for payloads above `large_transaction_cutoff`            |
| `maximum_number_of_gas_units`   | Upper limit on gas that a transaction can require                                      |
| `min_price_per_gas_unit`        | Minimum gas price allowed on a transaction                                             |
| `max_price_per_gas_unit`        | Maximum gas price allowed on a transaction                                             |
| `max_transaction_size_in_bytes` | Maximum transaction payload size in bytes                                              |
| `gas_unit_scaling_factor`       | Amount of gas units in one octal                                                       |

<!--- Alphabetized reference links -->

[#4540]:                           https://github.com/aptos-labs/aptos-core/pull/4540/files
[`base_8192_exponential_curve()`]: https://github.com/aptos-labs/aptos-core/blob/e30f24f6c5a9b5bb0e1815284cd7278b28f83b1d/aptos-move/framework/aptos-framework/build/AptosFramework/docs/storage_gas.md#0x1_storage_gas_base_8192_exponential_curve
[`initialize()`]:                  https://github.com/aptos-labs/aptos-core/blob/e30f24f6c5a9b5bb0e1815284cd7278b28f83b1d/aptos-move/framework/aptos-framework/build/AptosFramework/docs/storage_gas.md#0x1_storage_gas_initialize
[`instr.rs`]:                      https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/aptos-gas/src/instr.rs
[`on_reconfig()`]:                 https://github.com/aptos-labs/aptos-core/blob/e30f24f6c5a9b5bb0e1815284cd7278b28f83b1d/aptos-move/framework/aptos-framework/build/AptosFramework/docs/storage_gas.md#0x1_storage_gas_on_reconfig
[`storage_gas.md`]:                https://github.com/aptos-labs/aptos-core/blob/e30f24f6c5a9b5bb0e1815284cd7278b28f83b1d/aptos-move/framework/aptos-framework/build/AptosFramework/docs/storage_gas.md
[`storage_gas.move`]:              https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-framework/sources/storage_gas.move
[`StorageGas`]:                    https://github.com/aptos-labs/aptos-core/blob/e30f24f6c5a9b5bb0e1815284cd7278b28f83b1d/aptos-move/framework/aptos-framework/build/AptosFramework/docs/storage_gas.md#0x1_storage_gas_StorageGas
[`transaction.rs`]:                https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/aptos-gas/src/transaction.rs