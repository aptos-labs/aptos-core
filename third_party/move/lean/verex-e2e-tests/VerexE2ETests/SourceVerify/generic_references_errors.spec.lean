-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec id_write where
  ensures result == 7

spec id_write_false where
  ensures result == v -- error: the write through the result reaches the lender

spec id_write_vacuous where
  ensures false -- error: the call has an outcome

spec id_lender where
  ensures a == 7

spec id_lender_false where
  ensures a == old(a) -- error: the write through the result reaches the lender

spec id_at_value where
  ensures result == v

spec wrapper_field where
  ensures result == 7

spec wrapper_field_false where
  ensures result == v -- error: the write through the field reaches the lender

spec wrapper_field_vacuous where
  ensures false -- error: the call has an outcome

spec wrapper_through where
  ensures result == 7

spec wrapper_through_false where
  ensures result == v -- error: the write through the result reaches the lender

spec wrapper_through_vacuous where
  ensures false -- error: the call has an outcome

spec option_write where
  ensures result == 7

spec option_write_false where
  ensures result == v -- error: the write through the payload reaches the lender

spec option_write_vacuous where
  ensures false -- error: the call has an outcome

spec get_write where
  requires w <= 1000
  ensures result == 7 + w

spec get_write_false where
  requires w <= 1000
  ensures result == v + w -- error: the write through the result reaches the lender

spec get_write_vacuous where
  requires w <= 1000
  ensures false -- error: the call has an outcome

spec twice_write where
  ensures result == 7

spec twice_write_false where
  ensures result == v -- error: the write through the result reaches the lender

spec twice_write_vacuous where
  ensures false -- error: the call has an outcome

spec generic_at_reference where
  ensures result == 7

spec generic_at_reference_false where
  ensures result == v -- error: the write through the result reaches the lender

spec generic_at_reference_vacuous where
  ensures false -- error: the call has an outcome

spec consume_reference where
  ensures result == v -- error: the callee drops what it takes, and has no place to resolve it

spec consume_value where
  ensures result == v

spec consume_value_false where
  ensures result == v + 1 -- error: the callee leaves its argument

spec consume_value_vacuous where
  ensures false -- error: the call has an outcome

spec opaque_id where
  pragma opaque
  ensures result == x

spec opaque_reference where
  ensures result == 7 -- error: an opaque callee's generic contract does not transfer

spec opaque_value where
  ensures result == v

spec opaque_value_false where
  ensures result == v + 1 -- error: the contract says otherwise

spec opaque_value_vacuous where
  ensures false -- error: the contract has an outcome

spec recursive_reference where
  ensures result == 7 -- error: a recursive callee has no reading at the reference

spec native_reference where
  ensures true -- error: a native's generic contract does not transfer

spec observed_reference where
  ensures result == 7

spec observed_reference_false where
  ensures result == v -- error: it writes through the returned reference

spec observed_reference_vacuous where
  ensures false -- error: the call has an outcome

spec observed_value where
  ensures result == v

spec observed_value_false where
  ensures result == v + 1 -- error: the callee returns its argument

spec observed_value_vacuous where
  ensures false -- error: the call has an outcome

spec viewing_value where
  ensures result == v

spec viewing_value_false where
  ensures result == v + 1 -- error: the callee returns its argument

spec viewing_value_vacuous where
  ensures false -- error: the call has an outcome

spec nested_move where
  ensures result == 7 -- error: a move of a holder out of an aggregate has no carrier yet

spec holder_mem_drop where
  ensures result == 7

spec holder_dropped where
  ensures result == 7

spec holder_dropped_false where
  ensures result == v -- error: the write reaches the lender

spec holder_scope_end where
  ensures result == 7

spec holder_scope_end_false where
  ensures result == v -- error: the write reaches the lender

spec holder_scope_end_vacuous where
  ensures false -- error: the holder's death has an outcome

spec push_view where
  ensures result == 1 -- error: `Vec` methods are native, and their type argument mentions a mutable reference
