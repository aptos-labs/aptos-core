# Code for "AIP-41: Move APIs for randomness generation"

This is the code that was included in [AIP-41](https://github.com/aptos-foundation/AIPs/blob/main/aips/aip-41.md) (discussion [here](https://github.com/aptos-foundation/AIPs/issues/185)), modulo some cosmetic edits.

The purpose of the code is to help AIP-41 showcase two things:

1. The API interface of the proposed `aptos_std::randomness` module, in [`sources/randomness.move`](sources/randomness.move)
1. An example of an **insecure** lottery app in [`sources/lottery_insecure.move`](sources/lottery_insecure.move)
   - **TODO:** An example of an **undergasing** attack succeeding on this lottery in [`../tests/move_unit_tests.rs`](../tests/move_unit_tests.rs)?
1. An example of a *secure* lottery app in [`sources/lottery_secure.move`](sources/lottery_secure.move)

In addition, we make sure this code is syntactically-correct as well as semantically-correct via some tests in [`sources/lottery_test.move`](sources/lottery_test.move).
