// Port of the Are We Fast Yet "Towers" benchmark (Towers of Hanoi).
//
// This code is based on the SOM class library.
//
// Copyright (c) 2001-2016 see AUTHORS.md file
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the 'Software'), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED 'AS IS', WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
// THE SOFTWARE.
module bench::towers {
    use std::vector;

    const NULL: u64 = 18446744073709551615; // u64::MAX

    /// A larger disk may not be placed on a smaller one.
    const EBIG_DISK_ON_SMALL: u64 = 1;
    /// A disk was popped from an empty pile.
    const EEMPTY_PILE: u64 = 2;
    /// `run` produced a move count other than `expected`.
    const EWRONG_RESULT: u64 = 3;

    struct Disk has copy, drop, store {
        size: u64,
        next: u64,
    }

    /// Move has no recursive structs, so each pile is a singly linked list of
    /// indices into `nodes` rather than a chain of references. Retaining `next`
    /// instead of collapsing a pile into a plain stack preserves the field
    /// traffic of the original, which is most of what this benchmark measures.
    struct Towers has drop {
        nodes: vector<Disk>,
        piles: vector<u64>,
        moves_done: u64,
    }

    fun new_towers(): Towers {
        Towers {
            nodes: vector::empty<Disk>(),
            piles: vector[NULL, NULL, NULL],
            moves_done: 0,
        }
    }

    fun alloc_disk(t: &mut Towers, size: u64): u64 {
        let idx = vector::length(&t.nodes);
        vector::push_back(&mut t.nodes, Disk { size, next: NULL });
        idx
    }

    fun push_disk(t: &mut Towers, disk: u64, pile: u64) {
        let top = *vector::borrow(&t.piles, pile);
        if (top != NULL) {
            let size = vector::borrow(&t.nodes, disk).size;
            let top_size = vector::borrow(&t.nodes, top).size;
            assert!(size < top_size, EBIG_DISK_ON_SMALL);
        };
        vector::borrow_mut(&mut t.nodes, disk).next = top;
        *vector::borrow_mut(&mut t.piles, pile) = disk;
    }

    fun pop_disk_from(t: &mut Towers, pile: u64): u64 {
        let top = *vector::borrow(&t.piles, pile);
        assert!(top != NULL, EEMPTY_PILE);
        let next = vector::borrow(&t.nodes, top).next;
        *vector::borrow_mut(&mut t.piles, pile) = next;
        vector::borrow_mut(&mut t.nodes, top).next = NULL;
        top
    }

    fun move_top_disk(t: &mut Towers, from_pile: u64, to_pile: u64) {
        let disk = pop_disk_from(t, from_pile);
        push_disk(t, disk, to_pile);
        t.moves_done = t.moves_done + 1;
    }

    /// Pushes sizes `disks` down to 0, so `disks` of 13 stacks fourteen disks.
    /// The original counts down from `disks` inclusive; the extra disk stays
    /// parked at the bottom of the pile and is never moved.
    fun build_tower_at(t: &mut Towers, pile: u64, disks: u64) {
        let i = disks + 1;
        while (i > 0) {
            i = i - 1;
            let disk = alloc_disk(t, i);
            push_disk(t, disk, pile);
        }
    }

    fun move_disks(t: &mut Towers, disks: u64, from_pile: u64, to_pile: u64) {
        if (disks == 1) {
            move_top_disk(t, from_pile, to_pile);
        } else {
            let other_pile = (3 - from_pile) - to_pile;
            move_disks(t, disks - 1, from_pile, other_pile);
            move_top_disk(t, from_pile, to_pile);
            move_disks(t, disks - 1, other_pile, to_pile);
        }
    }

    /// Runs `iters` rounds of Hanoi and returns the total number of moves.
    /// Every round starts from a fresh tower, so the result is always
    /// `iters * ((1 << disks) - 1)`.
    public fun bench_towers(disks: u64, iters: u64): u64 {
        let total = 0;
        let i = 0;
        while (i < iters) {
            let t = new_towers();
            build_tower_at(&mut t, 0, disks);
            // The original never calls `move_disks` with 0. Skipping the call
            // keeps `disks` of 0 at zero moves rather than underflowing.
            if (disks > 0) {
                move_disks(&mut t, disks, 0, 1);
            };
            total = total + t.moves_done;
            i = i + 1;
        };
        total
    }

    /// `expected` is an argument rather than a constant so that no part of the
    /// kernel can be folded away at compile time.
    public entry fun run(_s: &signer, disks: u64, iters: u64, expected: u64) {
        assert!(bench_towers(disks, iters) == expected, EWRONG_RESULT);
    }

    #[test]
    fun test_awfy_reference() {
        assert!(bench_towers(13, 1) == 8191, 0);
    }

    #[test]
    fun test_rounds_accumulate() {
        assert!(bench_towers(13, 3) == 24573, 0);
    }

    #[test]
    fun test_three_disks() {
        assert!(bench_towers(3, 1) == 7, 0);
    }

    #[test]
    fun test_moves_per_round() {
        let d = 0;
        while (d <= 10) {
            let per_round = (1u64 << (d as u8)) - 1;
            assert!(bench_towers(d, 1) == per_round, d);
            assert!(bench_towers(d, 4) == per_round * 4, d);
            d = d + 1;
        }
    }

    #[test]
    #[expected_failure(abort_code = EBIG_DISK_ON_SMALL)]
    fun test_big_disk_on_small() {
        let t = new_towers();
        let small = alloc_disk(&mut t, 0);
        let big = alloc_disk(&mut t, 1);
        push_disk(&mut t, small, 0);
        push_disk(&mut t, big, 0);
    }

    #[test]
    #[expected_failure(abort_code = EEMPTY_PILE)]
    fun test_pop_empty_pile() {
        let t = new_towers();
        pop_disk_from(&mut t, 0);
    }

    #[test(s = @bench)]
    fun test_run(s: signer) {
        run(&s, 13, 1, 8191);
    }

    #[test(s = @bench)]
    #[expected_failure(abort_code = EWRONG_RESULT)]
    fun test_run_wrong_expected(s: signer) {
        run(&s, 13, 1, 8190);
    }
}
