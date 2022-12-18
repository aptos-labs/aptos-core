/// A work in progress NFT DAO Platform Example
/// Note: Each token Id can only vote once as NFT is globally unique. If you create a DAO allowing fungible token to vote, only 1 of the fungible token can vote
///
/// An example e2e flow.
/// There are multiple roles: DAO platform operator, DAO creator, proposer and voter.
/// 1. platform operator deploy this package to create a DAO platform
/// 2. DAO creator calls `create_dao` to create their DAO.
/// 3. A proposer can specify the DAO she wants to create a proposal and create the proposal through  `create_proposal`
///     two types of proposals can be created:
///     3.a: no-op, no automatic execution happens on chain. Only the proposal and its results are recorded on-chain for DAO admin to take actions
///     3.b: transfer fund, the fund will be automatically transferred from DAO resource account to the destination account.
/// 4. A voter can vote for a proposal of a DAO through `vote`
/// 5. Anyone can call the `resolve` to resolve a proposal. A proposal voting duration has to expire and the proposal should have more votes than the minimal required threshold.
///
/// An example of DAO delegation flow
///  1. the DAO creator can delegate her DAO through `delegate_dao` to another admin address
///
/// An example of DAO removal.
/// 1. The DAO creator can call `reclaim_signer_capability` to remove her DAO from the platform and get back her resource account's signercapability
///
/// Note: This current version stores all the signer_capability in the contract space. If the code upgrades, a potential risk is the platform operation can obtain the signer capability of the DAOs on this platform
/// It allows the operator to obtain the signer of these DAOs without any consent.
/// TODO: explore new tooling to dynamically compiling DAO modules on server side and store the signercapability in each DAO's own moudle to avoid exposing signer capability to platform
module dao_platform::nft_dao {
    use aptos_framework::account::{SignerCapability, create_signer_with_capability};
    use aptos_framework::account;
    use aptos_framework::aptos_account::transfer_coins;
    use aptos_framework::timestamp;
    use aptos_std::bucket_table::BucketTable;
    use aptos_std::bucket_table;
    use aptos_std::table::Table;
    use aptos_std::table;
    use aptos_token::property_map::PropertyMap;
    use aptos_token::property_map;
    use aptos_token::token::{Self, TokenId};
    use dao_platform::nft_dao_events::emit_create_dao_event;
    use std::bcs;
    use std::error;
    use std::signer;
    use std::string::String;
    use std::string;
    use std::vector;


    /// This account doesn't have enough voting power
    const EVOTING_POWER_NOT_ENOUGH: u64 = 1;

    /// This account doesn't own this DAO's voting token
    const ENOT_OWN_THE_VOTING_DAO_TOKEN: u64 = 2;

    /// This function is not supported in proposal
    const ENOT_SUPPROTED_FUNCTION: u64 = 3;

    /// Can only propose a start time in future
    const EPROPOSAL_START_TIME_SHOULD_BE_IN_FUTURE: u64 = 4;

    /// Invalid admin account
    const EINVALID_ADMIN_ACCOUNT: u64 = 5;

    /// String length exceeds limits
    const ESTRING_TOO_LONG: u64 = 6;

    /// Proposal ended and no more voting
    const EPROPOSAL_ENDED: u64 = 7;

    /// Proposal has not started voting
    const EPROPOSAL_NOT_STARTED: u64 = 8;

    /// Proposal has already been resolved
    const EPROPOSAL_RESOLVED: u64 = 9;

    /// Token already voted for the proposal
    const ETOKEN_ALREADY_VOTED: u64 = 10;

    /// DAO doesn't exist at this address
    const EDAO_NOT_EXIST: u64 = 11;

    /// Proposal resource not created
    const EPRPOSALS_NOT_EXIST_AT_ADDRESS: u64 = 12;

    /// Proposal with specified id doesn't exist
    const EPRPOSAL_ID_NOT_EXIST: u64 = 13;

    /// DAO already created at this address
    const EDAO_ADDRESS_USED: u64 = 14;

    /// Already an admin of the DAO
    const EALREADY_ADMIN_OF_THE_DAO: u64 = 15;

    /// Token already used for creating proposal
    const ETOKEN_USED_FOR_CREATING_PROPOSAL: u64 = 16;

