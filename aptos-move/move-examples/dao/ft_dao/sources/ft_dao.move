/// A work in progress FT DAO Platform Example
/// An example e2e flow is
/// DAO platform operator deploy this module to create a fungible token DAO platform
/// DAO creator calls `create_dao` to create FT DAO
/// proposer calls `create_proposal` to create a proposal
/// vote calls `vote` to vote on a proposal of a DAO, which will lock their token during proposal phase
/// Anyone calls `resolve` to resolve a proposal
/// After proposal ends and attempated resolve, voter can call `claim_token` to get their locked token back
///
/// DAO creator can also delegate the DAO to another address through `delegate_dao`
///
/// DAO creator can quit the platform through calling `reclaim_signer_capability` and get their signercapability back

module dao_platform::ft_dao {
    use aptos_framework::account::{SignerCapability, create_signer_with_capability, get_signer_capability_address};
    use aptos_framework::account;
    use aptos_framework::aptos_coin::AptosCoin;
    use aptos_framework::coin;
    use aptos_framework::timestamp;
    use aptos_std::table::Table;
    use aptos_std::table;
    use aptos_token::property_map::PropertyMap;
    use aptos_token::property_map;
    use aptos_token::token::{Self, Token};
    use dao_platform::ft_dao_events::emit_create_dao_event;
    use std::bcs;
    use std::error;
    use std::signer;
    use std::string::String;
    use std::string;
    use std::vector;



    /// The new address to be delegated to already exists in the platform
    const EDELEGATED_ADDRESS_ALREADY_EXISTS: u64 = 1;

    /// This account doesn't have enough voting weights
    const EVOTING_WEIGHTS_NOT_ENOUGH: u64 = 2;

    /// This account doesn't own this DAO's voting token
    const ENOT_OWN_ENOUGH_VOTING_DAO_TOKEN: u64 = 3;

    /// This function is supported in proposal
    const ENOT_SUPPROTED_FUNCTION: u64 = 4;

    /// Can only propose a start time in future
    const EPROPOSAL_START_TIME_SHOULD_IN_FUTURE: u64 = 5;

    /// Cannot only use membership token of the DAO
    const ENOT_FOUND_ADMIN_ACCOUNT: u64 = 6;

    /// String length exceeds limits
    const ESTRING_TOO_LONG: u64 = 7;

    /// Proposal ended
    const EPROPOSAL_ENDED: u64 = 8;

    /// Propsoal already resolved
    const EPROPOSAL_RESOLVED: u64 = 9;


    /// TOKEN VOTED FOR PROPOSAL
    const ETOKEN_ALREADY_VOTED: u64 = 10;

    /// Resource Account Doesn't exist
    const ERESOURCE_ACCT_NOT_EXIST: u64 = 11;

    /// Proposal resource not created
    const EPRPOSALS_NOT_EXIST_AT_ADDRESS: u64 = 12;

    /// Proposal id doesn't exist
    const EPRPOSAL_ID_NOT_EXIST: u64 = 13;

    /// Not find locked token
    const ENO_LOCKED_TOKEN_FOUND: u64 = 14;

    /// cannot withdraw token without valid resolve state
    const ECANNOT_WITHDRAW_BEFORE_PROPOSAL_RESOLVED: u64 = 14;




    /// Constants
    const PROPOSAL_UNRESOLVED: u8 = 0;
    const PROPOSAL_RESOLVED_PASSED: u8 = 1;
    const PROPOSAL_RESOLVED_NOT_PASSED: u8 = 2;

    const PROPERTY_VERSION: u64 = 0; // only property_version 0 can vote as fungible token

    struct DAO has key {
        /// Name of the DAO
        name: String,
        /// The threshold that the proposal can resolve, which is an absolute number of NFT voted
        resolve_threshold: u64,
        /// The NFT Collection that is used to govern the DAO
        governance_token: GovernanceToken, // This is the governance token or NFT used for voting
        /// The voting duration in secs
        voting_duration: u64,
        /// Minimum weight for proposal
        min_required_proposer_voting_power: u64,
        /// Proposal counter
        next_proposal_id: u64,
    }

