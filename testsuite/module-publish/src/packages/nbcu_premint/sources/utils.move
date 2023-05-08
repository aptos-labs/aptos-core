module self::utils {

    use std::string;
    use std::vector;


    // EST offset in seconds from UTC
    // 5 for EST, 4 for EDT
    const EST_OFFSET: u64 = 4 * 60 * 60;
    // number of seconds per day
    const SECONDS_PER_DAY: u64 = 60 * 60 * 24;

    // checks whether two Unix timestamps fall on the same day in EST using midnight as the cutoff
    public fun is_same_day_in_est_midnight(timestamp1: u64, timestamp2: u64): bool {
        // In the event we have an invalid small timestamp, this is the initial condition, so we return false
        if (timestamp1 < 100 || timestamp2 < 100) {
            return false
        };

        // Calculate the number of seconds since midnight EST for each timestamp
        let seconds1 = (timestamp1 - EST_OFFSET) / SECONDS_PER_DAY;
        let seconds2 = (timestamp2 - EST_OFFSET) / SECONDS_PER_DAY;

        // If the seconds since noon are the same, the timestamps are on the same day
        seconds1 == seconds2
    }

    // Legacy backwards compat
    public fun is_same_day_in_pst_noon(_timestamp1: u64, _timestamp2: u64): bool {
        return false
    }

    // checks whether two Unix timestamps are less than a week apart
    public fun is_within_one_week(timestamp1: u64, timestamp2: u64): bool {
        // In the event we have an invalid small timestamp, this is the initial condition, so we return false
        if (timestamp1 < 100 || timestamp2 < 100) {
            return false
        };
        // Ensures timestamp1 <= timestamp2
        if (timestamp1 > timestamp2) {
            let temp = timestamp1;
            timestamp1 = timestamp2;
            timestamp2 = temp;
        };

        (timestamp2 - timestamp1) < SECONDS_PER_DAY * 7
    }

    // Splits a combined timestamp and # times called into a tuple of the timestamp and times called
    public fun combined_to_last_timestamp_and_times(combined: u64): (u64, u64) {
        let last_timestamp = combined / 10;
        let times = combined % 10;
        (last_timestamp, times)
    }

    // Combines a timestamp and # times called into a single u64
    public fun last_timestamp_and_times_to_combined(last_timestamp: u64, times: u64): u64 {
        last_timestamp * 10 + times
    }

    public fun u64_to_string(value: u64): string::String {
        if (value == 0) {
            return string::utf8(b"0")
        };
        let buffer = vector::empty<u8>();
        while (value != 0) {
            vector::push_back(&mut buffer, ((48 + value % 10) as u8));
            value = value / 10;
        };
        vector::reverse(&mut buffer);
        string::utf8(buffer)
    }

    #[test]
    fun test_is_same_day_in_est_midnight() {
        // This is only during DST. If we are not in DST, we need to change this to 0
        let offset_delta_sec =  60 * 60;
        // 1677299008: Fri Feb 24 2023 23:23:28
        // 1677302555: Sat Feb 25 2023 00:22:35
        assert!(!is_same_day_in_est_midnight(1677299008 - offset_delta_sec, 1677302555 - offset_delta_sec), 0);

        // 1678280400: Wed Mar 08 2023 8 AM EST
        // 1678334400: Wed Mar 08 2023 11 PM EST
        assert!(is_same_day_in_est_midnight(1678280400 - offset_delta_sec, 1678334400 - offset_delta_sec), 1);

        // Test starting conditions; i.e invalid start times
        assert!(!is_same_day_in_est_midnight(10, 1677259355 - offset_delta_sec), 2);
    }

    #[test]
    fun test_is_within_one_week() {
        // 1675497600: Sat Feb 04 2023 00:00:00 GMT-0800
        // 1676052000: Fri Feb 10 2023 10:00:00 GMT-0800
        assert!(is_within_one_week(1675497600, 1676052000), 0);

        // 1675706400: Mon Feb 06 2023 18:00:00 GMT+0000
        // 1676052000: Fri Feb 10 2023 10:00:00 GMT-0800
        assert!(is_within_one_week(1675706400, 1676052000), 1);

        // 1675706400: Mon Feb 06 2023 18:00:00 GMT+0000
        // 1675965600: Thu Feb 09 2023 10:00:00 GMT-0800
        assert!(is_within_one_week(1675706400, 1675965600), 2);

        // 1675792800: Tue Feb 07 2023 18:00:00 GMT+0000
        // 1676224800: Sun Feb 12 2023 10:00:00 GMT-0800
        assert!(is_within_one_week(1675792800, 1676224800), 3);

        // 1676224800: Sun Feb 12 2023 10:00:00 GMT-0800
        // 1676233800: Sun Feb 12 2023 12:30:00 GMT-0800
        assert!(is_within_one_week(1676224800, 1676233800), 4);

        // 1676340000: Mon Feb 13 2023 18:00:00 GMT-0800
        // 1676833200: Sun Feb 19 2023 11:00:00 GMT-0800
        assert!(is_within_one_week(1676340000, 1676833200), 5);

        // 1675497600: Sat Feb 04 2023 00:00:00 GMT-0800
        // 1676224800: Sun Feb 12 2023 10:00:00 GMT-0800
        assert!(!is_within_one_week(1675497600, 1676224800), 6);

        // Test starting conditions; i.e invalid start times
        assert!(!is_within_one_week(10, 1676224800), 6);
    }


    #[test]
    fun test_combined_times() {
        // 1677302555: 9:22 PM EST
        let timestamp = 1677302555;
        let times = 3;
        let combined = last_timestamp_and_times_to_combined(timestamp, times);
        let (last_timestamp, last_times) = combined_to_last_timestamp_and_times(combined);
        assert!(last_timestamp == timestamp, 0);
        assert!(last_times == times, 1);
    }


    #[test]
    fun test_u64_to_string() {
        let test_cases: vector<u64> = vector[0, 1, 10, 100, 1000, 987654321, 1000000];
        let expected: vector<string::String> = vector[
            string::utf8(b"0"),
            string::utf8(b"1"),
            string::utf8(b"10"),
            string::utf8(b"100"),
            string::utf8(b"1000"),
            string::utf8(b"987654321"),
            string::utf8(b"1000000"),
        ];
        while (vector::length(&test_cases) > 0) {
            let test_case = vector::pop_back(&mut test_cases);
            let expected = vector::pop_back(&mut expected);
            assert!(u64_to_string(test_case) == expected, test_case);
        };
    }
}
