module aptos_framework::validator_rewards {
    use std::error;
    use std::option::{Self, Option};
    use std::signer;
    use std::string;
    use std::vector;

    /// Errors
    const E_NOT_INITIALIZED: u64 = 1;
    const E_ALREADY_INITIALIZED: u64 = 2;
    const E_NOT_ADMIN: u64 = 3;
    const E_INVALID_DENOMINATOR: u64 = 4;
    const E_INVALID_EPOCHS_PER_YEAR: u64 = 5;
    const E_ZERO_TOTAL_PROPOSALS: u64 = 6;

    /// Events
    struct RewardRateUpdatedEvent has copy, drop, store {
        old_num: u64,
        old_den: u64,
        new_num: u64,
        new_den: u64,
        reason: string::String, // "manual" | "auto-decrease" | "init"
        epoch_effective: u64,   // the epoch at/after which the rate applies
    }

    struct Events has key {
        reward_rate_updated: vector<RewardRateUpdatedEvent>,
    }

    fun emit_reward_rate_updated(
      e: &mut Events, 
      old_num: u64, 
      old_den: u64, 
      new_num: u64, 
      new_den: u64, 
      reason: &string::String, 
      epoch_effective: u64
    ) {
        vector::push_back(
            &mut e.reward_rate_updated,
            RewardRateUpdatedEvent {
                old_num,
                old_den,
                new_num,
                new_den,
                reason: string::utf8(reason.bytes()),
                epoch_effective,
            },
        );
    }

    /// Configuration

    /// Global configuration resource for the reward system.
    ///
    /// Notes:
    /// - `rewards_rate_num` / `rewards_rate_den` represent the **per-epoch** rate fraction.
    /// - `annual_apr_bp` tracks the current **annual APR** in basis points (1% = 100 bp), used only if
    ///   automatic yearly decreases are enabled, to recompute the per-epoch numerator each year.
    /// - `denominator` is fixed to 1_000_000_000 by default for precision, but stored so it can be verified/read.
    /// - `epochs_per_year` should match your epoch config (e.g., 2h epochs -> ~4_380).
    /// - `periodical_decrease_enabled` toggles Auto Path (Path 2).
    /// - `decrease_bp_per_year` is how many **basis points** to reduce the annual APR each year (e.g., 25 = 0.25%).
    /// - `min_annual_apr_bp` is a floor to the annual APR in basis points.
    /// - `last_decrease_epoch` stores the epoch when auto-decrease was last applied.
    /// - `admin` is a fixed admin address for simplicity—wire this behind governance in production.
    struct Config has key {
        rewards_rate_num: u64,
        rewards_rate_den: u64, // typically 1_000_000_000
        denominator: u64,

        // Annual APR tracking (basis points)
        annual_apr_bp: u64,
        min_annual_apr_bp: u64,

        // Epoch / timing parameters
        epochs_per_year: u64,
        last_decrease_epoch: u64,

        // Auto-decrease controls
        periodical_decrease_enabled: bool,
        decrease_bp_per_year: u64,

        // lightweight admin model 
        admin: address,
    }

    /// Singletons
    struct Global has key { config: Option<Config>, events: Events }

    /// Denominator for per-epoch fixed-point rate.
    const DEFAULT_DENOMINATOR: u64 = 1_000_000_000;

    /// Example: with 2h epochs, ~4_380 per year.
    const DEFAULT_EPOCHS_PER_YEAR: u64 = 4_380;

    /// View APIs
    public fun is_initialized(): bool acquires Global {
        option::is_some(&borrow_global<Global>(@0x42).config)
    }

    /// Returns the current per-epoch reward rate (numerator, denominator).
    public fun get_reward_rate(): (u64, u64) acquires Global {
        let cfg = borrow_config();
        (cfg.rewards_rate_num, cfg.rewards_rate_den)
    }

    /// Returns the current annual APR in basis points (1% = 100 bp).
    public fun get_annual_apr_bp(): u64 acquires Global {
        let cfg = borrow_config();
        cfg.annual_apr_bp
    }

    /// Returns whether automatic yearly decrease is enabled.
    public fun periodical_reward_rate_decrease_enabled(): bool acquires Global {
        let cfg = borrow_config();
        cfg.periodical_decrease_enabled
    }