    /// Constants
    const PROPOSAL_PENDING: u8 = 0;
    const PROPOSAL_RESOLVED_PASSED: u8 = 1;
    const PROPOSAL_RESOLVED_NOT_PASSED: u8 = 2;

    struct DAO has key {
        /// Name of the DAO
        name: String,
        /// The minimum number of total votes (both yes and no) a proposal must have in order to be considered valid. A proposal would still need more yes than no to pass.
        resolve_threshold: u64,
        /// The NFT Collection that is used to govern the DAO
        governance_token: GovernanceToken,
        /// The voting duration in secs
        voting_duration: u64,
        /// Minimum weight for proposal
        min_required_proposer_voting_power: u64,
        /// Proposal counter
        next_proposal_id: u64,
        /// DAO resource account signer capability
        dao_signer_capability: SignerCapability,
        /// DAO admin account
        admin: address,
    }

    /// This resource stores all the DAO resource addresses that an admin owns
    struct OwnedDAO has key {
        daos: vector<address>,
    }

    /// The collection should only contains NFTs, where all token name only has 1 maximal and immutable
    /// The total supply is fixed with the token names.
    struct GovernanceToken has drop, store {
        /// The creator address of this NFT collection
        creator: address,
        /// The collection name
        collection: String,
    }

    /// All proposals
    struct Proposals has key {
        proposals: Table<u64, Proposal>,
    }

    /// Store the general information about a proposal
    struct Proposal has copy, drop, store {
        /// Name of the proposal, limiting to 64 chars
        name: String,
        /// Description of the proposal, limiting to 512 chars
        description: String,
        /// The name of function to be executed
        function_name: String,
        /// The function arguments to be executed
        function_args: PropertyMap,
        /// The start time of the voting
        start_time_sec: u64,
        /// Proposal results, unresolved, passed, not passed
        resolution: u8,
        /// final voting count of yes votes
        final_yes_votes: u64,
        /// final voting count of no votes
        final_no_votes: u64,
    }

    struct ProposalVotingStatistics has key {
        proposals: Table<u64, VotingStatistics>,
    }

    struct VotingStatistics has store {
        /// Total yes votes
        total_yes: u64,
        /// Total no notes
        total_no: u64,
        /// Token voted yes
        yes_votes: BucketTable<TokenId, address>, // address is the original voter's address for keeping a record of who voted
        /// Token voted no
        no_votes: BucketTable<TokenId, address>,
    }

    /// Creator creates a DAO on the platform
    public entry fun create_dao(
        admin: &signer,
        name: String,
        threshold: u64,
        voting_duration: u64,
        voting_token_collection_creator: address, // this is the creator address of goverance token
        collection_name: String,
        min_required_proposer_voting_power: u64,
    ) acquires OwnedDAO {

        // create a resource account
        let seed = bcs::to_bytes(&name);
        vector::append(&mut seed, bcs::to_bytes(&voting_token_collection_creator));
        vector::append(&mut seed, bcs::to_bytes(&collection_name));

        let (res_signer, res_cap) = account::create_resource_account(admin, seed);
        let src_addr = signer::address_of(admin);

        assert!(string::length(&name) < 128, error::invalid_argument(ESTRING_TOO_LONG));

        move_to(
            &res_signer,
            DAO {
                name,
                resolve_threshold: threshold,
                governance_token: GovernanceToken { creator: voting_token_collection_creator, collection: collection_name },
                voting_duration,
                min_required_proposer_voting_power,
                next_proposal_id: 0,
                dao_signer_capability: res_cap,
                admin: src_addr,
            },
        );
        move_to(
            &res_signer,
            Proposals {
                proposals: table::new()
            }
        );

        move_to(
          &res_signer,
            ProposalVotingStatistics {
                proposals: table::new()
            }
        );

        let dao_addr = signer::address_of(&res_signer);
        if (!exists<OwnedDAO>(src_addr)) {
            let daos = vector::empty<address>();
            vector::push_back(&mut daos, dao_addr);
            move_to(
                admin,
                OwnedDAO {
                    daos,
                }
            );
        } else {
            let owned_address = borrow_global_mut<OwnedDAO>(src_addr);
            assert!(!vector::contains(&owned_address.daos, &dao_addr), error::already_exists(EDAO_ADDRESS_USED));
            vector::push_back(&mut owned_address.daos, dao_addr);
        };

        emit_create_dao_event(
            &res_signer,
            name,
            threshold,
            voting_duration,
            min_required_proposer_voting_power,
            voting_token_collection_creator,
            collection_name,
            signer::address_of(&res_signer),
        )
    }


