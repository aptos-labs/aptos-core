// Bounded loop-invariant evidence unrolls *every* loop in the function, not
// only the one it reports on, so the bounded DAG carries on the order of
// `(depth + 1)^loops` paths and a full weakest precondition runs once per loop
// head over that DAG. At the default depth of 3 that is 4 paths for one loop
// and roughly a million for ten.
//
// `many_loops` below is past the budget. The evidence is a diagnostic -- it
// suggests an invariant, it does not establish one -- so exceeding the budget
// must produce a note saying why no evidence is offered, and inference itself
// must still emit its contract. Before the budget existed this function did not
// terminate.
//
// `one_loop` is the control: comfortably inside the budget, it still receives
// bounded head facts, so the budget cannot be satisfied by disabling evidence.
module 0x42::loop_evidence_budget {

    fun one_loop(n: u64): u64 {
        let total = 0;
        let i = 0;
        while (i < n) {
            total = total + i;
            i = i + 1;
        };
        total
    }

    fun many_loops(n: u64): u64 {
        let total = 0;
        let i = 0;
        while (i < n) { total = total + i; i = i + 1; };
        let i = 0;
        while (i < n) { total = total + i; i = i + 1; };
        let i = 0;
        while (i < n) { total = total + i; i = i + 1; };
        let i = 0;
        while (i < n) { total = total + i; i = i + 1; };
        let i = 0;
        while (i < n) { total = total + i; i = i + 1; };
        let i = 0;
        while (i < n) { total = total + i; i = i + 1; };
        total
    }
}
