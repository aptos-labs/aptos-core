module aptos_framework::dkg_rounding {
    use std::option;
    use std::option::Option;
    use std::vector;
    use aptos_std::fixed_point64;
    use aptos_std::fixed_point64::FixedPoint64;
    #[test_only]
    use std::string::utf8;
    #[test_only]
    use aptos_std::debug;
    #[test_only]
    use aptos_framework::randomness;

    friend aptos_framework::reconfiguration_with_dkg;

    struct RoundingResult has drop, store {
        ideal_total_weight: u128,
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

    /// When an async reconfig starts,
    /// compute weights + threshold for the next validator set and store it on chain.
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
    /// Discard the rounding for the current epoch, mark the rounding for the next epoch as "current".
    public(friend) fun on_new_epoch(framework: &signer) acquires CurEpochRounding, NextEpochRounding {
        if (exists<CurEpochRounding>(@aptos_framework)) {
            move_from<CurEpochRounding>(@aptos_framework);
        };
        if (exists<NextEpochRounding>(@aptos_framework)) {
            let NextEpochRounding { rounding} = move_from<NextEpochRounding>(@aptos_framework);
            move_to(framework, CurEpochRounding { rounding })
        }
    }

    fun rounding(
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
        rounding_internal(
            stakes,
            fixed_point64::get_raw_value(secrecy_threshold_in_stake_ratio),
            fixed_point64::get_raw_value(reconstruct_threshold_in_stake_ratio),
            fast_secrecy_thresh_raw,
        )
    }

    native fun rounding_internal(
        stakes: vector<u64>,
        secrecy_thresh_raw: u128,
        recon_thresh_raw: u128,
        fast_secrecy_thresh_raw: u128,
    ): RoundingResult;

    #[test_only]
    struct Diff has drop {
        vid: u64,
        stake: u64,
        weight_0: u64,
        weight_1: u64,
    }

    #[test_only]
    fun random_stake_dist(): vector<u64> {
        let n = randomness::u64_range(10, 200);

        let ret = vector[];
        while (n > 0) {
            let stake = if (randomness::u64_range(0, 2) == 0) {
                randomness::u64_range(100000000000000, 1000000000000000)
            } else {
                randomness::u64_range(1000000000000000, 10000000000000000)
            };
            // let stake = randomness::u64_range(10, 100);
            vector::push_back(&mut ret, stake);
            n = n - 1;
        };
        ret
    }

    #[test_only]
    fun membership_arr(n: u64, subset_idx: u64): vector<u64> {
        let ret = vector[];
        while (n > 0) {
            vector::push_back(&mut ret, subset_idx % 2);
            subset_idx = subset_idx / 2;
            n = n - 1;
        };
        ret
    }

    #[test_only]
    fun subsum(values: &vector<u64>, subset: &vector<u64>): u128 {
        let ret = 0;
        vector::zip_ref(values, subset, |value, flag|{
            ret = ret + ((*value * *flag) as u128);
        });
        ret
    }

    #[test(framework = @0x1)]
    fun test_correctness(framework: signer) {
        randomness::initialize_for_testing(&framework);
        let attempts = 100;
        while (attempts > 0) {
            let n = randomness::u64_range(1, 7);
            let stake_total: u128 = 0;
            let stakes = vector::map(vector::range(0, n), |i|{
                let stake = randomness::u64_range(1000000, 20000000);
                stake_total = stake_total + (stake as u128);
                stake
            });
            let secrecy_thresh_pct = 50;
            let secrecy_thresh = fixed_point64::create_from_rational(secrecy_thresh_pct, 100);
            let recon_thresh_pct = 66;
            let recon_thresh = fixed_point64::create_from_rational(recon_thresh_pct, 100);
            let fast_secrecy_thresh_pct = 67;
            let fast_secrecy_thresh = fixed_point64::create_from_rational(fast_secrecy_thresh_pct, 100);
            let has_fast_secrect_thresh = 1;
            let maybe_fast_secrecy_thresh = if (has_fast_secrect_thresh == 1) {
                option::some(fast_secrecy_thresh)
            } else {
                option::none()
            };
            debug::print(&utf8(b"stakes="));
            debug::print(&stakes);

            let rounding_result = rounding(stakes, secrecy_thresh, recon_thresh, maybe_fast_secrecy_thresh);
            debug::print(&utf8(b"rounding_result="));
            debug::print(&rounding_result);
            let computed_fast_threshold = if (has_fast_secrect_thresh == 1) {
                option::extract(&mut rounding_result.reconstruct_threshold_fast_path)
            } else {
                0
            };

            let subset = 0;
            let num_subsets = 1 << (n as u8);
            while (subset < num_subsets) {
                let memberships = membership_arr(n, subset);
                let stake_subtotal = subsum(&stakes, &memberships);
                let stake_frac = fixed_point64::create_from_rational(stake_subtotal, stake_total);
                let weight_subtotal = subsum(&rounding_result.weights, &memberships);
                let secrecy_failure = fixed_point64::less_or_equal(stake_frac, secrecy_thresh) && weight_subtotal >= rounding_result.reconstruct_threshold_default_path;
                let recon_default_failure = fixed_point64::greater(stake_frac, recon_thresh) && weight_subtotal < rounding_result.reconstruct_threshold_default_path;
                let fast_secrecy_failure = has_fast_secrect_thresh == 1 && fixed_point64::less_or_equal(stake_frac, fast_secrecy_thresh) && weight_subtotal >= computed_fast_threshold;
                if (secrecy_failure || recon_default_failure || fast_secrecy_failure) {
                    debug::print(&memberships);
                    debug::print(&secrecy_failure);
                    debug::print(&recon_default_failure);
                    debug::print(&fast_secrecy_failure);
                    abort(1)
                };
                subset = subset + 1;
            };

            attempts = attempts - 1;
        }
    }

    #[test]
    fun test_mainnet_dist() {
        let stakes = mainnet_stakes();
        let secrecy_thre = fixed_point64::create_from_rational(1, 2);
        let recon_thre = fixed_point64::create_from_rational(2, 3);
        let fast_recon_thre = option::some(fixed_point64::create_from_rational(67, 100));
        let result = rounding(stakes, secrecy_thre, recon_thre, fast_recon_thre);
        debug::print(&result);
    }

    #[test(framework = @0x1)]
    fun compare(framework: signer) {
        randomness::initialize_for_testing(&framework);
        let stake_distributions = vector[];
        let n = 10;
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
                let diff = Diff {
                    vid: i,
                    stake: *vector::borrow(&stakes, i),
                    weight_0: *vector::borrow(&result_0.weights, i),
                    weight_1: *vector::borrow(&result_1.weights, i),
                };
                if (diff.weight_0 != diff.weight_1) {
                    debug::print(&diff);
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

    #[test_only]
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

    #[test_only]
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
