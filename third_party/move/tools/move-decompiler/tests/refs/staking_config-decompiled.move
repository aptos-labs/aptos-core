module 0x1::staking_config {
    struct StakingConfig has copy, drop, key {
        minimum_stake: u64,
        maximum_stake: u64,
        recurring_lockup_duration_secs: u64,
        allow_validator_set_change: bool,
        rewards_rate: u64,
        rewards_rate_denominator: u64,
        voting_power_increase_limit: u64,
    }
    
    struct StakingRewardsConfig has copy, drop, key {
        rewards_rate: 0x1::fixed_point64::FixedPoint64,
        min_rewards_rate: 0x1::fixed_point64::FixedPoint64,
        rewards_rate_period_in_secs: u64,
        last_rewards_rate_period_start_in_secs: u64,
        rewards_rate_decrease_rate: 0x1::fixed_point64::FixedPoint64,
    }
    
    public(friend) fun calculate_and_save_latest_epoch_rewards_rate() : 0x1::fixed_point64::FixedPoint64 acquires StakingRewardsConfig {
        assert!(0x1::features::periodical_reward_rate_decrease_enabled(), 0x1::error::invalid_state(11));
        let v0 = calculate_and_save_latest_rewards_config();
        v0.rewards_rate
    }
    
    fun calculate_and_save_latest_rewards_config() : StakingRewardsConfig acquires StakingRewardsConfig {
        let v0 = borrow_global_mut<StakingRewardsConfig>(@0x1);
        let v1 = 0x1::timestamp::now_seconds();
        assert!(v1 >= v0.last_rewards_rate_period_start_in_secs, 0x1::error::invalid_argument(7));
        if (v1 - v0.last_rewards_rate_period_start_in_secs < v0.rewards_rate_period_in_secs) {
            return *v0
        };
        let v2 = 0x1::fixed_point64::ceil(v0.rewards_rate_decrease_rate) <= 1;
        assert!(v2, 0x1::error::invalid_argument(8));
        let v3 = 0x1::fixed_point64::sub(0x1::fixed_point64::create_from_u128(1), v0.rewards_rate_decrease_rate);
        let v4 = 0x1::math_fixed64::mul_div(v0.rewards_rate, v3, 0x1::fixed_point64::create_from_u128(1));
        v0.rewards_rate = 0x1::fixed_point64::max(v4, v0.min_rewards_rate);
        let v5 = v0.last_rewards_rate_period_start_in_secs + v0.rewards_rate_period_in_secs;
        v0.last_rewards_rate_period_start_in_secs = v5;
        *v0
    }
    
    public fun get() : StakingConfig acquires StakingConfig {
        *borrow_global<StakingConfig>(@0x1)
    }
    
    public fun get_allow_validator_set_change(arg0: &StakingConfig) : bool {
        arg0.allow_validator_set_change
    }
    
    public fun get_recurring_lockup_duration(arg0: &StakingConfig) : u64 {
        arg0.recurring_lockup_duration_secs
    }
    
    public fun get_required_stake(arg0: &StakingConfig) : (u64, u64) {
        (arg0.minimum_stake, arg0.maximum_stake)
    }
    
    public fun get_reward_rate(arg0: &StakingConfig) : (u64, u64) acquires StakingRewardsConfig {
        if (0x1::features::periodical_reward_rate_decrease_enabled()) {
            let v2 = borrow_global<StakingRewardsConfig>(@0x1).rewards_rate;
            let (v3, v4) = if (0x1::fixed_point64::is_zero(v2)) {
                (0, 1)
            } else {
                let v5 = 0x1::fixed_point64::divide_u128(1000000 as u128, v2);
                let v6 = v5;
                if (v5 > 18446744073709551615) {
                    v6 = 18446744073709551615;
                };
                (0x1::fixed_point64::multiply_u128(v6, v2) as u64, v6 as u64)
            };
            (v3, v4)
        } else {
            (arg0.rewards_rate, arg0.rewards_rate_denominator)
        }
    }
    
    public fun get_voting_power_increase_limit(arg0: &StakingConfig) : u64 {
        arg0.voting_power_increase_limit
    }
    
    public(friend) fun initialize(arg0: &signer, arg1: u64, arg2: u64, arg3: u64, arg4: bool, arg5: u64, arg6: u64, arg7: u64) {
        0x1::system_addresses::assert_aptos_framework(arg0);
        validate_required_stake(arg1, arg2);
        assert!(arg3 > 0, 0x1::error::invalid_argument(1));
        assert!(arg6 > 0, 0x1::error::invalid_argument(2));
        assert!(arg7 > 0 && arg7 <= 50, 0x1::error::invalid_argument(4));
        assert!(arg5 <= 1000000, 0x1::error::invalid_argument(5));
        assert!(arg5 <= arg6, 0x1::error::invalid_argument(5));
        let v0 = StakingConfig{
            minimum_stake                  : arg1, 
            maximum_stake                  : arg2, 
            recurring_lockup_duration_secs : arg3, 
            allow_validator_set_change     : arg4, 
            rewards_rate                   : arg5, 
            rewards_rate_denominator       : arg6, 
            voting_power_increase_limit    : arg7,
        };
        move_to<StakingConfig>(arg0, v0);
    }
    
    public fun initialize_rewards(arg0: &signer, arg1: 0x1::fixed_point64::FixedPoint64, arg2: 0x1::fixed_point64::FixedPoint64, arg3: u64, arg4: u64, arg5: 0x1::fixed_point64::FixedPoint64) {
        0x1::system_addresses::assert_aptos_framework(arg0);
        validate_rewards_config(arg1, arg2, arg3, arg5);
        assert!(0x1::timestamp::now_seconds() >= arg4, 0x1::error::invalid_argument(7));
        let v0 = StakingRewardsConfig{
            rewards_rate                           : arg1, 
            min_rewards_rate                       : arg2, 
            rewards_rate_period_in_secs            : arg3, 
            last_rewards_rate_period_start_in_secs : arg4, 
            rewards_rate_decrease_rate             : arg5,
        };
        move_to<StakingRewardsConfig>(arg0, v0);
    }
    
    public fun update_recurring_lockup_duration_secs(arg0: &signer, arg1: u64) acquires StakingConfig {
        assert!(arg1 > 0, 0x1::error::invalid_argument(1));
        0x1::system_addresses::assert_aptos_framework(arg0);
        borrow_global_mut<StakingConfig>(@0x1).recurring_lockup_duration_secs = arg1;
    }
    
    public fun update_required_stake(arg0: &signer, arg1: u64, arg2: u64) acquires StakingConfig {
        0x1::system_addresses::assert_aptos_framework(arg0);
        validate_required_stake(arg1, arg2);
        let v0 = borrow_global_mut<StakingConfig>(@0x1);
        v0.minimum_stake = arg1;
        v0.maximum_stake = arg2;
    }
    
    public fun update_rewards_config(arg0: &signer, arg1: 0x1::fixed_point64::FixedPoint64, arg2: 0x1::fixed_point64::FixedPoint64, arg3: u64, arg4: 0x1::fixed_point64::FixedPoint64) acquires StakingRewardsConfig {
        0x1::system_addresses::assert_aptos_framework(arg0);
        validate_rewards_config(arg1, arg2, arg3, arg4);
        let v0 = borrow_global_mut<StakingRewardsConfig>(@0x1);
        assert!(arg3 == v0.rewards_rate_period_in_secs, 0x1::error::invalid_argument(9));
        v0.rewards_rate = arg1;
        v0.min_rewards_rate = arg2;
        v0.rewards_rate_period_in_secs = arg3;
        v0.rewards_rate_decrease_rate = arg4;
    }
    
    public fun update_rewards_rate(arg0: &signer, arg1: u64, arg2: u64) acquires StakingConfig {
        assert!(!0x1::features::periodical_reward_rate_decrease_enabled(), 0x1::error::invalid_state(10));
        0x1::system_addresses::assert_aptos_framework(arg0);
        assert!(arg2 > 0, 0x1::error::invalid_argument(2));
        assert!(arg1 <= 1000000, 0x1::error::invalid_argument(5));
        assert!(arg1 <= arg2, 0x1::error::invalid_argument(5));
        let v0 = borrow_global_mut<StakingConfig>(@0x1);
        v0.rewards_rate = arg1;
        v0.rewards_rate_denominator = arg2;
    }
    
    public fun update_voting_power_increase_limit(arg0: &signer, arg1: u64) acquires StakingConfig {
        0x1::system_addresses::assert_aptos_framework(arg0);
        assert!(arg1 > 0 && arg1 <= 50, 0x1::error::invalid_argument(4));
        borrow_global_mut<StakingConfig>(@0x1).voting_power_increase_limit = arg1;
    }
    
    fun validate_required_stake(arg0: u64, arg1: u64) {
        assert!(arg0 <= arg1 && arg1 > 0, 0x1::error::invalid_argument(3));
    }
    
    fun validate_rewards_config(arg0: 0x1::fixed_point64::FixedPoint64, arg1: 0x1::fixed_point64::FixedPoint64, arg2: u64, arg3: 0x1::fixed_point64::FixedPoint64) {
        let v0 = 0x1::fixed_point64::less_or_equal(arg0, 0x1::fixed_point64::create_from_u128(1));
        assert!(v0, 0x1::error::invalid_argument(5));
        assert!(0x1::fixed_point64::less_or_equal(arg1, arg0), 0x1::error::invalid_argument(6));
        assert!(0x1::fixed_point64::ceil(arg3) <= 1, 0x1::error::invalid_argument(8));
        assert!(arg2 > 0, 0x1::error::invalid_argument(9));
    }
    
    // decompiled from Move bytecode v6
}
