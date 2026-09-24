// Ported from https://github.com/aave/aptos-aave-v3, modules
// `aave_config::reserve_config` and `aave_config::user_config`.
// Copyright (c) Aave DAO and contributors.
// SPDX-License-Identifier: Apache-2.0

/// Bit-packed reserve and user configuration.
///
/// A reserve's parameters live in a single `u256` so that one storage slot
/// answers every question the hot paths ask. A user's positions live in a
/// second `u256` holding two bits per reserve: the even bit (2*i) marks
/// borrowing, the odd bit (2*i + 1) marks use as collateral.
module bench::aave_config {
    use bench::aave_math;

    /// Each mask clears its own field and keeps every other bit, so a setter
    /// is an `and` followed by an `or`.
    const LTV_MASK: u256 =
        0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF0000;
    const LIQUIDATION_THRESHOLD_MASK: u256 =
        0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF0000FFFF;
    const LIQUIDATION_BONUS_MASK: u256 =
        0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF0000FFFFFFFF;
    const DECIMALS_MASK: u256 =
        0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF00FFFFFFFFFFFF;
    const ACTIVE_MASK: u256 =
        0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEFFFFFFFFFFFFFF;
    const FROZEN_MASK: u256 =
        0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFDFFFFFFFFFFFFFF;
    const BORROWING_MASK: u256 =
        0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFBFFFFFFFFFFFFFF;
    /// Bit 59 is an unoccupied hole left by the removed stable-rate flag.
    const PAUSED_MASK: u256 =
        0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEFFFFFFFFFFFFFFF;
    const FLASHLOAN_ENABLED_MASK: u256 =
        0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF7FFFFFFFFFFFFFFF;
    const RESERVE_FACTOR_MASK: u256 =
        0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF0000FFFFFFFFFFFFFFFF;
    const BORROW_CAP_MASK: u256 =
        0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF000000000FFFFFFFFFFFFFFFFFFFF;
    const SUPPLY_CAP_MASK: u256 =
        0xFFFFFFFFFFFFFFFFFFFFFFFFFF000000000FFFFFFFFFFFFFFFFFFFFFFFFFFFFF;
    const LIQUIDATION_PROTOCOL_FEE_MASK: u256 =
        0xFFFFFFFFFFFFFFFFFFFFFF0000FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF;

    const LIQUIDATION_THRESHOLD_START_BIT_POSITION: u8 = 16;
    const LIQUIDATION_BONUS_START_BIT_POSITION: u8 = 32;
    const RESERVE_DECIMALS_START_BIT_POSITION: u8 = 48;
    const IS_ACTIVE_START_BIT_POSITION: u8 = 56;
    const IS_FROZEN_START_BIT_POSITION: u8 = 57;
    const BORROWING_ENABLED_START_BIT_POSITION: u8 = 58;
    const IS_PAUSED_START_BIT_POSITION: u8 = 60;
    const FLASHLOAN_ENABLED_START_BIT_POSITION: u8 = 63;
    const RESERVE_FACTOR_START_BIT_POSITION: u8 = 64;
    const BORROW_CAP_START_BIT_POSITION: u8 = 80;
    const SUPPLY_CAP_START_BIT_POSITION: u8 = 116;
    const LIQUIDATION_PROTOCOL_FEE_START_BIT_POSITION: u8 = 152;

    const MAX_VALID_LTV: u256 = 65535;
    const MAX_VALID_LIQUIDATION_THRESHOLD: u256 = 65535;
    const MAX_VALID_LIQUIDATION_BONUS: u256 = 65535;
    const MAX_VALID_DECIMALS: u256 = 18;
    const MAX_VALID_RESERVE_FACTOR: u256 = 65535;
    const MAX_VALID_BORROW_CAP: u256 = 68719476735;
    const MAX_VALID_SUPPLY_CAP: u256 = 68719476735;
    const MAX_VALID_LIQUIDATION_PROTOCOL_FEE: u256 = 65535;
    const MIN_RESERVE_ASSET_DECIMALS: u256 = 6;

    /// Two bits per reserve in a `u256` caps the protocol at 128 reserves.
    const MAX_RESERVES_COUNT: u256 = 128;

    const USER_BORROWING_MASK: u256 =
        0x5555555555555555555555555555555555555555555555555555555555555555;
    const USER_COLLATERAL_MASK: u256 =
        0xAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA;

