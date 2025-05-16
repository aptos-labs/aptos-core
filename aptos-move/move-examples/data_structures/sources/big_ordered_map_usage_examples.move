module aptos_std::big_ordered_map_usage_examples {
    use aptos_framework::big_ordered_map;

    #[test]
    fun example_with_primitive_types() {
        let map = big_ordered_map::new<u64, u64>();

        map.add(2, 2);
        map.add(3, 3);

        assert!(map.contains(&2));

        let sum = 0;
        map.for_each_ref(|k, v| sum += *k + *v);
        assert!(sum == 10);

        *map.borrow_mut(&2) = 5;
        assert!(map.get(&2).destroy_some() == 5);

        map.for_each_mut(|_k, v| *v += 1);

        let sum = 0;
        map.for_each(|k, v| sum += k + v);
        assert!(sum == 15);
    }
}
