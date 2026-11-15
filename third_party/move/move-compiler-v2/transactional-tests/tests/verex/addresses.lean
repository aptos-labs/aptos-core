-- Copyright © Aptos Foundation

--# publish

import VerexMove

verex module 0x42::VerexAddresses where
  public fun own_address() -> Address := @0x42

  public fun literal_address() -> Address := @0xCAFE

  public fun is_application(address : Address) -> Bool := address == @0x42

--# run 0x42::VerexAddresses::own_address

--# run 0x42::VerexAddresses::literal_address

--# run 0x42::VerexAddresses::is_application --args @0x42

--# run 0x42::VerexAddresses::is_application --args @0x43