    /// Initialization (Genesis or first-time setup)
    ///
    /// Initialize from a target APR (percentage) and compute the per-epoch rate:
    ///     per_epoch_num = (target_apr_pct * denominator / 100) / epochs_per_year
    ///
    /// `target_apr_percentage_x100` is APR in hundredths of a percent (e.g. 500 = 5.00%).
    /// If you prefer whole percent, pass `target_apr_percentage_x100 = apr_pct * 100`.
    public entry fun initialize(
        admin: &signer,
        target_apr_percentage_x100: u64, // APR in 1/100 %, e.g. 625 => 6.25%
        epochs_per_year: u64,            // e.g. 4_380
        denominator: u64,                // typically 1_000_000_000
        auto_decrease_enabled: bool,     // Path 2 toggle
        decrease_bp_per_year: u64,       // e.g., 25 = 0.25% per year
        min_annual_apr_bp: u64           // floor APR in bp, e.g., 200 = 2.00%
    ) acquires Global {
        assert!(denominator > 0, error::invalid_argument(E_INVALID_DENOMINATOR));
        assert!(epochs_per_year > 0, error::invalid_argument(E_INVALID_EPOCHS_PER_YEAR));

        let admin_addr = signer::address_of(admin);

        if (!exists<Global>(@0x42)) {
            move_to(
                admin,
                Global {
                    config: option::none<Config>(),
                    events: Events { reward_rate_updated: vector::empty<RewardRateUpdatedEvent>() },
                }
            );
        };

        let g = borrow_global_mut<Global>(@0x42);
        assert!(!option::is_some(&g.config), error::already_exists(E_ALREADY_INITIALIZED));

        // Convert APR x100 to basis points: 1% = 100 bp, 1 bp = 0.01%
        // APR(x100) * 1 bp / (0.01%) => bp = x100
        let annual_apr_bp = target_apr_percentage_x100;

        // per_epoch_num = (APR% * den / 100) / epochs_per_year
        // APR% = annual_apr_bp / 100  (since bp = 1/100 %)
        // So: num = ((annual_apr_bp * den) / 100) / epochs_per_year
        let num_u128 = (((annual_apr_bp as u128) * (denominator as u128)) / 100u128) / (epochs_per_year as u128);
        let per_epoch_num = num_u128 as u64;

        g.config = option::some(Config {
            rewards_rate_num: per_epoch_num,
            rewards_rate_den: denominator,
            denominator,
            annual_apr_bp,
            min_annual_apr_bp,
            epochs_per_year,
            last_decrease_epoch: 0,
            periodical_decrease_enabled: auto_decrease_enabled,
            decrease_bp_per_year,
            admin: admin_addr,
        });

        emit_reward_rate_updated(&mut g.events, 0, 1, per_epoch_num, denominator, &string::utf8(b"init"), /*epoch_effective*/ 0);
    }

    /// Manual Governance Update (Path 1)
    ///
    /// Update the **per-epoch** rate directly (numerator/denominator).
    /// Gate this behind real governance in production (replace admin check).
    public entry fun update_rewards_rate(
        caller: &signer,
        new_rate_num: u64,
        new_rate_den: u64,
        epoch_effective: u64
    ) acquires Global {
        let cfg = borrow_config_mut();
        assert!(signer::address_of(caller) == cfg.admin, error::permission_denied(E_NOT_ADMIN));
        assert!(new_rate_den > 0, error::invalid_argument(E_INVALID_DENOMINATOR));

        let old_num = cfg.rewards_rate_num;
        let old_den = cfg.rewards_rate_den;
        cfg.rewards_rate_num = new_rate_num;
        cfg.rewards_rate_den = new_rate_den;

        // Keep annual APR in-sync for transparency: recompute APR (bp) from per-epoch rate.
        // annual_apr_bp ≈ per_epoch_rate * epochs_per_year * 100
        // per_epoch_rate = num/den
        // APR% = (num * epochs_per_year / den) * 100
        cfg.annual_apr_bp = recompute_annual_bp_from_epoch_rate(new_rate_num, new_rate_den, cfg.epochs_per_year);

        let g = borrow_global_mut<Global>(@0x42);
        emit_reward_rate_updated(&mut g.events, old_num, old_den, new_rate_num, new_rate_den, &string::utf8(b"manual"), epoch_effective);
    }