    /// Only DAO Goverance token holders can create proposal
    public entry fun create_proposal(
        account: &signer,
        nft_dao: address, // resource account address of the nft dao
        name: String, // name of the proposal
        description: String, // description of the proposal
        function_name: String, // only two functions: (1) "no_op" and (2) "transfer_fund". Only transfer_fund has correponding function now.
        arg_names: vector<String>, // name of the arguments of the function to be called. The arg here should be the same as the argument used in the function
        arg_values: vector<vector<u8>>, // bcs serailized values of argument values
        arg_types:vector<String>, // types of arguments. currently, we only support string, u8, u64, u128, bool, address.
        start_time_sec: u64, // when the voting starts
        token_names: vector<String>, // The name of the token, the proposer want to use for proposing
        property_versions: vector<u64>, // the property versions of the corresponding tokens, the proposer want to use for proposing
    ) acquires DAO, Proposals {
        let dao = borrow_global_mut<DAO>(nft_dao);
        assert!(string::length(&name) <= 64, error::invalid_argument(ESTRING_TOO_LONG));
        assert!(string::length(&description) <= 512, error::invalid_argument(ESTRING_TOO_LONG));

        // verify the account's token has enough weights to create proposal
        let weights = get_proposal_weights(account, &token_names, &property_versions, dao);
        assert!(weights >= dao.min_required_proposer_voting_power, error::permission_denied(EVOTING_POWER_NOT_ENOUGH));

        // verify the parameters are legit
        let pm = property_map::new(arg_names, arg_values, arg_types);
        assert_function_valid(function_name, &pm);

        // verify the start_time is in future
        let now = timestamp::now_seconds();
        assert!(start_time_sec > now, error::invalid_argument(EPROPOSAL_START_TIME_SHOULD_BE_IN_FUTURE));

        let proposal = Proposal {
            name,
            description,
            function_name,
            function_args: pm,
            start_time_sec,
            resolution: PROPOSAL_PENDING,
            final_yes_votes: 0,
            final_no_votes: 0,
        };

        let proposal_store = borrow_global_mut<Proposals>(nft_dao);
        let proposal_id = dao.next_proposal_id + 1;
        table::add(&mut proposal_store.proposals, proposal_id, proposal);
        dao.next_proposal_id = proposal_id;
    }

    /// Vote with a batch of tokens
    public entry fun vote(
        account: &signer,
        nft_dao: address,
        proposal_id: u64,
        vote: bool,
        token_names: vector<String>,
        property_versions: vector<u64>,
    ) acquires DAO, ProposalVotingStatistics, Proposals {
        let dao = borrow_global_mut<DAO>(nft_dao);
        let gtoken = &dao.governance_token;
        let proposals = borrow_global<Proposals>(nft_dao);

        // assert the proposal hasn't ended, voter can can only vote for the proposal that starts and hasn't ended
        let proposal = table::borrow(&proposals.proposals, proposal_id);
        let now = timestamp::now_seconds();
        assert!(now < proposal.start_time_sec + dao.voting_duration, error::invalid_argument(EPROPOSAL_ENDED));
        assert!(now > proposal.start_time_sec, error::invalid_argument(EPROPOSAL_NOT_STARTED));

        let prop_stats = borrow_global_mut<ProposalVotingStatistics>(nft_dao);

        // initialize the voting statistics of the proposal
        if (!table::contains(&prop_stats.proposals, proposal_id)) {
            let vstat = VotingStatistics {
                total_yes: 0,
                total_no: 0,
                yes_votes: bucket_table::new(10),
                no_votes: bucket_table::new(10),
            };
            table::add(&mut prop_stats.proposals, proposal_id, vstat);
        };
        let stats = table::borrow_mut(&mut prop_stats.proposals, proposal_id);

        let voter_addr = signer::address_of(account);
        let i = 0;
        // loop through all NFTs used for voting and update the voting result
        while (i < vector::length(&token_names)) {
            let token_name = *vector::borrow(&token_names, i);
            let property_version = *vector::borrow(&property_versions, i);
            let token_id = token::create_token_id_raw(gtoken.creator, gtoken.collection, token_name, property_version);
            // check if this token already voted
            assert!(!bucket_table::contains(&stats.no_votes, &token_id), error::invalid_argument(ETOKEN_ALREADY_VOTED));
            assert!(!bucket_table::contains(&stats.yes_votes, &token_id), error::invalid_argument(ETOKEN_ALREADY_VOTED));

            // this account owns the token
            assert!(token::balance_of(signer::address_of(account), token_id) == 1, error::permission_denied(ENOT_OWN_THE_VOTING_DAO_TOKEN));
            if (vote) {
                stats.total_yes = stats.total_yes + 1;
                bucket_table::add(&mut stats.yes_votes, token_id, voter_addr);
            } else {
                stats.total_no = stats.total_no + 1;
                bucket_table::add(&mut stats.no_votes, token_id, voter_addr);
            };
            i = i + 1;
        };
    }

