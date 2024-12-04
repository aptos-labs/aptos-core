module aptos_framework::dkg_rounding {
    use std::option;
    use std::option::Option;
    use std::vector;
    use aptos_std::fixed_point64;
    use aptos_std::fixed_point64::FixedPoint64;
    use aptos_std::arbitrary_precision;
    #[test_only]
    use std::string::utf8;
    #[test_only]
    use aptos_std::debug;
    #[test_only]
    use aptos_framework::randomness;

    friend aptos_framework::reconfiguration_with_dkg;

    const ROUNDING_METHOD_BINARY_SEARCH: u64 = 1;
    const ROUNDING_METHOD_INFALLIBLE: u64 = 2;

    struct WeightConfig has drop {
        weights: vector<u64>,
        reconsutruct_threshold: u128,
    }

    struct RoundingResult has drop, store {
        ideal_total_weight: u64,
        weights: vector<u64>,
        reconstruct_threshold_default_path: u128,
        reconstruct_threshold_fast_path: Option<u128>,
    }

    struct CurEpochRounding has drop, key {
        rounding: RoundingResult,
    }

    struct NextEpochRounding has drop, key {
        rounding: RoundingResult,
    }

    const E_FATAL: u64 = 9999;

    fun default_threshold_info(): ReconstructThresholdInfo {
        ReconstructThresholdInfo {
            in_weights: 0,
            in_stakes: 0,
        }
    }

    fun default_profile(): Profile {
        Profile {
            ideal_total_weight: 0,
            validator_weights: vector[],
            threshold_default_path: default_threshold_info(),
            threshold_fast_path: option::none(),
        }
    }

    /// Invoked when an async reconfig starts.
    /// Compute rounding for the next epoch and store it on chain.
    public(friend) fun on_reconfig_start(
        framework: &signer,
        stakes: vector<u64>,
        secrecy_threshold_in_stake_ratio: FixedPoint64,
        reconstruct_threshold_in_stake_ratio: FixedPoint64,
        fast_secrecy_threshold_in_stake_ratio: Option<FixedPoint64>,
    ) acquires NextEpochRounding {
        let rounding = rounding(stakes, secrecy_threshold_in_stake_ratio, reconstruct_threshold_in_stake_ratio, fast_secrecy_threshold_in_stake_ratio);
        if (exists<NextEpochRounding>(@aptos_framework)) {
            move_from<NextEpochRounding>(@aptos_framework);
        };
        move_to(framework, NextEpochRounding { rounding });
    }

    /// Invoked when an async reconfig finishes.
    /// Update discard the rounding for the current epoch, mark the rounding for the next epoch as "current".
    public(friend) fun on_new_epoch(framework: &signer) acquires CurEpochRounding, NextEpochRounding {
        if (exists<CurEpochRounding>(@aptos_framework)) {
            move_from<CurEpochRounding>(@aptos_framework);
        };
        if (exists<NextEpochRounding>(@aptos_framework)) {
            let NextEpochRounding { rounding} = move_from<NextEpochRounding>(@aptos_framework);
            move_to(framework, CurEpochRounding { rounding })
        }
    }

    /// Given a stake distribution, compute a weight distribution.
    ///
    fun rounding(
        stakes: vector<u64>,
        secrecy_threshold_in_stake_ratio: FixedPoint64,
        reconstruct_threshold_in_stake_ratio: FixedPoint64,
        fast_secrecy_threshold_in_stake_ratio: Option<FixedPoint64>,
    ): RoundingResult {
        let epsilon = fixed_point64::create_from_raw_value(1);
        let n = vector::length(&stakes);

        // Ensure reconstruct_threshold > secrecy_threshold
        reconstruct_threshold_in_stake_ratio = fixed_point64::max(
            reconstruct_threshold_in_stake_ratio,
            fixed_point64::add(secrecy_threshold_in_stake_ratio, epsilon)
        );

        let secrecy_threshold_in_stake_ratio = arbitrary_precision::from_fixed_point64(secrecy_threshold_in_stake_ratio);
        let reconstruct_threshold_in_stake_ratio = arbitrary_precision::from_fixed_point64(reconstruct_threshold_in_stake_ratio);

        let total_weight_max = arbitrary_precision::div_ceil(
            arbitrary_precision::sum(vector[arbitrary_precision::from_u64(n), arbitrary_precision::from_u64(4)]),
            arbitrary_precision::product(vector[
                arbitrary_precision::sub(reconstruct_threshold_in_stake_ratio, secrecy_threshold_in_stake_ratio),
                arbitrary_precision::from_u64(2),
            ]),
        );
        let stakes_total = 0;
        vector::for_each(stakes, |stake|{
            stakes_total = stakes_total + (stake as u128);
        });
        let stakes_total = arbitrary_precision::from_u128(stakes_total);

        let bar = arbitrary_precision::as_u128(
            arbitrary_precision::ceil(
                arbitrary_precision::product(vector[stakes_total, reconstruct_threshold_in_stake_ratio])));
        let fast_secrecy_threshold_in_stake_ratio = option::map(fast_secrecy_threshold_in_stake_ratio, |v|arbitrary_precision::from_fixed_point64(v));

        let profile = default_profile();
        let lo = 0;
        let hi = arbitrary_precision::as_u128(total_weight_max) * 2;
        // while (lo + 1 < hi) {
        while (true) {
            let md = lo + 1;
            let weight_per_stake = arbitrary_precision::shift_down_by_bit(
                arbitrary_precision::div_ceil(
                    arbitrary_precision::shift_up_by_bit(arbitrary_precision::from_u128(md), 64),
                    stakes_total,
                ),
                64,
            );
            let cur_profile = compute_profile(secrecy_threshold_in_stake_ratio, fast_secrecy_threshold_in_stake_ratio, stakes, (md as u64), weight_per_stake);

            if (cur_profile.threshold_default_path.in_stakes <= bar) {
                // hi = md;
                profile = cur_profile;
                break;
            } else {
                lo = md;
            };
        };

        let Profile {
            ideal_total_weight,
            validator_weights,
            threshold_default_path,
            threshold_fast_path,
        } = profile;

        RoundingResult {
            ideal_total_weight,
            weights: validator_weights,
            reconstruct_threshold_default_path: threshold_default_path.in_weights,
            reconstruct_threshold_fast_path: option::map(threshold_fast_path, |t|{let t: ReconstructThresholdInfo = t; t.in_weights}),
        }
    }

    const BINARY_SEARCH_ERR_1: u64 = 0;
    const BINARY_SEARCH_ERR_2: u64 = 0;
    const BINARY_SEARCH_ERR_3: u64 = 0;
    const BINARY_SEARCH_ERR_4: u64 = 0;
    const BINARY_SEARCH_ERR_5: u64 = 0;

    struct ReconstructThresholdInfo has drop {
        in_weights: u128,
        in_stakes: u128,
    }

    struct Profile has drop {
        ideal_total_weight: u64,
        /// weight is a u64 because we assume `weight_per_stake <= 1` and validator stake is a u64.
        validator_weights: vector<u64>,
        threshold_default_path: ReconstructThresholdInfo,
        threshold_fast_path: Option<ReconstructThresholdInfo>,
    }

    ///
    /// Now, a validator subset of stake ratio `r` has `weight_sub_total` in range `[stake_total * r * weight_per_stake - delta_down, stake_total * r * weight_per_stake + delta_up]`.
    /// Therefore,
    /// - the threshold in weight has to be set to `1 + floor(secrecy_threshold_in_stake_ratio * stake_total * weight_per_stake + delta_up)`;
    /// - the stake ratio required for liveness is `secrecy_threshold_in_stake_ratio + (1 + delta_down + delta_up) / (take_total * weight_per_stake)`.
    /// Note that as `weight_per_stake` increases, the `stake_ratio_required_for_liveness` decreases.
    /// Further, when `weight_per_stake >= (n + 2) / (2 * stake_total * (reconstruct_threshold_in_stake_ratio - secrecy_threshold_in_stake_ratio))`,
    /// it is guaranteed that `stake_ratio_required_for_liveness <= reconstruct_threshold_in_stake_ratio`.
    fun compute_profile(
        secrecy_threshold_in_stake_ratio: arbitrary_precision::Number,
        secrecy_threshold_in_stake_ratio_fast_path: Option<arbitrary_precision::Number>,
        stakes: vector<u64>,
        ideal_total_weight: u64,
        weight_per_stake: arbitrary_precision::Number,
    ): Profile {
        let one = arbitrary_precision::from_u64(1);
        arbitrary_precision::min_assign(&mut weight_per_stake, one);

        // Initialize accumulators.
        let validator_weights = vector[];
        let delta_down = arbitrary_precision::from_u64(0);
        let delta_up = arbitrary_precision::from_u64(0);
        let weight_total = 0;
        let stake_total = 0;

        // Assign weights.
        vector::for_each(stakes, |stake|{
            let stake: u64 = stake;
            stake_total = stake_total + (stake as u128);
            let ideal_weight = weight_per_stake;
            arbitrary_precision::mul_u64_assign(&mut ideal_weight, stake);
            let rounded_weight = arbitrary_precision::round(ideal_weight, one);
            let rounded_weight_u64 = arbitrary_precision::as_u64(rounded_weight);
            vector::push_back(&mut validator_weights, rounded_weight_u64);
            weight_total = weight_total + (rounded_weight_u64 as u128);
            if (arbitrary_precision::greater_than(&ideal_weight, &rounded_weight)) {
                arbitrary_precision::add_assign(&mut delta_down, arbitrary_precision::sub(ideal_weight, rounded_weight));
            } else {
                arbitrary_precision::add_assign(&mut delta_up, arbitrary_precision::sub(rounded_weight, ideal_weight));
            };
        });

        // Compute default path thresholds.
        let threshold_default_path = compute_threshold(
            secrecy_threshold_in_stake_ratio,
            weight_per_stake,
            stake_total,
            weight_total,
            delta_up,
            delta_down,
        );

        let threshold_fast_path = option::map(secrecy_threshold_in_stake_ratio_fast_path, |t|{
            let t: arbitrary_precision::Number = t;
            compute_threshold(
                t,
                weight_per_stake,
                stake_total,
                weight_total,
                delta_up,
                delta_down,
            )
        });

        Profile {
            ideal_total_weight,
            validator_weights,
            threshold_default_path,
            threshold_fast_path,
        }
    }

    /// Once a **weight assignment** with `weight_per_stake` is done and `(weight_total, delta_up, delta_down)` are available,
    /// return the minimum reconstruct threshold that satisfies a `secrecy_threshold_in_stake_ratio`.
    fun compute_threshold(
        secrecy_threshold_in_stake_ratio: arbitrary_precision::Number,
        weight_per_stake: arbitrary_precision::Number,
        stake_total: u128,
        weight_total: u128,
        delta_up: arbitrary_precision::Number,
        delta_down: arbitrary_precision::Number,
    ): ReconstructThresholdInfo {
        let reconstruct_threshold_in_weights = arbitrary_precision::sum(vector[
            arbitrary_precision::product(vector[
                secrecy_threshold_in_stake_ratio,
                arbitrary_precision::from_u128(stake_total),
                weight_per_stake,
            ]),
            delta_up
        ]);
        arbitrary_precision::floor_assign(&mut reconstruct_threshold_in_weights);
        arbitrary_precision::add_assign(&mut reconstruct_threshold_in_weights, arbitrary_precision::from_u64(1));
        arbitrary_precision::min_assign(&mut reconstruct_threshold_in_weights, arbitrary_precision::from_u128(weight_total));

        let reconstruct_threshold_in_stakes = arbitrary_precision::div_ceil(
            arbitrary_precision::sum(vector[reconstruct_threshold_in_weights, delta_down]),
            weight_per_stake,
        );

        ReconstructThresholdInfo {
            in_stakes: arbitrary_precision::as_u128(reconstruct_threshold_in_stakes),
            in_weights: arbitrary_precision::as_u128(reconstruct_threshold_in_weights),
        }
    }

    struct Obj has drop {
        vid: u64,
        stake: u64,
        weight_0: u64,
        weight_1: u64,
    }

    #[test_only]
    fun random_stake_dist(): vector<u64> {
        let n = randomness::u64_range(3, 5);

        let ret = vector[];
        while (n > 0) {
            // let stake = if (randomness::u64_range(0, 2) == 0) {
            //     randomness::u64_range(100000000000000, 1000000000000000)
            // } else {
            //     randomness::u64_range(1000000000000000, 10000000000000000)
            // };
            let stake = randomness::u64_range(10, 100);
            vector::push_back(&mut ret, stake);
            n = n - 1;
        };
        ret
    }

    #[test(framework = @0x1)]
    fun test_mainnet_dist(framework: signer) {
        let stakes = mainnet_stakes();
        let secrecy_thre = fixed_point64::create_from_rational(1, 2);
        let recon_thre = fixed_point64::create_from_rational(2, 3);
        let fast_recon_thre = option::some(fixed_point64::create_from_rational(67, 100));
        let result = rounding(stakes, secrecy_thre, recon_thre, fast_recon_thre);
        debug::print(&result);
    }

    #[test(framework = @0x1)]
    fun rounding_test(framework: signer) {
        randomness::initialize_for_testing(&framework);
        let stake_distributions = vector[mainnet_stakes()];
        let n = 20;
        while (n > 0) {
            vector::push_back(&mut stake_distributions, random_stake_dist());
            n = n - 1;
        };

        vector::for_each(stake_distributions, |stakes| {
            debug::print(&utf8(b"target dist:"));
            debug::print(&stakes);
            let stakes: vector<u64> = stakes;
            let secrecy_thre = fixed_point64::create_from_rational(1, 2);
            let recon_thre = fixed_point64::create_from_rational(2, 3);
            let fast_recon_thre = option::some(fixed_point64::create_from_rational(67, 100));
            let result_1 = rounding(stakes, secrecy_thre, recon_thre, fast_recon_thre);
            let result_0 = rounding_v0(stakes, secrecy_thre, recon_thre, fast_recon_thre);
            let n = vector::length(&stakes);
            let i = 0;
            while (i < n) {
                let obj = Obj {
                    vid: i,
                    stake: *vector::borrow(&stakes, i),
                    weight_0: *vector::borrow(&result_0.weights, i),
                    weight_1: *vector::borrow(&result_1.weights, i),
                };
                if (obj.weight_0 != obj.weight_1) {
                    debug::print(&obj);
                };
                i = i + 1;
            };
            debug::print(&utf8(b">>>>>>>>>>>>>>>>>>>>>>>>>>>>>"));
            debug::print(&result_0.weights);
            debug::print(&get_total_weight(&result_0));
            debug::print(&result_0.ideal_total_weight);
            debug::print(&result_0.reconstruct_threshold_default_path);
            debug::print(&utf8(b"-----------------------------"));
            debug::print(&result_1.weights);
            debug::print(&get_total_weight(&result_1));
            debug::print(&result_1.ideal_total_weight);
            debug::print(&result_1.reconstruct_threshold_default_path);
            debug::print(&utf8(b"<<<<<<<<<<<<<<<<<<<<<<<<<<<<<"));
        });
    }

    fun rounding_v0(
        stakes: vector<u64>,
        secrecy_threshold_in_stake_ratio: FixedPoint64,
        reconstruct_threshold_in_stake_ratio: FixedPoint64,
        fast_secrecy_threshold_in_stake_ratio: Option<FixedPoint64>,
    ): RoundingResult {
        let fast_secrecy_thresh_raw = if (option::is_some(&fast_secrecy_threshold_in_stake_ratio)) {
            fixed_point64::get_raw_value(option::extract(&mut fast_secrecy_threshold_in_stake_ratio))
        } else {
            0
        };

        rounding_v0_internal(
            stakes,
            fixed_point64::get_raw_value(secrecy_threshold_in_stake_ratio),
            fixed_point64::get_raw_value(reconstruct_threshold_in_stake_ratio),
            fast_secrecy_thresh_raw,
        )
    }

    native fun rounding_v0_internal(
        stakes: vector<u64>,
        secrecy_thresh_raw: u128,
        recon_thresh_raw: u128,
        fast_secrecy_thresh_raw: u128,
    ): RoundingResult;

    #[test_only]
    fun mainnet_stakes(): vector<u64> {
        vector[176832132154307, 112941018684246, 412839080413554, 115172808675727, 115168017684180, 261855278242133, 1306077536709339, 1304399354594757, 813545320925069, 112637405176585, 112976487177606, 112631374510249, 112613045591143, 112664939182153, 112640915566243, 112668858392334, 112627002729329, 112726824559855, 112666687553259, 112791534661254, 115081318040034, 112967502656476, 112669214737233, 918265028889185, 603875463242711, 112645337392025, 112678715179123, 112633623834513, 771764168286602, 112616177608541, 112818970234372, 112961980587291, 988190452044313, 988446362167623, 771112373476773, 835830484962878, 382158341963245, 111614042281855, 112620551669122, 113003150307150, 190833083081060, 190857578912405, 190835321643184, 156705025472325, 1098156955072175, 112639617276140, 573807182709621, 112685485082760, 988100831440958, 112663878696387, 673154971287928, 944771956376450, 879487352523780, 815438508213548, 112636121363416, 990713661687165, 944572694660715, 784528838200622, 846329015067961, 819068677429213, 143223871620678, 1007292100775561, 569746645211370, 1139493260580945, 803718941835964, 1054455593520328, 333992425062532, 1446781308905131, 216600455865535, 1002652975596097, 920146329621772, 889709292422217, 884998996262055, 991903420719855, 992018134146776, 991366076801130, 992179025152789, 1681803589825156, 254182324842514, 439762615098321, 247816203225617, 1283074703914405, 405158399660351, 297717615440086, 887375428578225, 886772176485600, 886854042842821, 503272928493557, 886667781444005, 2723514450847744, 1001138216026583, 1188340621617105, 1055627249062932, 1500837505338946, 1500837505338946, 395756328950312, 247989472643615, 1059109793792755, 1002226898602334, 1002226753534966, 607779513191997, 679935710814246, 460291229643298, 314316730630270, 311996242456823, 1046898309718555, 1106169392018273, 1593686103796066, 326006966802254, 1047546802691151, 285536655339537, 415449434699879, 626297826825348, 1002653053768477, 650054469165460, 414085693436900, 413575481470645, 504310784154238, 1002037955052385, 495908991188468, 103557826097000, 310409646279256, 309377088941727, 115673541500963, 1028055003688482, 103345107003928, 142143479657409, 102640365597007, 1030244342137458, 1006883044362638, 204105612063907, 1002226853519821, 506512358553167, 190696800743521, 228421626059318, 101670189314425, 206207830773724, 201855304874370, 909017065039797, 240392331046134, 135958453138771, 200972882669207, 302080220385906, 502273225829506, 601464747728199, 1179733156025346, 100023208554737]
    }

    fun get_total_weight(result: &RoundingResult): u128 {
        let ret = 0;
        vector::for_each(result.weights, |weight|{
            ret = ret + (weight as u128);
        });
        ret
    }
}
