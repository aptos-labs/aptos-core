// Port of the Are We Fast Yet `Queens` benchmark.
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

module bench::queens {
    use std::vector;

    /// `run` was given an `expected` that the kernel did not produce.
    const EBAD_RESULT: u64 = 1;

    /// Stands in for the `-1` the Java fills `queenRows` with.
    const NULL: u64 = 18446744073709551615; // u64::MAX

    struct Board has drop {
        n: u64,
        free_rows: vector<bool>,
        free_maxs: vector<bool>,
        free_mins: vector<bool>,
        queen_rows: vector<u64>,
    }

    /// An empty `n` by `n` board. The diagonal vectors are `2n` long, as in the
    /// Java. That is one slot more than the `2n - 1` distinct diagonals.
    fun new_board(n: u64): Board {
        let free_rows = vector::empty<bool>();
        let queen_rows = vector::empty<u64>();
        let i = 0;
        while (i < n) {
            vector::push_back(&mut free_rows, true);
            vector::push_back(&mut queen_rows, NULL);
            i = i + 1;
        };

        let free_maxs = vector::empty<bool>();
        let free_mins = vector::empty<bool>();
        let diags = 2 * n;
        i = 0;
        while (i < diags) {
            vector::push_back(&mut free_maxs, true);
            vector::push_back(&mut free_mins, true);
            i = i + 1;
        };

        Board { n, free_rows, free_maxs, free_mins, queen_rows }
    }

    /// Whether a queen may sit on row `r` of column `c`.
    ///
    /// The Java indexes `freeMins` as `[c - r + 7]`. On `u64` that underflows
    /// whenever `c < r`, so the port adds before subtracting. `c + n - r - 1`
    /// is the same number and never underflows for `r < n`.
    fun get_row_column(board: &Board, r: u64, c: u64): bool {
        *vector::borrow(&board.free_rows, r)
            && *vector::borrow(&board.free_maxs, c + r)
            && *vector::borrow(&board.free_mins, c + board.n - r - 1)
    }

    fun set_row_column(board: &mut Board, r: u64, c: u64, v: bool) {
        let n = board.n;
        *vector::borrow_mut(&mut board.free_rows, r) = v;
        *vector::borrow_mut(&mut board.free_maxs, c + r) = v;
        *vector::borrow_mut(&mut board.free_mins, c + n - r - 1) = v;
    }

    /// Place a queen in column `c` and recurse, backtracking on failure.
    ///
    /// `queen_rows` is never cleared on backtrack, matching the Java. A
    /// solution occupies every row, so each entry ends up holding the column of
    /// the queen that finally claimed its row.
    fun place_queen(board: &mut Board, c: u64): bool {
        let n = board.n;
        let r = 0;
        while (r < n) {
            if (get_row_column(board, r, c)) {
                *vector::borrow_mut(&mut board.queen_rows, r) = c;
                set_row_column(board, r, c, false);

                if (c + 1 == n) {
                    return true
                };
                if (place_queen(board, c + 1)) {
                    return true
                };
                set_row_column(board, r, c, true);
            };
            r = r + 1;
        };
        false
    }

    fun queens(n: u64): bool {
        let board = new_board(n);
        place_queen(&mut board, 0)
    }

    /// Solve the `n` queens problem `iters` times from scratch and return how
    /// many rounds found a solution.
    ///
    /// The Java folds the rounds with `&&`, which stops at the first failure.
    /// Counting instead keeps every round's work on an unsolvable board.
    public fun bench_queens(n: u64, iters: u64): u64 {
        let solved = 0;
        let round = 0;
        while (round < iters) {
            if (queens(n)) {
                solved = solved + 1;
            };
            round = round + 1;
        };
        solved
    }

    public entry fun run(_s: &signer, n: u64, iters: u64, expected: u64) {
        assert!(bench_queens(n, iters) == expected, EBAD_RESULT);
    }

    // The board is unsolvable, so there is no first solution to report.
    #[test_only]
    const ENO_SOLUTION: u64 = 2;

    // The column of the queen on each row of the first solution found. This
    // checks the search order, which a solve count on its own does not.
    #[test_only]
    public fun first_solution(n: u64): vector<u64> {
        let board = new_board(n);
        assert!(place_queen(&mut board, 0), ENO_SOLUTION);
        let Board { n: _, free_rows: _, free_maxs: _, free_mins: _, queen_rows } = board;
        queen_rows
    }

    // The AWFY setting: eight queens, and all ten rounds solve.
    #[test]
    fun test_awfy_reference() {
        assert!(bench_queens(8, 10) == 10, 0);
    }

    // Two and three queens have no solution, so the search exhausts the tree.
    #[test]
    fun test_unsolvable() {
        assert!(bench_queens(2, 1) == 0, 0);
        assert!(bench_queens(3, 1) == 0, 0);
        assert!(bench_queens(2, 5) == 0, 0);
    }

    #[test]
    fun test_solvable() {
        assert!(bench_queens(1, 1) == 1, 0);
        assert!(bench_queens(4, 1) == 1, 0);
        assert!(bench_queens(6, 1) == 1, 0);
        assert!(bench_queens(8, 1) == 1, 0);
    }

    #[test]
    fun test_iters_scale() {
        assert!(bench_queens(8, 3) == 3, 0);
        assert!(bench_queens(6, 7) == 7, 0);
    }

    // An empty board has no column to place a queen in. `n = 0` also catches a
    // port that precomputes `n - 1` for the `free_mins` index, which underflows.
    #[test]
    fun test_degenerate() {
        assert!(bench_queens(0, 1) == 0, 0);
        assert!(bench_queens(8, 0) == 0, 0);
    }

    #[test]
    fun test_first_solution() {
        assert!(first_solution(1) == vector[0], 0);
        assert!(first_solution(6) == vector[3, 0, 4, 1, 5, 2], 0);
        assert!(first_solution(8) == vector[0, 6, 4, 7, 1, 3, 5, 2], 0);
    }

    #[test]
    #[expected_failure(abort_code = ENO_SOLUTION, location = Self)]
    fun test_first_solution_unsolvable() {
        first_solution(3);
    }

    #[test(s = @bench)]
    fun test_run(s: &signer) {
        run(s, 8, 10, 10);
    }

    #[test(s = @bench)]
    #[expected_failure(abort_code = EBAD_RESULT, location = Self)]
    fun test_run_bad_expected(s: &signer) {
        run(s, 8, 10, 9);
    }
}