    /// Entry function that can be called by anyone
    public entry fun resolve<CoinType>(proposal_id: u64, nft_dao: address) acquires Proposals, DAO, ProposalVotingStatistics {
        // validate if proposal is ready to resolve
        let dao = borrow_global_mut<DAO>(nft_dao);

        // assert the proposal voting ended
        let proposals = borrow_global_mut<Proposals>(nft_dao);
        let proposal = table::borrow_mut(&mut proposals.proposals, proposal_id);
        let now = timestamp::now_seconds();
        assert!(now >= proposal.start_time_sec + dao.voting_duration, error::invalid_argument(EPROPOSAL_ENDED));

        // assert the proposal is unresolved yet
        assert!(proposal.resolution == PROPOSAL_PENDING, error::invalid_argument(EPROPOSAL_RESOLVED));

        let proposal_stat = &mut borrow_global_mut<ProposalVotingStatistics>(nft_dao).proposals;
        let voting_stat = table::borrow_mut(proposal_stat, proposal_id);
        proposal.final_yes_votes = voting_stat.total_yes;
        proposal.final_no_votes = voting_stat.total_no;
        // validate resolve threshold and result
        let voted = voting_stat.total_no + voting_stat.total_yes;
        if (voted < dao.resolve_threshold) {
            // not sufficient token voted
            proposal.resolution = PROPOSAL_RESOLVED_NOT_PASSED;
        } else if(voting_stat.total_yes > voting_stat.total_no) {
            let function_name = proposal.function_name;
            if (function_name == string::utf8(b"transfer_fund")) {
                let res_signer =  create_signer_with_capability(&dao.dao_signer_capability);
                let dst_addr = property_map::read_address(&proposal.function_args, &string::utf8(b"dst"));
                let amount = property_map::read_u64(&proposal.function_args, &string::utf8(b"amount"));
                transfer_fund<CoinType>(&res_signer, dst_addr, amount);
            } else {
               assert!(function_name == string::utf8(b"no_op"), error::invalid_argument(ENOT_SUPPROTED_FUNCTION));
            };
            proposal.resolution = PROPOSAL_RESOLVED_PASSED;
        } else {
            proposal.resolution = PROPOSAL_RESOLVED_NOT_PASSED;
        };
    }

    /// DAO creator can quit the platform and claim back his resource account signer capability
    public fun destory_dao_and_reclaim_signer_capability(account: &signer, dao: address): SignerCapability acquires DAO {
        let addr = signer::address_of(account);
        assert!(exists<DAO>(dao), error::not_found(EDAO_NOT_EXIST));
        let dao_config = borrow_global_mut<DAO>(dao);
        assert!(dao_config.admin == addr, error::permission_denied(EINVALID_ADMIN_ACCOUNT));
        let DAO {
            name: _,
            resolve_threshold: _,
            governance_token: _,
            voting_duration: _,
            min_required_proposer_voting_power: _,
            next_proposal_id: _,
            dao_signer_capability,
            admin: _
        } = move_from<DAO>(dao);
       dao_signer_capability
    }

