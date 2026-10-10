-- Copyright © Aptos Foundation

--# publish

import LeanerMove

leaner module 0x42::LeanerAddresses where
  public fun own_address() -> Address := @0x42

  public fun literal_address() -> Address := @0xCAFE

  public fun is_application(address : Address) -> Bool := address == @0x42

--# run 0x42::LeanerAddresses::own_address

--# run 0x42::LeanerAddresses::literal_address

--# run 0x42::LeanerAddresses::is_application --args @0x42

--# run 0x42::LeanerAddresses::is_application --args @0x43