    /// Automatic Yearly Decrease (Path 2)
    ///
    /// If enabled, call this at (or after) epoch boundaries. When at least one "year"
    /// (in epochs) has elapsed since `last_decrease_epoch`, the annual APR (bp) is
    /// reduced by `decrease_bp_per_year`, floored at `min_annual_apr_bp`, and the
    /// per-epoch numerator is recomputed accordingly.
    public entry fun maybe_apply_periodical_decrease(
        caller: &signer,
        current_epoch: u64,
    ) acquires Global {
        let cfg = borrow_config_mut();
        assert!(signer::address_of(caller) == cfg.admin, error::permission_denied(E_NOT_ADMIN));

        if (!cfg.periodical_decrease_enabled) {
            return
        };

        if (current_epoch < cfg.last_decrease_epoch + cfg.epochs_per_year) {
            return
        };

        let old_apr_bp = cfg.annual_apr_bp;
        let new_apr_bp_unfloored = if (old_apr_bp > cfg.decrease_bp_per_year) {
            old_apr_bp - cfg.decrease_bp_per_year
        } else { 0 };

        let new_apr_bp = if (new_apr_bp_unfloored >= cfg.min_annual_apr_bp) {
            new_apr_bp_unfloored
        } else {
            cfg.min_annual_apr_bp
        };

        // If no effective change, do nothing.
        if (new_apr_bp == old_apr_bp) {
            cfg.last_decrease_epoch = current_epoch;
            return
        };

        // Recompute per-epoch numerator from APR(bp):
        // per_epoch_num = ((APR(bp) * den) / 100) / epochs_per_year
        let new_num_u128 = (((new_apr_bp as u128) * (cfg.denominator as u128)) / 100u128) / (cfg.epochs_per_year as u128);
        let new_num = new_num_u128 as u64;
        let old_num = cfg.rewards_rate_num;
        let old_den = cfg.rewards_rate_den;

        cfg.annual_apr_bp = new_apr_bp;
        cfg.rewards_rate_num = new_num;
        // keep denominator unchanged
        cfg.last_decrease_epoch = current_epoch;

        let g = borrow_global_mut<Global>(@0x42);
        emit_reward_rate_updated(&mut g.events, old_num, old_den, new_num, cfg.rewards_rate_den, &string::utf8(b"auto-decrease"), /*effective next epoch*/ current_epoch + 1);
    }

    /// Pure reward calculation (matches MIP-125 formula)
    ///
    /// rewards_amount =
    ///   (stake_amount * rewards_rate * num_successful_proposals)
    ///   / (rewards_rate_denominator * num_total_proposals)
    ///
    /// Notes:
    /// - Uses u128 intermediates for safety, returns u64 (saturates at u64::MAX).
    /// - If `num_total_proposals == 0`, aborts with E_ZERO_TOTAL_PROPOSALS.
    public fun calculate_rewards_amount(
        stake_amount: u64,
        num_successful_proposals: u64,
        num_total_proposals: u64,
    ): u64 acquires Global {
        assert!(num_total_proposals > 0, error::invalid_argument(E_ZERO_TOTAL_PROPOSALS));
        let (rate_num, rate_den) = get_reward_rate();

        let stake_u128 = stake_amount as u128;
        let num_succ_u128 = num_successful_proposals as u128;
        let num_total_u128 = num_total_proposals as u128;
        let rate_num_u128 = rate_num as u128;
        let rate_den_u128 = rate_den as u128;

        let numerator = stake_u128 * rate_num_u128 * num_succ_u128;
        let denominator = rate_den_u128 * num_total_u128;

        let reward_u128 = numerator / denominator;

        // clamp to u64::MAX
        if (reward_u128 > (u64::max_value() as u128)) {
            u64::max_value()
        } else {
            reward_u128 as u64
        }
    }

    /// Admin helpers
    public fun admin_address(): address acquires Global {
        borrow_config().admin
    }

    public entry fun set_admin(caller: &signer, new_admin: address) acquires Global {
        let cfg = borrow_config_mut();
        assert!(signer::address_of(caller) == cfg.admin, error::permission_denied(E_NOT_ADMIN));
        cfg.admin = new_admin;
    }

    /// Internal helpers
    fun recompute_annual_bp_from_epoch_rate(per_epoch_num: u64, den: u64, epochs_per_year: u64): u64 {
        // annual_apr_bp ≈ (per_epoch_num / den) * epochs_per_year * 100
        // Compute in u128 for headroom.
        let v = ((per_epoch_num as u128) * (epochs_per_year as u128) * 100u128) / (den as u128);
        v as u64
    }

    fun borrow_config(): &Config acquires Global {
        let g = borrow_global<Global>(@0x42);
        assert!(option::is_some(&g.config), error::not_found(E_NOT_INITIALIZED));
        option::borrow(&g.config)
    }

    fun borrow_config_mut(): &mut Config acquires Global {
        let g = borrow_global_mut<Global>(@0x42);
        assert!(option::is_some(&g.config), error::not_found(E_NOT_INITIALIZED));
        option::borrow_mut(&mut g.config)
    }
}
