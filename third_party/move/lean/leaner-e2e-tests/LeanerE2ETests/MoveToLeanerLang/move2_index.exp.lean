-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-! Move 2 reference-transparent fields and vector/global index notation. -/
leaner module 0x42::move2_index where
  struct Resource has Store, Key where
    value : u64
    values : Vector<u64>

  public fun field(self : &Resource) -> u64 := self.value

  public fun borrow_field(self : &Resource) -> &u64 := &self.value

  public fun borrow_mut_field(self : &mut Resource) -> &mut u64 :=
    &mut self.value

  public fun vector_value(self : &Resource, index : u64) -> u64 :=
    self.values[index]

  public fun vector_borrow(self : &Resource, index : u64) -> &u64 :=
    &self.values[index]

  public fun vector_borrow_mut(self : &mut Resource, index : u64) -> &mut u64 :=
    &mut self.values[index]

  public fun vector_write(
    self : &mut Resource, index : u64, value : u64
  ) -> Unit := self.values[index] := value

  public fun storage_field(address : Address) -> u64 := Resource[address].value

  public fun storage_borrow(address : Address) -> &Resource :=
    &Resource[address]

  public fun storage_borrow_mut(address : Address) -> &mut Resource :=
    &mut Resource[address]

  public fun storage_borrow_field(address : Address) -> &u64 :=
    &Resource[address].value

  public fun storage_borrow_mut_field(address : Address) -> &mut u64 :=
    &mut Resource[address].value

  public fun storage_write(address : Address, value : u64) -> Unit :=
    Resource[address].value := value

  public fun storage_write_resource(
    address : Address, value : Resource
  ) -> Unit :=
    Resource[address] := value
