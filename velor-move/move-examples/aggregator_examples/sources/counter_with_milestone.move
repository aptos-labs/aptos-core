/// This is a module that showcases how to use new AggregatorV2 parallelism and branch prediction features
/// to create a global counter, which is checked for specific milestones,
/// and transactions that cross the milestone have special logic to celebrate the milestone.
/// Checking the milestone only creates a reduced parallelism right around the point where
/// milestone is reached, and so if milestones are spread out (like a thousand or more apart),
/// there should be no impact on throughput / parallelism overall.
///
/// In a very similar way, milestones for token (Digital Asset) minting can be processed,
/// by using collection::is_total_minted_at_least call, as Digital Asset collections track their
/// supply and total minted thus far.
module aggregator_examples::counter_with_milestone {
    use std::error;
    use std::signer;
    use velor_framework::aggregator_v2::{Self, Aggregator};
    use velor_framework::event;

    // Resource being modified doesn't exist
    const ERESOURCE_NOT_PRESENT: u64 = 2;

    // Incrementing a counter failed
    const ECOUNTER_INCREMENT_FAIL: u64 = 4;

    const ENOT_AUTHORIZED: u64 = 5;

    struct MilestoneCounter has key {
        next_milestone: u64,
        milestone_every: u64,
        count: Aggregator<u64>,
    }

    #[event]
    struct MilestoneReached has drop, store {
        milestone: u64,
    }

    // Create the global `MilestoneCounter`.
    // Stored under the module publisher address.
    public entry fun create(publisher: &signer, milestone_every: u64) {
        assert!(
            signer::address_of(publisher) == @aggregator_examples,
            ENOT_AUTHORIZED,
        );

        move_to<MilestoneCounter>(
            publisher,
            MilestoneCounter {
                next_milestone: milestone_every,
                milestone_every,
                count: aggregator_v2::create_unbounded_aggregator(),
            }
        );
    }

    public entry fun increment_milestone() acquires MilestoneCounter {
        assert!(exists<MilestoneCounter>(@aggregator_examples), error::invalid_argument(ERESOURCE_NOT_PRESENT));
        let milestone_counter = borrow_global_mut<MilestoneCounter>(@aggregator_examples);
        assert!(aggregator_v2::try_add(&mut milestone_counter.count, 1), ECOUNTER_INCREMENT_FAIL);

        if (aggregator_v2::is_at_least(&milestone_counter.count, milestone_counter.next_milestone) && !aggregator_v2::is_at_least(&milestone_counter.count, milestone_counter.next_milestone + 1)) {
            event::emit(MilestoneReached { milestone: milestone_counter.next_milestone});
            milestone_counter.next_milestone = milestone_counter.next_milestone + milestone_counter.milestone_every;
        }
    }
}