    /// The collection should only contains NFTs, where all token name only has 1 maximal and immutable
    /// The total supply is fixed with the token names.
    struct GovernanceToken has store {
        /// The creator address of this NFT collection
        creator: address,
        /// The collection name
        collection: String,
        /// The token name
        name: String,
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
        /// The function arguments to be exectued
        function_args: PropertyMap,
        /// The start time of the voting
        start_time_sec: u64,
        /// Proposal results, unresolved, passed, not passed
        resolution: u8,
    }

    struct ProposalVotingStatistics has key {
        proposals: Table<u64, VotingStatistics>,
    }

    struct VotingStatistics has store {
        /// Total yes votes
        total_yes: u64,
        /// Total no notes
        total_no: u64,
        /// locked token during voting
        votes: Table<address, Token>,

    }

    struct RegisteredDAOs has key {
        /// Key is the address of DAO creator's account and value is the signer capability of its resource account
        /// Only one DAO per admin address for now.
        accounts_signer_caps: Table<address, SignerCapability>,
        res_acct_to_src_acct: Table<address, address>,
    }


    /// Initialize the DAO platform
    fun init_module(account :&signer) {
        move_to(
            account,
            RegisteredDAOs {
                accounts_signer_caps: table::new(),
                res_acct_to_src_acct: table::new(),
            }
        );
    }

