// Two map writes must each be applied exactly once.
// inference-reject-mutation: // MUTATION_POINT => simple_map::remove(m, &a);
module 0x42::intrinsic_map_writes {
    use aptos_std::simple_map::{Self, SimpleMap};

    fun add_twice(m: &mut SimpleMap<u64, u64>, a: u64, b: u64) {
        simple_map::add(m, a, 10);
        simple_map::add(m, b, 20);
        // MUTATION_POINT
    }
    spec add_twice(m: &mut 0x1::simple_map::SimpleMap<u64, u64>, a: u64, b: u64) {
        use 0x1::simple_map;
        pragma opaque = true;
        ensures [inferred] m == simple_map::spec_set<u64, u64>(simple_map::spec_set<u64, u64>(old(m), a, 10), b, 20);
        aborts_if [inferred] simple_map::spec_aborts_add<u64, u64>(m, a, 10);
        aborts_if [inferred] simple_map::spec_aborts_add<u64, u64>(simple_map::spec_set<u64, u64>(m, a, 10), b, 20);
    }

}
/*
Verification: Succeeded.
Mutation: Rejected by postcondition.
*/
