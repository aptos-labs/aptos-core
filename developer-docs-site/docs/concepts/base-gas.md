---
title: "Computing Transaction Gas"
id: "base-gas"
---

# Computing Transaction Gas

Aptos transactions by default charge a base gas fee, regardless of market conditions.
For each transaction, this "base gas" amount is based on three conditions:

1. Instructions.
2. Storage.
3. Payload.

The more function calls, branching conditional statements, etc. that a transaction requires, the more instruction gas it will cost.
Likewise, the more reads from and writes into global storage that a transaction requires, the more storage gas it will cost.
Finally, the more bytes in a transaction payload, the more it will cost.

As explained in the [optimization principles](#optimization-principles) section, storage gas has by far the largest effect on base gas. For background on the Aptos gas model, see [The Making of the Aptos Gas Schedule](https://aptoslabs.medium.com/the-making-of-the-aptos-gas-schedule-508d5686a350).


## Instruction gas

Basic instruction gas parameters are defined at [`instr.rs`] and include the following instruction types:

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
| `ld_u16`            | Load a `u16`                     |
| `ld_u32`            | Load a `u32`                     |
| `ld_u64`            | Load a `u64`                     |
| `ld_u128`           | Load a `u128`                    |
| `ld_256`            | Load a `u256`                    |
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

| Parameter                 | Meaning                         |
|---------------------------|---------------------------------|
| `call_base`               | Base cost for a function call   |
| `call_per_arg`            | Cost per function argument      |
| `call_per_local`          | Cost per local argument         |
| `call_generic_base`       |                                 |
| `call_generic_per_ty_arg` | Cost per type argument          |
| `call_generic_per_arg`    |                                 |
| `call_generic_per_local`  | Cost generic per local argument |

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
| `cast_u16`  | Cast to a `u16`  |
| `cast_u32`  | Cast to a `u32`  |
| `cast_u64`  | Cast to a `u64`  |
| `cast_u128` | Cast to a `u128` |
| `cast_u256` | Cast to a `u256` |

### Arithmetic

| Parameter | Meaning  |
|-----------|----------|
| `add`     | Add      |
| `sub`     | Subtract |
| `mul`     | Multiply |
| `mod_`    | Modulo   |
| `div`     | Divide   |


###  Bitwise

| Parameter | Meaning                   |
|-----------|---------------------------|
| `bit_or`  | `OR`: <code>&#124;</code> |
| `bit_and` | `AND`: `&`                |
| `xor`     | `XOR`: `^`                |
| `shl`     | Shift left: `<<`          |
| `shr`     | Shift right: `>>`         |

###  Boolean

| Parameter | Meaning                         |
|-----------|---------------------------------|
| `or`      | `OR`: <code>&#124;&#124;</code> |
| `and`     | `AND`: `&&`                     |
| `not`     | `NOT`: `!`                      |


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

Additional storage gas parameters are defined in [`table.rs`], [`move_stdlib.rs`], and other assorted source files in [`aptos-gas-schedule/src/`].

## Storage gas

Storage gas is defined in [`storage_gas.move`], which is accompanied by a comprehensive and internally-linked DocGen file at [`storage_gas.md`].


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

Here, an *item* is either a resource having the `key` attribute, or an entry in a table, and notably, per-byte costs are assessed on the *entire* size of an item.
As stated in [`storage_gas.md`], for example, if an operation mutates a `u8` field in a resource that has five other `u128` fields, the per-byte gas write cost will account for $(5 * 128) / 8 + 1 = 81$ bytes.

### Vectors

Byte-wise fees are similarly assessed on vectors, which consume $\sum_{i = 0}^{n - 1} e_i + b(n)$ bytes, where:

* $n$ is the number of elements in the vector
* $e_i$ is the size of element $i$
* $b(n)$ is a "base size" which is a function of $n$

See the [BCS sequence specification] for more information on vector base size (technically a `ULEB128`), which typically occupies just one byte in practice, such that a vector of 100 `u8` elements accounts for $100 + 1 = 101$ bytes.
Hence per the item-wise read methodology described above, reading the last element of such a vector is treated as a 101-byte read.

## Payload gas

Payload gas is defined in [`transaction.rs`](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/aptos-gas-schedule/src/gas_schedule/transaction.rs), which incorporates storage gas with several payload- and pricing-associated parameters:

| Parameter                       | Meaning                                                                                |
|---------------------------------|----------------------------------------------------------------------------------------|
| `min_transaction_gas_units`     | Minimum internal gas units for a transaction, charged at the start of execution        |
| `large_transaction_cutoff`      | Size, in bytes, above which transactions will be charged an additional amount per byte |
| `intrinsic_gas_per_byte`        | Internal gas units charged per byte for payloads above `large_transaction_cutoff`      |
| `maximum_number_of_gas_units`   | Upper limit on external gas units for a transaction                                    |
| `min_price_per_gas_unit`        | Minimum gas price allowed for a transaction                                            |
| `max_price_per_gas_unit`        | Maximum gas price allowed for a transaction                                            |
| `max_transaction_size_in_bytes` | Maximum transaction payload size in bytes                                              |
| `gas_unit_scaling_factor`       | Conversion factor between internal gas units and external gas units                    |

Here, "internal gas units" are defined as constants in source files like [`instr.rs`] and [`storage_gas.move`], which are more granular than "external gas units" by a factor of `gas_unit_scaling_factor`:
to convert from internal gas units to external gas units, divide by `gas_unit_scaling_factor`.
Then, to convert from external gas units to octas, multiply by the "gas price", which denotes the number of octas per unit of external gas.

## Optimization principles

### Unit and pricing constants

As of the time of this writing, `min_price_per_gas_unit` in [`transaction.rs`](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/aptos-gas-schedule/src/gas_schedule/transaction.rs) is defined as [`aptos_global_constants`]`::GAS_UNIT_PRICE` (which is itself defined as 100), with other noteworthy [`transaction.rs`](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/aptos-gas-schedule/src/gas_schedule/transaction.rs) constants as follows:

| Constant                  | Value  |
|---------------------------|--------|
| `min_price_per_gas_unit`  | 100    |
| `max_price_per_gas_unit`  | 10,000,000,000 |
| `gas_unit_scaling_factor` | 1,000,000 |

See [Payload gas](#payload-gas) for the meaning of these constants.

### Storage gas

As of the time of this writing, [`initialize()`] sets the following minimum storage gas amounts:

| Data style | Operation | Symbol | Minimum internal gas |
|------------|-----------|--------|----------------------|
| Per item   | Read      | $r_i$  | 300,000              |
| Per item   | Create    | $c_i$  | 300,000              |
| Per item   | Write     | $w_i$  | 300,000              |
| Per byte   | Read      | $r_b$  | 300                  |
| Per byte   | Create    | $c_b$  | 5,000                |
| Per byte   | Write     | $w_b$  | 5,000                |

Maximum amounts are 100 times the minimum amounts, which means that for a utilization ratio of 40% or less, total gas costs will be on the order of 1 to 1.5 times the minimum amounts (see [`base_8192_exponential_curve()`] for supporting calculations).
Hence, in terms of octas, initial mainnet gas costs can be estimated as follows (divide internal gas by scaling factor, then multiply by minimum gas price):

| Operation       | Operation | Minimum octas |
|-----------------|-----------|---------------|
| Per-item read   | $r_i$     | 3000          |
| Per-item create | $c_i$     | 50,000          |
| Per-item write  | $w_i$     | 3000          |
| Per-byte read   | $r_b$     | 3             |
| Per-byte create | $c_b$     | 50            |
| Per-byte write  | $w_b$     | 50            |

Here, the most expensive per-item operation by far is creating a new item (via either `move_to<T>()` or adding to a table), which costs nearly 17 times as much as reading or overwriting an old item: $c_i = 16.\overline{6} r_i = 16.\overline{6} w_i$. Additionally:

* Writes cost the same as reads on a per-item basis: $w_i = r_i$
* On a per-byte basis, however, writes cost the same as creates: $w_b = c_b$
* Per-byte writes and creates cost nearly 17 times as much as per-byte reads: $w_b = c_b = 16.\overline{6} r_b$
* Per-item reads cost 1000 times as much as per-byte reads: $r_i = 1000 r_b$
* Per-item creates cost 1000 times as much as per-byte creates: $c_i = 1000 c_b$
* Per-item writes cost 60 times as much as per-byte writes: $w_i = 60 w_b$

Hence per-item operations cost 1000 times more than per-byte operations for both reads and creates, but only 60 times more for writes.

Thus, in the absence of a legitimate economic incentive to deallocate from global storage (via either `move_from<T>()` or by removing from a table), the most effective storage gas optimization strategy is as follows:

1. Minimize per-item creations
2. Track unused items and overwrite them, rather than creating new items, when possible
3. Contain per-item writes to as few items as possible
4. Read, rather than write, whenever possible
5. Minimize the number of bytes in all operations, especially writes

### Instruction gas

As of the time of this writing, all instruction gas operations are multiplied by the `EXECUTION_GAS_MULTIPLIER` defined in [`gas_meter.rs`], which is set to 20.
Hence the following representative operations assume gas costs as follows (divide internal gas by scaling factor, then multiply by minimum gas price):

| Operation                    | Minimum octas |
|------------------------------|---------------|
| Table add/borrow/remove box  | 240           |
| Function call                | 200           |
| Load constant                | 130           |
| Globally borrow              | 100           |
| Read/write reference         | 40            |
| Load `u128` on stack         | 16            |
| Table box operation per byte | 2             |

(Note that per-byte table box operation instruction gas does not account for storage gas, which is assessed separately).

For comparison, reading a 100-byte item costs $r_i + 100 * r_b = 3000 + 100 * 3 = 3300$ octas at minimum, some 16.5 times as much as a function call, and in general, instruction gas costs are largely dominated by storage gas costs.

Notably, however, there is still technically an incentive to reduce the number of function calls in a program, but engineering efforts are more effectively dedicated to writing modular, decomposed code that is geared toward reducing storage gas costs, rather than attempting to write repetitive code blocks with fewer nested functions (in nearly all cases).

In extreme cases it is possible for instruction gas to far outweigh storage gas, for example if a loopwise mathematical function takes 10,000 iterations to converge; but again this is an extreme case and for most applications storage gas has a larger impact on base gas than does instruction gas.

### Payload gas

As of the time of this writing, [`transaction/mod.rs`](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/aptos-gas-schedule/src/gas_schedule/transaction.rs) defines the minimum amount of internal gas per transaction as 1,500,000 internal units (15,000 octas at minimum), an amount that increases by 2,000 internal gas units (20 octas minimum) per byte for payloads larger than 600 bytes, with the maximum number of bytes permitted in a transaction set at 65536.
Hence in practice, payload gas is unlikely to be a concern.

<!--- Alphabetized reference links -->

[#4540]:                           https://github.com/aptos-labs/aptos-core/pull/4540/files
[`aptos-gas-schedule/src/`]:       https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/aptos-gas-schedule/src
[`aptos_global_constants`]:        https://github.com/aptos-labs/aptos-core/blob/main/config/global-constants/src/lib.rs
[`base_8192_exponential_curve()`]: https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-framework/doc/storage_gas.md#0x1_storage_gas_base_8192_exponential_curve
[BCS sequence specification]:      https://github.com/diem/bcs#fixed-and-variable-length-sequences
[`gas_meter.rs`]:                  https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/aptos-gas/src/gas_meter.rs
[`initialize()`]:                  https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-framework/doc/storage_gas.md#0x1_storage_gas_initialize
[`instr.rs`]:                      https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/aptos-gas-schedule/src/gas_schedule/instr.rs
[`move_stdlib.rs`]:                https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/aptos-gas-schedule/src/gas_schedule/move_stdlib.rs
[`on_reconfig()`]:                 https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-framework/doc/storage_gas.md#@Specification_16_on_reconfig
[`storage_gas.md`]:                https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-framework/doc/storage_gas.md
[`storage_gas.move`]:              https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-framework/sources/storage_gas.move
[`StorageGas`]:                    https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-framework/doc/storage_gas.md#resource-storagegas
[`table.rs`]:                      https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/aptos-gas-schedule/src/gas_schedule/table.rs
[`transaction.rs`]:                https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/aptos-gas-schedule/src/gas_schedule/transaction.rs
