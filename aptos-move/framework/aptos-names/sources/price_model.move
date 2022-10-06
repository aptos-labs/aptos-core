module aptos_names::price_model {
    use aptos_names::config;
    use aptos_std::math64;
    use std::error;


    /// The domain length is too short- currently the minimum is 2 characters
    const EDOMAIN_TOO_SHORT: u64 = 1;

    /// The longer the name is registered for, the more expensive it is per year.
    /// The curve is exponential, with every year costing more than the previous
    fun scale_price_for_years(price: u64, years: u8): u64 {
        // TODO: THIS WHOLE FUNCTION IS A PLACEHOLDER
        let final_price = 0;
        let multiplier = 100;
        let years = (years as u64);
        let i = 1;
        while (i <= years) {
            final_price = final_price + (price * multiplier) / 100;
            multiplier = multiplier + 5 + i * 5;
            i = i + 1;
        };
        final_price
    }

    /// There is a fixed cost per each tier of domain names, from 2 to >=6, and it also scales exponentially with number of years to register
    public fun price_for_domain_v1(domain_length: u64, registration_years: u8): u64 {
        assert!(domain_length >= 2, error::out_of_range(EDOMAIN_TOO_SHORT));
        let length_to_charge_for = math64::min(domain_length, 6);
        scale_price_for_years(config::domain_price_for_length(length_to_charge_for), registration_years)
    }

    /// Subdomains have a fixed unit cost
    public fun price_for_subdomain_v1(_registration_duration_secs: u64): u64 {
        config::subdomain_price()
    }

    #[test(myself = @aptos_names, framework = @0x1)]
    fun test_price_for_domain_v1(myself: &signer, framework: &signer) {
        use aptos_names::config;
        use aptos_framework::aptos_coin::AptosCoin;
        use aptos_framework::coin;
        use aptos_framework::account;
        use std::signer;

        account::create_account_for_test(signer::address_of(myself));
        account::create_account_for_test(signer::address_of(framework));

        config::initialize_aptoscoin_for(framework);
        coin::register<AptosCoin>(myself);
        config::initialize_v1(myself, @aptos_names, @aptos_names);

        config::set_subdomain_price(myself, config::octas() / 5);
        config::set_domain_price_for_length(myself, (100 * config::octas()), 2);
        config::set_domain_price_for_length(myself, (60 * config::octas()), 3);
        config::set_domain_price_for_length(myself, (30 * config::octas()), 4);
        config::set_domain_price_for_length(myself, (15 * config::octas()), 5);
        config::set_domain_price_for_length(myself, (5 * config::octas()), 6);

        let price = price_for_domain_v1(2, 1) / config::octas();
        assert!(price == 100, price);

        let price = price_for_domain_v1(4, 1) / config::octas();
        assert!(price == 30, price);

        let price = price_for_domain_v1(2, 3) / config::octas();
        assert!(price == 335, price);

        let price = price_for_domain_v1(5, 1) / config::octas();
        assert!(price == 15, price);

        let price = price_for_domain_v1(5, 8) / config::octas();
        assert!(price == 204, price);

        let price = price_for_domain_v1(10, 1) / config::octas();
        assert!(price == 5, price);

        let price = price_for_domain_v1(15, 1) / config::octas();
        assert!(price == 5, price);

        let price = price_for_domain_v1(15, 10) / config::octas();
        assert!(price == 102, price);
    }

    #[test_only]
    struct YearPricePair has copy, drop {
        years: u8,
        expected_price: u64,
    }

    #[test(myself = @aptos_names, framework = @0x1)]
    fun test_scale_price_for_years(myself: &signer, framework: &signer) {
        use aptos_framework::account;
        use std::signer;
        use std::vector;
        // If the price is 100 APT, for 1 year, the price should be 100 APT, etc
        let prices_and_years = vector[
            YearPricePair { years: 1, expected_price: 100 },
            YearPricePair { years: 2, expected_price: 210 },
            YearPricePair { years: 3, expected_price: 335 },
            YearPricePair { years: 4, expected_price: 480 },
            YearPricePair { years: 5, expected_price: 650 },
            YearPricePair { years: 6, expected_price: 850 },
            YearPricePair { years: 7, expected_price: 1085 },
            YearPricePair { years: 8, expected_price: 1360 },
            YearPricePair { years: 9, expected_price: 1680 },
            YearPricePair { years: 10, expected_price: 2050 },
        ];

        account::create_account_for_test(signer::address_of(myself));
        account::create_account_for_test(signer::address_of(framework));

        while (vector::length(&prices_and_years) > 0) {
            let pair = vector::pop_back(&mut prices_and_years);
            let price = scale_price_for_years(100 * config::octas(), pair.years) / config::octas();
            assert!(price == pair.expected_price, price);
        };
    }
}
