script {
    use velor_framework::velor_coin;
    use velor_framework::coin;

    use std::signer;

    /// An example of a **test-and-abort** attack that fails thanks to the use of a private entry function
    /// being marked as *private* entry function.
    fun main(attacker: &signer) {
        let attacker_addr = signer::address_of(attacker);

        let old_balance = coin::balance<velor_coin::VelorCoin>(attacker_addr);

        // SECURITY: The fact that `randomly_pick_winner` is a *private* entry function is what
        // prevents this call here. The compiler will output the following error:
        //
        // ```
        //    error[E04001]: restricted visibility
        //    |- /tmp/velor-core/velor-move/move-examples/raffle/scripts/test_and_abort_attack_defeated.move:19:9
        //    |
        //    19 |         raffle::raffle::randomly_pick_winner();
        //    |         ^^^^^^^^^^^^^^^^^^^^^^^^^ Invalid call to '(raffle=0xC3BB8488AB1A5815A9D543D7E41B0E0DF46A7396F89B22821F07A4362F75DDC5)::raffle::randomly_pick_winner'
        //    |
        //    |- /tmp/velor-core/velor-move/move-examples/raffle/sources/raffle.move:122:15
        //    |
        //    122 |     entry fun randomly_pick_winner() acquires raffle, Credentials {
        //    |               -------------- This function is internal to its module. Only 'public' and 'public(friend)' functions can be called outside of their module
        // ```

        // TODO: Uncomment this call to reproduce the error above & see the attack failing.
        // (Commented out to ensure this Move example compiles.)
        //raffle::raffle::randomly_pick_winner_internal();

        let new_balance = coin::balance<velor_coin::VelorCoin>(attacker_addr);

        // The attacker can see if his balance remained the same. If it did, then
        // the attacker knows they did NOT win the raffle and can abort everything.
        if (new_balance == old_balance) {
            abort (1)
        };
    }
}
