// Recursive sudoku enumeration, shaped like SPEC CPU2017's 548.exchange2.
// Cells are visited in a fixed row-major order with no minimum-remaining-values
// heuristic, and every completion is counted rather than stopping at the first.
// Both choices keep the search tree deep and wide on purpose: the workload is
// the recursion and the backtracking, not the solving.

module bench::exchange2 {
    use std::vector;

    /// The computed solution count did not match `expected`.
    const EBAD_RESULT: u64 = 1;
    /// `puzzle_id` is not in the built-in table.
    const EBAD_PUZZLE_ID: u64 = 2;
    /// The grid was not 81 cells, or held a value above 9.
    const EBAD_GRID: u64 = 3;

    /// Number of grids in the built-in table.
    const NUM_PUZZLES: u64 = 6;

    // The table is a cost ladder: each grid is roughly an order of magnitude
    // past the one before it. Every solution count below was cross-checked
    // against a throwaway minimum-remaining-values solver, which visits cells
    // in a different order and so cannot share a bug with the kernel here.

    /// 30 givens, unique solution. 231 search nodes.
    const PUZZLE_0: vector<u8> = vector[
        0, 8, 0, 4, 2, 0, 5, 3, 0,
        0, 0, 7, 0, 6, 3, 8, 0, 0,
        2, 0, 3, 1, 7, 0, 0, 0, 0,
        1, 0, 0, 0, 0, 7, 0, 0, 4,
        4, 6, 2, 0, 8, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 9, 0, 0,
        0, 0, 0, 0, 5, 0, 4, 7, 0,
        5, 0, 0, 0, 9, 0, 0, 0, 2,
        7, 0, 0, 0, 1, 4, 0, 0, 0,
    ];

    /// 30 givens, 28 solutions. 1196 search nodes. Under-constrained, so the
    /// tree stays wide near the leaves instead of collapsing to one path.
    const PUZZLE_1: vector<u8> = vector[
        6, 0, 0, 0, 0, 1, 5, 4, 3,
        9, 0, 0, 5, 0, 0, 0, 8, 0,
        0, 0, 8, 2, 7, 4, 1, 9, 6,
        3, 0, 0, 4, 0, 5, 0, 2, 9,
        0, 0, 5, 0, 0, 7, 0, 1, 0,
        0, 0, 4, 6, 0, 0, 0, 0, 0,
        0, 0, 3, 0, 0, 0, 2, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0,
        2, 1, 0, 0, 0, 3, 0, 0, 0,
    ];

    /// 17 givens, unique solution. 2836 search nodes. 17 is the proven minimum
    /// for uniqueness, so 64 of the 81 cells are empty.
    const PUZZLE_2: vector<u8> = vector[
        6, 0, 0, 0, 0, 0, 7, 3, 0,
        0, 0, 0, 0, 0, 5, 2, 0, 0,
        8, 0, 0, 0, 0, 9, 0, 0, 0,
        0, 7, 0, 0, 2, 0, 4, 0, 0,
        0, 5, 1, 0, 0, 0, 0, 0, 0,
        0, 9, 0, 6, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 9,
        0, 0, 0, 0, 7, 0, 0, 0, 0,
        0, 0, 0, 4, 0, 0, 0, 0, 0,
    ];

    /// 25 givens, unique solution. 21682 search nodes.
    const PUZZLE_3: vector<u8> = vector[
        0, 0, 2, 5, 0, 0, 0, 0, 6,
        3, 0, 0, 2, 7, 0, 5, 0, 0,
        0, 0, 0, 0, 4, 6, 3, 0, 0,
        0, 6, 0, 0, 0, 8, 4, 0, 0,
        4, 0, 9, 0, 0, 0, 0, 0, 0,
        0, 2, 0, 3, 0, 0, 0, 0, 5,
        7, 8, 0, 0, 0, 0, 0, 5, 0,
        0, 0, 0, 4, 0, 0, 7, 0, 0,
        0, 0, 1, 0, 0, 0, 0, 8, 0,
    ];

    /// 17 givens, unique solution. 156189 search nodes.
    const PUZZLE_4: vector<u8> = vector[
        0, 5, 0, 9, 0, 0, 0, 0, 0,
        0, 0, 0, 4, 0, 0, 7, 0, 0,
        6, 8, 0, 0, 0, 0, 1, 0, 0,
        0, 0, 0, 0, 0, 1, 0, 0, 4,
        0, 0, 0, 0, 0, 0, 0, 3, 9,
        0, 2, 0, 0, 5, 0, 0, 0, 8,
        0, 0, 0, 0, 8, 0, 0, 0, 0,
        0, 0, 4, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 2, 0, 0, 0,
    ];