    const HEALTH_FACTOR_LIQUIDATION_THRESHOLD: u256 = 1_000_000_000_000_000_000;
    const INTEREST_RATE_MODE_NONE: u8 = 0;
    const INTEREST_RATE_MODE_VARIABLE: u8 = 2;

    const EINVALID_LTV: u64 = 10;
    const EINVALID_LIQ_THRESHOLD: u64 = 11;
    const EINVALID_LIQ_BONUS: u64 = 12;
    const EINVALID_DECIMALS: u64 = 13;
    const EINVALID_RESERVE_FACTOR: u64 = 14;
    const EINVALID_BORROW_CAP: u64 = 15;
    const EINVALID_SUPPLY_CAP: u64 = 16;
    const EINVALID_LIQ_PROTOCOL_FEE: u64 = 17;
    const EINVALID_RESERVE_INDEX: u64 = 18;

    struct ReserveConfigurationMap has copy, store, drop {
        data: u256
    }

    struct UserConfigurationMap has copy, store, drop {
        data: u256
    }

    public fun max_reserves_count(): u256 {
        MAX_RESERVES_COUNT
    }

    public fun health_factor_liquidation_threshold(): u256 {
        HEALTH_FACTOR_LIQUIDATION_THRESHOLD
    }

    public fun interest_rate_mode_none(): u8 {
        INTEREST_RATE_MODE_NONE
    }

    public fun interest_rate_mode_variable(): u8 {
        INTEREST_RATE_MODE_VARIABLE
    }

    public fun user_borrowing_mask(): u256 {
        USER_BORROWING_MASK
    }

    public fun user_collateral_mask(): u256 {
        USER_COLLATERAL_MASK
    }

    public fun init_reserve_configuration(): ReserveConfigurationMap {
        ReserveConfigurationMap { data: 0 }
    }

    public fun reserve_configuration_data(self: &ReserveConfigurationMap): u256 {
        self.data
    }

    public fun reserve_configuration_from_data(data: u256): ReserveConfigurationMap {
        ReserveConfigurationMap { data }
    }

    public fun set_ltv(self: &mut ReserveConfigurationMap, ltv: u256) {
        assert!(ltv <= MAX_VALID_LTV, EINVALID_LTV);
        self.data = (self.data & LTV_MASK) | ltv;
    }

    public fun get_ltv(self: &ReserveConfigurationMap): u256 {
        self.data & aave_math::bitwise_negation(LTV_MASK)
    }

    public fun set_liquidation_threshold(
        self: &mut ReserveConfigurationMap, threshold: u256
    ) {
        assert!(threshold <= MAX_VALID_LIQUIDATION_THRESHOLD, EINVALID_LIQ_THRESHOLD);
        self.data = (self.data & LIQUIDATION_THRESHOLD_MASK)
            | (threshold << LIQUIDATION_THRESHOLD_START_BIT_POSITION);
    }

    public fun get_liquidation_threshold(self: &ReserveConfigurationMap): u256 {
        (self.data & aave_math::bitwise_negation(LIQUIDATION_THRESHOLD_MASK))
            >> LIQUIDATION_THRESHOLD_START_BIT_POSITION
    }

    public fun set_liquidation_bonus(
        self: &mut ReserveConfigurationMap, bonus: u256
    ) {
        assert!(bonus <= MAX_VALID_LIQUIDATION_BONUS, EINVALID_LIQ_BONUS);
        self.data = (self.data & LIQUIDATION_BONUS_MASK)
            | (bonus << LIQUIDATION_BONUS_START_BIT_POSITION);
    }

    public fun get_liquidation_bonus(self: &ReserveConfigurationMap): u256 {
        (self.data & aave_math::bitwise_negation(LIQUIDATION_BONUS_MASK))
            >> LIQUIDATION_BONUS_START_BIT_POSITION
    }

    public fun set_decimals(self: &mut ReserveConfigurationMap, decimals: u256) {
        assert!(
            decimals >= MIN_RESERVE_ASSET_DECIMALS && decimals <= MAX_VALID_DECIMALS,
            EINVALID_DECIMALS
        );
        self.data = (self.data & DECIMALS_MASK)
            | (decimals << RESERVE_DECIMALS_START_BIT_POSITION);
    }

    public fun get_decimals(self: &ReserveConfigurationMap): u256 {
        (self.data & aave_math::bitwise_negation(DECIMALS_MASK))
            >> RESERVE_DECIMALS_START_BIT_POSITION
    }