    #[view]
    /// Get the proposal
    public fun get_proposal(proposal_id: u64, nft_dao: address): Proposal acquires Proposals {
        assert!(exists<Proposals>(nft_dao), error::not_found(EPRPOSALS_NOT_EXIST_AT_ADDRESS));
        let proposals = &borrow_global<Proposals>(nft_dao).proposals;
        assert!(table::contains(proposals, proposal_id), error::not_found(EPRPOSAL_ID_NOT_EXIST));
        *table::borrow(proposals, proposal_id)
    }

    #[view]
    /// Get the proposal resolution result
    public fun get_proposal_resolution(proposal_id: u64, nft_dao: address): u8 acquires Proposals {
        let proposal = get_proposal(proposal_id, nft_dao);
        proposal.resolution
    }

    /// Unpack the proposal fields
    public fun unpack_proposal(proposal: &Proposal): (String, String, String, PropertyMap, u64, u8) {
        (
            proposal.name,
            proposal.description,
            proposal.function_name,
            proposal.function_args,
            proposal.start_time_sec,
            proposal.resolution,
        )
    }

    #[view]
    /// Unpack the DAO fields
    public fun unpack_dao(nft_dao: address): (String, u64, address, String, u64, u64, u64) acquires DAO {
        let dao = borrow_global<DAO>(nft_dao);
        (
            dao.name,
            dao.resolve_threshold,
            dao.governance_token.creator,
            dao.governance_token.collection,
            dao.voting_duration,
            dao.min_required_proposer_voting_power,
            dao.next_proposal_id,
        )
    }

    #[view]
    /// Get owned DAOs of an admin
    public fun get_owned_daos(account: address): vector<address> acquires OwnedDAO {
        assert!(exists<OwnedDAO>(account), error::not_found(EDAO_NOT_EXIST));
        let owned_dao = borrow_global<OwnedDAO>(account);
        owned_dao.daos
    }

    // transfer coin from the DAO account to the destination account
    fun transfer_fund<CoinType>(res_acct: &signer, dst: address, amount: u64) {
        transfer_coins<CoinType>(res_acct, dst, amount);
    }

    fun get_proposal_weights(
        account: &signer,
        token_names: &vector<String>,
        property_versions: &vector<u64>,
        dao: &DAO
    ): u64 {
        let gtoken = &dao.governance_token;
        let i = 0;
        let total_weight = 0;
        let used_token_ids = vector::empty();

        while (i < vector::length(token_names)) {
            let token_name = *vector::borrow(token_names, i);
            let property_version = *vector::borrow(property_versions, i);
            let token_id = token::create_token_id_raw(gtoken.creator, gtoken.collection, token_name, property_version);
            assert!(!vector::contains(&used_token_ids, &token_id), error::already_exists(ETOKEN_USED_FOR_CREATING_PROPOSAL));
            vector::push_back(&mut used_token_ids, token_id);
            assert!(token::balance_of(signer::address_of(account), token_id) == 1, error::permission_denied(ENOT_OWN_THE_VOTING_DAO_TOKEN));
            total_weight = total_weight + 1;
            i = i + 1;
        };
        total_weight
    }

    fun assert_function_valid(function_name: String, map: &PropertyMap){
        if (function_name == string::utf8(b"transfer_fund")) {
            assert!(property_map::length(map) == 2, error::invalid_argument(ENOT_SUPPROTED_FUNCTION));
            property_map::read_address(map, &string::utf8(b"dst"));
            property_map::read_u64(map, &string::utf8(b"amount"));
        } else if (function_name == string::utf8(b"no_op")) {
            assert!(property_map::length(map) == 0, error::invalid_argument(ENOT_SUPPROTED_FUNCTION));
        } else {
            abort error::invalid_argument(ENOT_SUPPROTED_FUNCTION)
        }
    }

    #[test_only]
    public fun create_one_token(
        creator: &signer,
        collection_name: String,
        token_name: String,
        amount: u64,
        token_max: u64
    ): TokenId {
        create_token_script(
            creator,
            collection_name,
            token_name,
            string::utf8(b"Hello, Token"),
            amount,
            token_max,
            string::utf8(b"https://aptos.dev"),
            signer::address_of(creator),
            100,
            0,
            vector<bool>[false, false, false, false, false],
            vector::empty(),
            vector::empty(),
            vector::empty(),
        );
        token::create_token_id_raw(signer::address_of(creator), collection_name, token_name, 0)
    }

