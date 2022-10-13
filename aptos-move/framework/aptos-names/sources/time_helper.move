module aptos_names::time_helper {

    const SECONDS_PER_MINUTE: u64 = 60;
    const SECONDS_PER_HOUR: u64 = 60 * 60;
    const SECONDS_PER_DAY: u64 = 60 * 60 * 24;
    const SECONDS_PER_WEEK: u64 = 60 * 60 * 24 * 7;
    const SECONDS_PER_YEAR: u64 = 60 * 60 * 24 * 365;

    public fun minutes_to_seconds(minutes: u64): u64 {
        SECONDS_PER_MINUTE * minutes
    }

    public fun days_to_seconds(days: u64): u64 {
        SECONDS_PER_DAY * days
    }

    public fun seconds_to_days(seconds: u64): u64 {
        seconds / SECONDS_PER_DAY
    }

    public fun seconds_to_years(seconds: u64): u64 {
        seconds / SECONDS_PER_YEAR
    }

    public fun hours_to_seconds(hours: u64): u64 {
        SECONDS_PER_HOUR * hours
    }

    public fun weeks_to_seconds(weeks: u64): u64 {
        SECONDS_PER_WEEK * weeks
    }

    public fun years_to_seconds(years: u64): u64 {
        SECONDS_PER_YEAR * years
    }


    #[test]
    fun test_time_conversion()
    {
        assert!(minutes_to_seconds(1) == 60, minutes_to_seconds(1));
        assert!(minutes_to_seconds(60) == hours_to_seconds(1), minutes_to_seconds(1));

        assert!(days_to_seconds(1) == minutes_to_seconds(1) * 60 * 24, days_to_seconds(1));
        assert!(weeks_to_seconds(1) == days_to_seconds(1) * 7, weeks_to_seconds(1));
        assert!(hours_to_seconds(24) == days_to_seconds(1), hours_to_seconds(24));

        assert!(years_to_seconds(1) == days_to_seconds(1) * 365, years_to_seconds(1));

        assert!(1 == seconds_to_years(years_to_seconds(1)), seconds_to_years(years_to_seconds(1)));
    }
}