    public fun set_active(self: &mut ReserveConfigurationMap, active: bool) {
        self.data = (self.data & ACTIVE_MASK)
            | ((if (active) { 1 } else { 0 }) << IS_ACTIVE_START_BIT_POSITION);
    }

    public fun get_active(self: &ReserveConfigurationMap): bool {
        (self.data & aave_math::bitwise_negation(ACTIVE_MASK)) != 0
    }

    public fun set_frozen(self: &mut ReserveConfigurationMap, frozen: bool) {
        self.data = (self.data & FROZEN_MASK)
            | ((if (frozen) { 1 } else { 0 }) << IS_FROZEN_START_BIT_POSITION);
    }

    public fun get_frozen(self: &ReserveConfigurationMap): bool {
        (self.data & aave_math::bitwise_negation(FROZEN_MASK)) != 0
    }

    public fun set_borrowing_enabled(
        self: &mut ReserveConfigurationMap, enabled: bool
    ) {
        self.data = (self.data & BORROWING_MASK)
            | ((if (enabled) { 1 } else { 0 }) << BORROWING_ENABLED_START_BIT_POSITION);
    }

    public fun get_borrowing_enabled(self: &ReserveConfigurationMap): bool {
        (self.data & aave_math::bitwise_negation(BORROWING_MASK)) != 0
    }

    public fun set_paused(self: &mut ReserveConfigurationMap, paused: bool) {
        self.data = (self.data & PAUSED_MASK)
            | ((if (paused) { 1 } else { 0 }) << IS_PAUSED_START_BIT_POSITION);
    }

    public fun get_paused(self: &ReserveConfigurationMap): bool {
        (self.data & aave_math::bitwise_negation(PAUSED_MASK)) != 0
    }

    public fun set_flashloan_enabled(
        self: &mut ReserveConfigurationMap, enabled: bool
    ) {
        self.data = (self.data & FLASHLOAN_ENABLED_MASK)
            | ((if (enabled) { 1 } else { 0 }) << FLASHLOAN_ENABLED_START_BIT_POSITION);
    }

    public fun get_flashloan_enabled(self: &ReserveConfigurationMap): bool {
        (self.data & aave_math::bitwise_negation(FLASHLOAN_ENABLED_MASK)) != 0
    }

    public fun set_reserve_factor(
        self: &mut ReserveConfigurationMap, reserve_factor: u256
    ) {
        assert!(reserve_factor <= MAX_VALID_RESERVE_FACTOR, EINVALID_RESERVE_FACTOR);
        self.data = (self.data & RESERVE_FACTOR_MASK)
            | (reserve_factor << RESERVE_FACTOR_START_BIT_POSITION);
    }

    public fun get_reserve_factor(self: &ReserveConfigurationMap): u256 {
        (self.data & aave_math::bitwise_negation(RESERVE_FACTOR_MASK))
            >> RESERVE_FACTOR_START_BIT_POSITION
    }

    public fun set_borrow_cap(self: &mut ReserveConfigurationMap, cap: u256) {
        assert!(cap <= MAX_VALID_BORROW_CAP, EINVALID_BORROW_CAP);
        self.data = (self.data & BORROW_CAP_MASK)
            | (cap << BORROW_CAP_START_BIT_POSITION);
    }

    public fun get_borrow_cap(self: &ReserveConfigurationMap): u256 {
        (self.data & aave_math::bitwise_negation(BORROW_CAP_MASK))
            >> BORROW_CAP_START_BIT_POSITION
    }

    public fun set_supply_cap(self: &mut ReserveConfigurationMap, cap: u256) {
        assert!(cap <= MAX_VALID_SUPPLY_CAP, EINVALID_SUPPLY_CAP);
        self.data = (self.data & SUPPLY_CAP_MASK)
            | (cap << SUPPLY_CAP_START_BIT_POSITION);
    }

    public fun get_supply_cap(self: &ReserveConfigurationMap): u256 {
        (self.data & aave_math::bitwise_negation(SUPPLY_CAP_MASK))
            >> SUPPLY_CAP_START_BIT_POSITION
    }

