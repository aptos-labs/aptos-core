-- Copyright © Aptos Foundation

--# publish

import Move

open scoped Move

address_alias application = 0x42

module LeanerAddresses at application where

  @[move_public]
  fun own_address : Address :=
    @application

  @[move_public]
  fun literal_address : Address :=
    @0xCAFE

  @[move_public]
  fun is_application (address : Address) : Bool :=
    address == @application

--# run 0x42::LeanerAddresses::own_address

--# run 0x42::LeanerAddresses::literal_address

--# run 0x42::LeanerAddresses::is_application --args @0x42

--# run 0x42::LeanerAddresses::is_application --args @0x43
