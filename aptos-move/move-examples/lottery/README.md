# Code for "AIP-41: Move APIs for randomness generation"

This is the code that was included in [AIP-41](https://github.com/aptos-foundation/AIPs/blob/main/aips/aip-41.md) (discussion [here](https://github.com/aptos-foundation/AIPs/issues/185)), modulo some cosmetic edits.

The purpose of the code is to help AIP-41 showcase two things:

 1. The API interface of the proposed `aptos_std::randomness` module, in `sources/randomness.move`
 2. An example of a simple lottery based on `aptos_std::randomness`, in `sources/lottery.move`

In addition, we make sure this code is syntactically-correct as well as semantically-correct via some tests in `sources/lottery_test.move`.