    /// 30 givens, 1032 solutions. 1599860 search nodes, the top of the ladder.
    const PUZZLE_5: vector<u8> = vector[
        7, 0, 5, 0, 0, 0, 0, 0, 6,
        4, 0, 9, 6, 7, 0, 0, 0, 0,
        6, 0, 0, 0, 0, 0, 0, 0, 7,
        0, 0, 0, 0, 0, 0, 0, 0, 4,
        0, 0, 3, 5, 0, 0, 0, 7, 0,
        5, 0, 0, 4, 0, 0, 3, 0, 9,
        3, 0, 7, 1, 0, 0, 0, 0, 2,
        0, 0, 4, 2, 8, 0, 0, 0, 3,
        8, 5, 2, 0, 0, 3, 0, 9, 0,
    ];

    /// Digit occupancy, one nine-bit mask per row, column and 3x3 box. Bit
    /// `d - 1` set means digit `d` is already placed in that unit.
    struct Masks has drop {
        rows: vector<u64>,
        cols: vector<u64>,
        boxes: vector<u64>,
    }

    /// Number of complete grids reachable from `grid`. Returns 0 if the givens
    /// already conflict.
    public fun count_solutions(grid: &vector<u8>): u64 {
        assert!(vector::length(grid) == 81, EBAD_GRID);

        let masks = Masks {
            rows: vector[0u64, 0, 0, 0, 0, 0, 0, 0, 0],
            cols: vector[0u64, 0, 0, 0, 0, 0, 0, 0, 0],
            boxes: vector[0u64, 0, 0, 0, 0, 0, 0, 0, 0],
        };

        let idx = 0;
        while (idx < 81) {
            let digit = *vector::borrow(grid, idx);
            assert!(digit <= 9, EBAD_GRID);
            if (digit != 0) {
                let r = idx / 9;
                let c = idx % 9;
                let b = (r / 3) * 3 + (c / 3);
                let bit = 1u64 << ((digit - 1) as u8);
                let row = vector::borrow_mut(&mut masks.rows, r);
                if (*row & bit != 0) { return 0 };
                *row = *row | bit;
                let col = vector::borrow_mut(&mut masks.cols, c);
                if (*col & bit != 0) { return 0 };
                *col = *col | bit;
                let bx = vector::borrow_mut(&mut masks.boxes, b);
                if (*bx & bit != 0) { return 0 };
                *bx = *bx | bit;
            };
            idx = idx + 1;
        };

        solve(grid, &mut masks, 0)
    }

    /// Fill cells `idx..81` in order and return how many ways it can be done.
    fun solve(grid: &vector<u8>, masks: &mut Masks, idx: u64): u64 {
        if (idx == 81) {
            return 1
        };
        if (*vector::borrow(grid, idx) != 0) {
            return solve(grid, masks, idx + 1)
        };

        let r = idx / 9;
        let c = idx % 9;
        let b = (r / 3) * 3 + (c / 3);
        // Saved so the restore on the way up is a plain write rather than a
        // second read-modify-write.
        let row_mask = *vector::borrow(&masks.rows, r);
        let col_mask = *vector::borrow(&masks.cols, c);
        let box_mask = *vector::borrow(&masks.boxes, b);
        let used = row_mask | col_mask | box_mask;

        let count = 0;
        let d = 0;
        while (d < 9) {
            let bit = 1u64 << (d as u8);
            if (used & bit == 0) {
                *vector::borrow_mut(&mut masks.rows, r) = row_mask | bit;
                *vector::borrow_mut(&mut masks.cols, c) = col_mask | bit;
                *vector::borrow_mut(&mut masks.boxes, b) = box_mask | bit;
                count = count + solve(grid, masks, idx + 1);
                *vector::borrow_mut(&mut masks.rows, r) = row_mask;
                *vector::borrow_mut(&mut masks.cols, c) = col_mask;
                *vector::borrow_mut(&mut masks.boxes, b) = box_mask;
            };
            d = d + 1;
        };
        count
    }

    fun puzzle(puzzle_id: u64): vector<u8> {
        assert!(puzzle_id < NUM_PUZZLES, EBAD_PUZZLE_ID);
        if (puzzle_id == 0) {
            PUZZLE_0
        } else if (puzzle_id == 1) {
            PUZZLE_1
        } else if (puzzle_id == 2) {
            PUZZLE_2
        } else if (puzzle_id == 3) {
            PUZZLE_3
        } else if (puzzle_id == 4) {
            PUZZLE_4
        } else {
            PUZZLE_5
        }
    }

