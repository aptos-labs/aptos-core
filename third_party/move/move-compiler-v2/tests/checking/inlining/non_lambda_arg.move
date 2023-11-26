module 0x42::sort {
    use std::vector;

    public fun incorrect_sort<T: copy>(arr: &mut vector<T>, a_less_b: |T, T| bool) {
        let n = vector::length(arr);
        incorrect_sort_recursive<T>(arr, 0, n - 1, a_less_b)
    }

    public fun incorrect_sort_recursive<T: copy>(arr: &mut vector<T>, low: u64, high: u64, a_less_b: |T, T| bool) {
        if (low < high) {
            let pi = low + high / 2;
            incorrect_sort_recursive(arr, low, pi - 1, a_less_b);
            incorrect_sort_recursive(arr, pi + 1, high, a_less_b);
        };
    }

}
