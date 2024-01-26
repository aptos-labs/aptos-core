# Code for "AIP-41: Move APIs for randomness generation"

The purpose of the code is to help AIP-41 showcase two things:

1. The API interface of the proposed `aptos_std::randomness` module, in [`sources/randomness.move`](sources/randomness.move)
1. An example of a potentially-**insecure** lottery app, show undergasing attacks be feasible, in [`sources/lottery_insecure.move`](sources/lottery_insecure.move)
   - **TODO:** An example of an **undergasing** attack succeeding on this lottery in [`../tests/move_unit_tests.rs`](../tests/move_unit_tests.rs)?
   - An example of a *secure* lottery app in [`sources/lottery_secure.move`](sources/lottery_secure.move)

In addition, we some reasonable tests in [`sources/lottery_test.move`](sources/lottery_test.move).