    /// Creator creates a DAO on the platform
    public entry fun create_dao(
        admin: &signer, // dao creator
        name: String, // name of the DAO
        threshold: u64, // the absolute number of token required to resolve a proposal
        voting_duration: u64,
        creator: address, // this is the creator address of goverance token
        collection_name: String, // collection of the fungible token
        token_name: String, // token name
        min_required_proposer_voting_power: u64, // minimal token required to vote
    ) acquires RegisteredDAOs {

        // create a resource account
        let seed = bcs::to_bytes(&name);
        vector::append(&mut seed, bcs::to_bytes(&creator));
        vector::append(&mut seed, bcs::to_bytes(&collection_name));

        let (res_signer, res_cap) = account::create_resource_account(admin, seed);
        let res_addr = signer::address_of(&res_signer);
        let src_addr = signer::address_of(admin);
        join_platform(src_addr, res_addr, res_cap);

        assert!(string::length(&name) < 128, error::invalid_argument(ESTRING_TOO_LONG));

        move_to(
            &res_signer,
            DAO {
                name,
                resolve_threshold: threshold,
                governance_token: GovernanceToken { creator, collection: collection_name, name: token_name },
                voting_duration,
                min_required_proposer_voting_power,
                next_proposal_id: 0,
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

        emit_create_dao_event(
            &res_signer,
            name,
            threshold,
            voting_duration,
            min_required_proposer_voting_power,
            creator,
            collection_name,
        )
    }


    /// Only DAO Goverance token holders can create proposal
    public entry fun create_proposal(
        account: &signer,
        ft_dao: address, // resource account address of the nft dao
        name: String,
        description: String,
        function_name: String,
        arg_names: vector<String>, // name of the arguments of the function to be called
        arg_values: vector<vector<u8>>, // bcs serailized values of argument values
        arg_types:vector<String>, // types of arguments. currently, we only support string, u8, u64, u128, bool, address.
        start_time_sec: u64,
    ) acquires DAO, Proposals {
        let dao = borrow_global_mut<DAO>(ft_dao);
        assert!(string::length(&name) <= 64, error::invalid_argument(ESTRING_TOO_LONG));
        assert!(string::length(&description) <= 512, error::invalid_argument(ESTRING_TOO_LONG));

        // verify the account's token has enough weights to create proposal
        let weights = get_token_balance(account, dao);
        assert!(weights >= dao.min_required_proposer_voting_power, error::permission_denied(EVOTING_WEIGHTS_NOT_ENOUGH));

        // verify the parameters are legit
        let pm = property_map::new(arg_names, arg_values, arg_types);
        assert_function_valid(function_name, &pm);

        // verify the start_time is in future
        let now = timestamp::now_seconds();
        assert!(start_time_sec > now, error::invalid_argument(EPROPOSAL_START_TIME_SHOULD_IN_FUTURE));

        let proposal = Proposal {
            name,
            description,
            function_name,
            function_args: pm,
            start_time_sec,
            resolution: PROPOSAL_UNRESOLVED,
        };

        let proposal_store = borrow_global_mut<Proposals>(ft_dao);
        let proposal_id = dao.next_proposal_id + 1;
        table::add(&mut proposal_store.proposals, proposal_id, proposal);
        dao.next_proposal_id = proposal_id;
    }

    /// Vote with a batch of tokens. Tokens will be stored in a locker during voting period
    public entry fun vote(
        account: &signer,
        ft_dao: address,
        proposal_id: u64,
        vote: bool,
        amount: u64,
    ) acquires DAO, ProposalVotingStatistics, Proposals {
        let dao = borrow_global_mut<DAO>(ft_dao);
        let gtoken = &dao.governance_token;
        let proposals = borrow_global<Proposals>(ft_dao);

        // assert the proposal hasn't ended
        let proposal = table::borrow(&proposals.proposals, proposal_id);
        let now = timestamp::now_seconds();
        assert!(now < proposal.start_time_sec + dao.voting_duration, error::invalid_argument(EPROPOSAL_ENDED));

        let prop_stats = borrow_global_mut<ProposalVotingStatistics>(ft_dao);
        if (!table::contains(&prop_stats.proposals, proposal_id)) {
            let vstat = VotingStatistics {
                total_yes: 0,
                total_no: 0,
                votes: table::new(),
            };
            table::add(&mut prop_stats.proposals, proposal_id, vstat);
        };
        let stats = table::borrow_mut(&mut prop_stats.proposals, proposal_id);

        let voter_addr = signer::address_of(account);
        let token_id = token::create_token_id_raw(gtoken.creator, gtoken.collection, dao.governance_token.name, PROPERTY_VERSION);

        // this account owns the token
        let balance = token::balance_of(signer::address_of(account), token_id);
        assert!(balance >= amount, error::permission_denied(ENOT_OWN_ENOUGH_VOTING_DAO_TOKEN));
        let token = token::withdraw_token(account, token_id, amount);
        if (vote) {
            stats.total_yes = stats.total_yes + amount;
        } else {
            stats.total_no = stats.total_no + amount;
        };
        table::add(&mut stats.votes, voter_addr, token);
    }

    /// Entry function that can be called by anyone
    public entry fun resolve(proposal_id: u64, ft_dao: address) acquires Proposals, DAO, ProposalVotingStatistics, RegisteredDAOs {
        // validate if proposal is ready to resolve
        let dao = borrow_global_mut<DAO>(ft_dao);

        // assert the proposal voting ended
        let proposals = borrow_global_mut<Proposals>(ft_dao);
        let proposal = table::borrow_mut(&mut proposals.proposals, proposal_id);
        let now = timestamp::now_seconds();
        assert!(now >= proposal.start_time_sec + dao.voting_duration, error::invalid_argument(EPROPOSAL_ENDED));

        // assert the proposal is unresolved yet
        assert!(proposal.resolution == PROPOSAL_UNRESOLVED, error::invalid_argument(EPROPOSAL_RESOLVED));

        let proposal_stat = &mut borrow_global_mut<ProposalVotingStatistics>(ft_dao).proposals;
        let voting_stat = table::borrow_mut(proposal_stat, proposal_id);
        // validate resolve threshold and result
        let voted = voting_stat.total_no + voting_stat.total_yes;
        if (voted < dao.resolve_threshold) {
            // not sufficient token voted
            proposal.resolution = PROPOSAL_RESOLVED_NOT_PASSED;
            return
        };
        let passed = if (voting_stat.total_yes > voting_stat.total_no) {true} else {false};
        if (passed) {
            let accts = &borrow_global<RegisteredDAOs>(@dao_platform).res_acct_to_src_acct;
            assert!(table::contains(accts, ft_dao), error::not_found(ERESOURCE_ACCT_NOT_EXIST));
            let src_acct = *table::borrow(accts, ft_dao);

            let function_name = proposal.function_name;
            if (function_name == string::utf8(b"transfer_fund")) {
                // This is very dangerous.
                // We should exploring have server side dynamic compiling and deploying DAO contracts in each DAO's own accounts.
                let res_signer = get_dao_signer(src_acct);
                let dst_addr = property_map::read_address(&proposal.function_args, &string::utf8(b"dst"));
                let amount = property_map::read_u64(&proposal.function_args, &string::utf8(b"amount"));
                transfer_fund(&res_signer, dst_addr, amount);
            } else {
               assert!(function_name == string::utf8(b"no_op"), error::invalid_argument(ENOT_SUPPROTED_FUNCTION));
            };
            proposal.resolution = PROPOSAL_RESOLVED_PASSED;
        } else {
            proposal.resolution = PROPOSAL_RESOLVED_NOT_PASSED;
        };
    }

    /// DAO creator can delegate its DAO to another admin account
    public entry fun delegate_dao(admin: &signer, new_admin: address)acquires RegisteredDAOs {
        let registered_accounts = borrow_global_mut<RegisteredDAOs>(@dao_platform);
        let old_addr = signer::address_of(admin);
        assert!(table::contains(&registered_accounts.accounts_signer_caps, old_addr), error::not_found(ENOT_FOUND_ADMIN_ACCOUNT));
        assert!(!table::contains(&registered_accounts.accounts_signer_caps, old_addr), error::not_found(EDELEGATED_ADDRESS_ALREADY_EXISTS));

        let signer_cap = table::remove(&mut registered_accounts.accounts_signer_caps, old_addr);
        let res_addr = get_signer_capability_address(&signer_cap);
        table::add(&mut registered_accounts.accounts_signer_caps, new_admin, signer_cap);

        let src_addr = table::borrow_mut(&mut registered_accounts.res_acct_to_src_acct, res_addr);
        *src_addr = new_admin;

    }
    /// Claim the escrowed token during voting
    public entry fun claim_token(voter: &signer, dao: address, proposal_id: u64) acquires ProposalVotingStatistics, Proposals {
        let proposals = borrow_global_mut<Proposals>(dao);
        let proposal = table::borrow_mut(&mut proposals.proposals, proposal_id);
        assert!(
            proposal.resolution == PROPOSAL_RESOLVED_PASSED || proposal.resolution == PROPOSAL_RESOLVED_NOT_PASSED,
            error::permission_denied(ECANNOT_WITHDRAW_BEFORE_PROPOSAL_RESOLVED)
        );

        assert!(exists<ProposalVotingStatistics>(dao), error::not_found(ERESOURCE_ACCT_NOT_EXIST));
        let proposal_stat = &mut borrow_global_mut<ProposalVotingStatistics>(dao).proposals;
        assert!(table::contains(proposal_stat, proposal_id), error::not_found(EPRPOSAL_ID_NOT_EXIST));
        let voting_stat = table::borrow_mut(proposal_stat, proposal_id);
        let addr = signer::address_of(voter);
        assert!(table::contains(&voting_stat.votes, addr), error::not_found(ENO_LOCKED_TOKEN_FOUND));
        let token = table::remove(&mut voting_stat.votes, addr);
        token::deposit_token(voter, token);
    }

    /// DAO creator can quit the platform and claim back his resource account signer capability
    public fun reclaim_signer_capability(account: &signer): (address, SignerCapability) acquires RegisteredDAOs {
        let addr = signer::address_of(account);
        let registered_accounts = borrow_global_mut<RegisteredDAOs>(@dao_platform);
        assert!(table::contains(&registered_accounts.accounts_signer_caps, addr), error::not_found(ERESOURCE_ACCT_NOT_EXIST));
        let cap = table::borrow(&registered_accounts.accounts_signer_caps, addr);
        let resource_addr = get_signer_capability_address(cap);
        let resource_cap = table::remove(&mut registered_accounts.accounts_signer_caps, addr);
        table::remove(&mut registered_accounts.res_acct_to_src_acct, resource_addr);
        (resource_addr, resource_cap)
    }

    /// Get the corresponding resource account address of the source account
    public fun get_resource_account_address(source: address): address acquires RegisteredDAOs {
        let reg_accts = borrow_global_mut<RegisteredDAOs>(@dao_platform);
        assert!(table::contains(&reg_accts.accounts_signer_caps, source), error::not_found(ERESOURCE_ACCT_NOT_EXIST));
        let cap = table::borrow(&reg_accts.accounts_signer_caps, source);
        get_signer_capability_address(cap)
    }

    /// Get the proposal
    public fun get_proposal(proposal_id: u64, nft_dao: address): Proposal acquires Proposals {
        assert!(exists<Proposals>(nft_dao), error::not_found(EPRPOSALS_NOT_EXIST_AT_ADDRESS));
        let proposals = &borrow_global<Proposals>(nft_dao).proposals;
        assert!(table::contains(proposals, proposal_id), error::not_found(EPRPOSAL_ID_NOT_EXIST));
        *table::borrow(proposals, proposal_id)
    }

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

    /// Unpack the DAO fields
    public fun unpack_dao(nft_dao: address): (String, u64, address, String, u64, u64, u64) acquires DAO {
        let dao = borrow_global_mut<DAO>(nft_dao);
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

    /// Acquire the signer of the DAO resource account administratored by src account
    public fun acqurire_dao_signer_with_src_account_signer(src: &signer): signer acquires RegisteredDAOs {
        let src_addr = signer::address_of(src);
        let reg_accts = borrow_global_mut<RegisteredDAOs>(@dao_platform);
        let cap = table::borrow(& reg_accts.accounts_signer_caps, src_addr);
        create_signer_with_capability(cap)
    }

    fun join_platform(
        src_address: address,
        res_address: address,
        signer_cap: SignerCapability
    ) acquires RegisteredDAOs {
        let registered_accounts = borrow_global_mut<RegisteredDAOs>(@dao_platform);
        table::add(&mut registered_accounts.accounts_signer_caps, src_address, signer_cap);
        table::add(&mut registered_accounts.res_acct_to_src_acct, res_address, src_address);
    }

    // transfer APT fund from the DAO account to the destination account
    fun transfer_fund(res_acct: &signer, dst: address, amount: u64) {
        coin::transfer<AptosCoin>(res_acct, dst, amount);
    }

    fun get_token_balance(
        account: &signer,
        dao: &DAO
    ): u64 {
        let gtoken = &dao.governance_token;
        let token_id = token::create_token_id_raw(gtoken.creator, gtoken.collection, dao.governance_token.name, PROPERTY_VERSION);
        token::balance_of(signer::address_of(account), token_id)
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

    fun get_dao_signer(src_addr: address): signer acquires RegisteredDAOs {
        let reg_accts = borrow_global_mut<RegisteredDAOs>(@dao_platform);
        let cap = table::borrow(& reg_accts.accounts_signer_caps, src_addr);
        create_signer_with_capability(cap)
    }


    #[test(aptos_framework = @0x1, platform = @dao_platform, creator = @0xdeaf, voter = @0xaf)]
    public fun test_e2e_scenario(aptos_framework: &signer, platform: &signer, creator: &signer, voter: &signer) acquires RegisteredDAOs, DAO, Proposals, ProposalVotingStatistics {
        timestamp::set_time_has_started_for_testing(aptos_framework);
        account::create_account_for_test(@dao_platform);
        account::create_account_for_test(@0xdeaf);
        account::create_account_for_test(@0xaf);
        init_module(platform);

        // create an NFT collection
        let token_id = token::create_collection_and_token(
            creator,
            10,
            10,
            10,
            vector::empty(),
           vector::empty(),
            vector::empty(),
            vector<bool>[false, false, false],
            vector<bool>[false, false, false, false, false],
        );

        token::direct_transfer(creator, voter, token_id, 5);

        // creator creates a dao
        let creator_addr = signer::address_of(creator);
        create_dao(
            creator,
            string::utf8(b"my_dao"),
            3,
            10,
            creator_addr,
            string::utf8(b"Hello, World"),
            string::utf8(b"Token"),
            1,
        );

        // resource account created
        let res_acc = get_resource_account_address(creator_addr);

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
        );

        vote(
            voter,
            res_acc,
            1,
            false,
            4,
        );
        timestamp::update_global_time_for_test(20000000);
        resolve(1, res_acc);
        assert!(get_proposal_resolution(1, res_acc) == PROPOSAL_RESOLVED_NOT_PASSED, 1);
        claim_token(voter, res_acc, 1);
    }
}