    /// Enumerate built-in grid `puzzle_id` `iters` times and sum the counts.
    public fun bench_exchange2(puzzle_id: u64, iters: u64): u64 {
        bench_exchange2_grid(puzzle(puzzle_id), iters)
    }

    /// Same, over a caller-supplied 81-cell grid in row-major order, 0 = empty.
    public fun bench_exchange2_grid(grid: vector<u8>, iters: u64): u64 {
        let acc = 0;
        let i = 0;
        while (i < iters) {
            acc = acc + count_solutions(&grid);
            i = i + 1;
        };
        acc
    }

    public entry fun run(_s: &signer, puzzle_id: u64, iters: u64, expected: u64) {
        assert!(bench_exchange2(puzzle_id, iters) == expected, EBAD_RESULT);
    }

    // Puzzles 0 to 3 are asserted below; 4 and 5 are harness knobs, with their
    // counts in the README. A unit test is bounded at 1e9 internal gas units
    // and no CLI flag reaches that bound. Measured, it runs out somewhere
    // between 173456 and 216820 branch nodes. PUZZLE_5 is ten times past it.
    // PUZZLE_4 does fit, but by under 1.4x and at 17 seconds on its own, which
    // is too thin and too slow to pin a test to.

    #[test_only]
    /// 40 givens, unique solution. 156 search nodes.
    const EASY: vector<u8> = vector[
        0, 7, 9, 0, 6, 0, 5, 0, 8,
        0, 6, 0, 0, 9, 1, 7, 0, 2,
        0, 0, 0, 0, 5, 0, 0, 0, 0,
        0, 0, 0, 4, 0, 6, 0, 2, 1,
        0, 8, 0, 9, 0, 3, 0, 0, 0,
        0, 2, 4, 0, 1, 5, 9, 0, 6,
        5, 0, 0, 0, 0, 7, 6, 8, 3,
        0, 0, 7, 5, 0, 2, 1, 9, 0,
        4, 1, 8, 0, 3, 9, 0, 0, 7,
    ];

    #[test]
    fun test_puzzle_0() {
        assert!(bench_exchange2(0, 1) == 1, 0);
    }

    #[test]
    fun test_puzzle_1_many_solutions() {
        assert!(bench_exchange2(1, 1) == 28, 0);
    }

    #[test]
    fun test_puzzle_2_seventeen_givens() {
        assert!(bench_exchange2(2, 1) == 1, 0);
    }

    #[test]
    fun test_puzzle_3() {
        assert!(bench_exchange2(3, 1) == 1, 0);
    }

    #[test]
    fun test_grid_surface() {
        assert!(bench_exchange2_grid(EASY, 1) == 1, 0);
    }

    #[test]
    fun test_contradiction_has_no_solutions() {
        // Two 5s in row 0.
        let grid = vector[
            5, 0, 0, 0, 5, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];
        assert!(bench_exchange2_grid(grid, 4) == 0, 0);
    }

    #[test]
    fun test_iters_scales() {
        assert!(bench_exchange2(1, 2) == 2 * bench_exchange2(1, 1), 0);
    }

    #[test]
    fun test_zero_iters() {
        assert!(bench_exchange2(5, 0) == 0, 0);
    }

    #[test(s = @bench)]
    fun test_run(s: signer) {
        run(&s, 1, 2, 56);
    }

    #[test(s = @bench)]
    #[expected_failure(abort_code = EBAD_RESULT)]
    fun test_run_wrong_expected(s: signer) {
        run(&s, 1, 2, 55);
    }

    #[test(s = @bench)]
    #[expected_failure(abort_code = EBAD_PUZZLE_ID)]
    fun test_run_bad_puzzle_id(s: signer) {
        run(&s, NUM_PUZZLES, 1, 0);
    }

    #[test]
    #[expected_failure(abort_code = EBAD_GRID)]
    fun test_short_grid() {
        bench_exchange2_grid(vector[1u8, 2, 3], 1);
    }

    #[test]
    #[expected_failure(abort_code = EBAD_GRID)]
    fun test_digit_out_of_range() {
        let grid = EASY;
        *vector::borrow_mut(&mut grid, 0) = 10;
        bench_exchange2_grid(grid, 1);
    }
}
