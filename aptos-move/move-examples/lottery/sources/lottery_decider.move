module lottery::lottery_decider {
    use lottery::lottery;

    entry fun decide_winners() {
        lottery::decide_winners();
    }
}