    public fun set_liquidation_protocol_fee(
        self: &mut ReserveConfigurationMap, fee: u256
    ) {
        assert!(fee <= MAX_VALID_LIQUIDATION_PROTOCOL_FEE, EINVALID_LIQ_PROTOCOL_FEE);
        self.data = (self.data & LIQUIDATION_PROTOCOL_FEE_MASK)
            | (fee << LIQUIDATION_PROTOCOL_FEE_START_BIT_POSITION);
    }

    public fun get_liquidation_protocol_fee(self: &ReserveConfigurationMap): u256 {
        (self.data & aave_math::bitwise_negation(LIQUIDATION_PROTOCOL_FEE_MASK))
            >> LIQUIDATION_PROTOCOL_FEE_START_BIT_POSITION
    }

    /// One read serves every hot-path branch, so the flags are unpacked
    /// together rather than one call per bit.
    public fun get_flags(self: &ReserveConfigurationMap): (bool, bool, bool, bool, bool) {
        (
            get_active(self),
            get_frozen(self),
            get_borrowing_enabled(self),
            get_paused(self),
            get_flashloan_enabled(self)
        )
    }

    public fun get_params(
        self: &ReserveConfigurationMap
    ): (u256, u256, u256, u256, u256) {
        (
            get_ltv(self),
            get_liquidation_threshold(self),
            get_liquidation_bonus(self),
            get_decimals(self),
            get_reserve_factor(self)
        )
    }

    public fun init_user_configuration(): UserConfigurationMap {
        UserConfigurationMap { data: 0 }
    }

    public fun user_configuration_data(self: &UserConfigurationMap): u256 {
        self.data
    }

    public fun user_configuration_from_data(data: u256): UserConfigurationMap {
        UserConfigurationMap { data }
    }

    public fun set_borrowing(
        self: &mut UserConfigurationMap, reserve_index: u256, borrowing: bool
    ) {
        assert!(reserve_index < MAX_RESERVES_COUNT, EINVALID_RESERVE_INDEX);
        let bit = (1 as u256) << ((reserve_index << 1) as u8);
        if (borrowing) {
            self.data = self.data | bit;
        } else {
            self.data = self.data & aave_math::bitwise_negation(bit);
        }
    }

    public fun set_using_as_collateral(
        self: &mut UserConfigurationMap, reserve_index: u256, using: bool
    ) {
        assert!(reserve_index < MAX_RESERVES_COUNT, EINVALID_RESERVE_INDEX);
        let bit = (1 as u256) << (((reserve_index << 1) + 1) as u8);
        if (using) {
            self.data = self.data | bit;
        } else {
            self.data = self.data & aave_math::bitwise_negation(bit);
        }
    }

    public fun is_borrowing(
        self: &UserConfigurationMap, reserve_index: u256
    ): bool {
        assert!(reserve_index < MAX_RESERVES_COUNT, EINVALID_RESERVE_INDEX);
        (self.data >> ((reserve_index << 1) as u8)) & 1 != 0
    }

    public fun is_using_as_collateral(
        self: &UserConfigurationMap, reserve_index: u256
    ): bool {
        assert!(reserve_index < MAX_RESERVES_COUNT, EINVALID_RESERVE_INDEX);
        (self.data >> (((reserve_index << 1) + 1) as u8)) & 1 != 0
    }

    public fun is_using_as_collateral_or_borrowing(
        self: &UserConfigurationMap, reserve_index: u256
    ): bool {
        assert!(reserve_index < MAX_RESERVES_COUNT, EINVALID_RESERVE_INDEX);
        (self.data >> ((reserve_index << 1) as u8)) & 3 != 0
    }

    public fun is_borrowing_any(self: &UserConfigurationMap): bool {
        self.data & USER_BORROWING_MASK != 0
    }

    public fun is_using_as_collateral_any(self: &UserConfigurationMap): bool {
        self.data & USER_COLLATERAL_MASK != 0
    }

    /// A power of two has a single bit set, which `n & (n - 1) == 0` detects.
    public fun is_using_as_collateral_one(self: &UserConfigurationMap): bool {
        let masked = self.data & USER_COLLATERAL_MASK;
        masked != 0 && (masked & (masked - 1) == 0)
    }

    public fun is_borrowing_one(self: &UserConfigurationMap): bool {
        let masked = self.data & USER_BORROWING_MASK;
        masked != 0 && (masked & (masked - 1) == 0)
    }

    public fun is_empty(self: &UserConfigurationMap): bool {
        self.data == 0
    }
}