    #[test_only]
    use aptos_token::token::create_token_script;
    #[test_only]
    use aptos_framework::coin::FakeMoney;
    #[test_only]
    use aptos_framework::coin;

    #[test(aptos_framework = @0x1, creator = @0xdeaf, voter = @0xaf)]
    public fun test_e2e_scenario(aptos_framework: &signer, creator: &signer, voter: &signer) acquires DAO, Proposals, ProposalVotingStatistics, OwnedDAO {
        timestamp::set_time_has_started_for_testing(aptos_framework);
        account::create_account_for_test(@0x1);
        account::create_account_for_test(@0xdeaf);
        account::create_account_for_test(@0xaf);

        // create an NFT collection
        token::create_collection_and_token(
            creator,
            1,
            3,
            1,
            vector::empty(),
           vector::empty(),
            vector::empty(),
            vector<bool>[false, false, false],
            vector<bool>[false, false, false, false, false],
        );

        let token_id_2 = create_one_token(
            creator,
            string::utf8(b"Hello, World"),
            string::utf8(b"artist2"),
            1,
            1,
        );
        let token_id_3 = create_one_token(
            creator,
            string::utf8(b"Hello, World"),
            string::utf8(b"artist3"),
            1,
            1,
        );
        token::direct_transfer(creator, voter, token_id_2, 1);
        token::direct_transfer(creator, voter, token_id_3, 1);

        // creator creates a dao
        let creator_addr = signer::address_of(creator);
        create_dao(
            creator,
            string::utf8(b"my_dao"),
            1,
            10,
            creator_addr,
            string::utf8(b"Hello, World"),
            1,
        );

        let owned_dao = get_owned_daos(creator_addr);
        let res_acc = vector::pop_back(&mut owned_dao);

        //
        // Test no_op proposal
        //

        // creator creates a proposal
        create_proposal(
            creator,
            res_acc, // resource account address of the nft dao
            string::utf8(b"Proposal 1"),
            string::utf8(b"description"),
            string::utf8(b"no_op"),
            vector<String>[],
            vector<vector<u8>>[],
            vector<String>[],
            1,
            vector<String>[string::utf8(b"Token")],
            vector<u64>[0],
        );
        timestamp::update_global_time_for_test(2000000);

        vote(
            voter,
            res_acc,
            1,
            true,
            vector<String>[string::utf8(b"artist2"), string::utf8(b"artist3")],
            vector<u64>[0, 0],
        );
        timestamp::update_global_time_for_test(20000000);
        resolve<FakeMoney>(1, res_acc);
        assert!(get_proposal_resolution(1, res_acc) == PROPOSAL_RESOLVED_PASSED, 1);

        //
        // Test transfer fund proposal
        //

        // intialize with some fund in the DAO resource account
        coin::create_fake_money(aptos_framework, creator, 100);

        coin::transfer<coin::FakeMoney>(aptos_framework, signer::address_of(creator), 100);
        // now resource account has a fund pool of 90 coins
        transfer_coins<coin::FakeMoney>(creator, res_acc, 90);

        // creator a proposal to transfer 45 coins to voter's account
        create_proposal(
            creator,
            res_acc, // resource account address of the nft dao
            string::utf8(b"Proposal 2"),
            string::utf8(b"description"),
            string::utf8(b"transfer_fund"),
            vector<String>[string::utf8(b"dst"), string::utf8(b"amount")],
            vector<vector<u8>>[bcs::to_bytes(&@0xaf), bcs::to_bytes(&(45 as u64))],
            vector<String>[string::utf8(b"address"), string::utf8(b"u64")],
            21,
            vector<String>[string::utf8(b"Token")],
            vector<u64>[0],
        );
        timestamp::update_global_time_for_test(22000000);
        vote(
            voter,
            res_acc,
            2,
            true,
            vector<String>[string::utf8(b"artist2"), string::utf8(b"artist3")],
            vector<u64>[0, 0],
        );
        timestamp::update_global_time_for_test(40000000);
        resolve<FakeMoney>(2, res_acc);
        assert!(get_proposal_resolution(1, res_acc) == PROPOSAL_RESOLVED_PASSED, 1);
        // vote get 45 coin transferred to her account after resolving
        assert!(coin::balance<FakeMoney>(signer::address_of(voter)) == 45, 1);
    }
}
