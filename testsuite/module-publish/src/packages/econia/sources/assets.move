/// Mock asset types for on- and off-chain testing.
module econia::assets {

    // Uses >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>

    use aptos_framework::coin;
    use std::signer::address_of;
    use std::string::utf8;

    // Uses <<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<

    // Test-only uses >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>

    #[test_only]
    use aptos_framework::account;
    #[test_only]
    use aptos_framework::aptos_coin;

    // Test-only uses <<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<

    // Structs >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>

    /// Stores mock coin type capabilities.
    struct CoinCapabilities<phantom CoinType> has key {
        burn_capability: coin::BurnCapability<CoinType>,
        freeze_capability: coin::FreezeCapability<CoinType>,
        mint_capability: coin::MintCapability<CoinType>
    }

    /// Base coin type.
    struct BC{}

    /// Quote coin type.
    struct QC{}

    /// Utility coin type.
    struct UC{}

    /// Aditional coin types
    struct AC{}

    struct DC{}

    struct EC{}

    struct FC{}

    struct GC{}

    struct HC{}
    
    struct IC{}

    struct JC{}

    struct KC{}

    struct LC{}

    struct MC{}

    struct NC{}

    struct OC{}

    struct PC{}

    struct RC{}

    struct SC{}

    struct TC{}

    struct VC{}

    struct WC{}

    struct XC{}

    struct AAC{}

    struct ABC{}

    struct ACC{}

    struct ADC{}

    struct AEC{}

    struct AFC{}

    struct AGC{}

    struct AHC{}

    struct AIC{}

    struct AJC{}

    struct AKC{}

    struct ALC{}

    struct AMC{}

    struct ANC{}

    struct AOC{}

    struct APC{}

    struct AQC{}

    struct ARC{}

    struct ASC{}

    struct ATC{}

    struct AUC{}

    struct AVC{}

    struct AWC{}

    struct AXC{}

    struct AYC{}

    struct AZC{}

    struct BAC{}

    struct BBC{}

    struct BCC{}

    struct BDC{}

    struct BEC{}

    struct BFC{}

    struct BGC{}

    struct BHC{}

    struct BIC{}

    struct BJC{}

    struct BKC{}

    struct BLC{}

    struct BMC{}

    struct BNC{}

    struct BOC{}

    struct BPC{}

    struct BQC{}

    struct BRC{}

    struct BSC{}

    struct BTC{}

    struct BUC{}

    struct BVC{}

    struct BWC{}

    struct BXC{}

    struct BYC{}

    struct BZC{}

    struct CAC{}

    struct CBC{}

    struct CCC{}

    struct CDC{}

    struct CEC{}

    struct CFC{}

    struct CGC{}

    struct CHC{}

    struct CIC{}

    struct CJC{}

    struct CKC{}

    struct CLC{}

    struct CMC{}

    struct CNC{}

    struct COC{}

    struct CPC{}

    struct CQC{}

    struct CRC{}

    struct CSC{}

    struct CTC{}

    struct CUC{}

    struct CVC{}

    struct CWC{}

    struct CXC{}

    struct CYC{}

    struct CZC{}

    struct DAC{}

    struct DBC{}

    struct DCC{}

    struct DDC{}

    struct DEC{}

    struct DFC{}

    struct DGC{}

    struct DHC{}

    struct DIC{}

    struct DJC{}

    struct DKC{}

    struct DLC{}

    struct DMC{}

    struct DNC{}

    struct DOC{}

    struct DPC{}

    struct DQC{}

    struct DRC{}

    struct DSC{}

    struct DTC{}

    struct DUC{}

    struct DVC{}

    struct DWC{}

    struct DXC{}

    struct DYC{}

    struct DZC{}



    // Structs <<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<

    // Error codes >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>

    /// Caller is not Econia.
    const E_NOT_ECONIA: u64 = 0;
    /// Coin capabilities have already been initialized.
    const E_HAS_CAPABILITIES: u64 = 1;

    // Error codes <<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<

    // Constants >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>

    /// Base coin name.
    const BASE_COIN_NAME: vector<u8> = b"Base coin";
    /// Base coin symbol.
    const BASE_COIN_SYMBOL: vector<u8> = b"BC";
    /// Base coin decimals.
    const BASE_COIN_DECIMALS: u8 = 4;
    /// Quote coin name.
    const QUOTE_COIN_NAME: vector<u8> = b"Quote coin";
    /// Quote coin symbol.
    const QUOTE_COIN_SYMBOL: vector<u8> = b"QC";
    /// Quote coin decimals.
    const QUOTE_COIN_DECIMALS: u8 = 12;
    /// Utility coin name.
    const UTILITY_COIN_NAME: vector<u8> = b"Utility coin";
    /// Utility coin symbol.
    const UTILITY_COIN_SYMBOL: vector<u8> = b"UC";
    /// Utility coin decimals.
    const UTILITY_COIN_DECIMALS: u8 = 10;

    // Constants <<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<

    // Public functions >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>

    /// Burn `coins` for which `CoinType` is defined at Econia account.
    public fun burn<CoinType>(
        coins: coin::Coin<CoinType>
    ) acquires CoinCapabilities {
        // Borrow immutable reference to burn capability.
        let burn_capability = &borrow_global<CoinCapabilities<CoinType>>(
                @econia).burn_capability;
        coin::burn<CoinType>(coins, burn_capability); // Burn coins.
    }

    /// Mint new `amount` of `CoinType`, aborting if not called by
    /// Econia account.
    public fun mint<CoinType>(
        account: &signer,
        amount: u64
    ): coin::Coin<CoinType>
    acquires CoinCapabilities {
        // Get account address.
        let account_address = address_of(account); // Get account address.
        // Assert caller is Econia.
        assert!(account_address == @econia, E_NOT_ECONIA);
        // Borrow immutable reference to mint capability.
        let mint_capability = &borrow_global<CoinCapabilities<CoinType>>(
                account_address).mint_capability;
        // Mint specified amount.
        coin::mint<CoinType>(amount, mint_capability)
    }

    // Public functions <<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<

    // Private functions >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>

    /// Initialize given coin type under Econia account.
    fun init_coin_type<CoinType>(
        account: &signer,
        coin_name: vector<u8>,
        coin_symbol: vector<u8>,
        decimals: u8,
    ) {
        // Assert caller is Econia.
        // assert!(address_of(account) == @econia, E_NOT_ECONIA);
        // Assert Econia does not already have coin capabilities stored.
        assert!(!exists<CoinCapabilities<CoinType>>(address_of(account)),
            E_HAS_CAPABILITIES);
        // Initialize coin, storing capabilities.
        let (burn_capability, freeze_capability, mint_capability) =
        coin::initialize<CoinType>(
            account, utf8(coin_name), utf8(coin_symbol), decimals, false);
        move_to<CoinCapabilities<CoinType>>(account,
            CoinCapabilities<CoinType>{
                burn_capability,
                freeze_capability,
                mint_capability
        }); // Store capabilities under Econia account.
    }

    /// Initialize mock base, quote, and utility coin types upon genesis
    /// publication.
    fun init_module(
        account: &signer
    ) {
        init_coin_type<BC>(account, BASE_COIN_NAME, BASE_COIN_SYMBOL,
            BASE_COIN_DECIMALS); // Initialize mock base coin.
        init_coin_type<QC>(account, QUOTE_COIN_NAME, QUOTE_COIN_SYMBOL,
            QUOTE_COIN_DECIMALS); // Initialize mock quote coin.
        init_coin_type<UC>(account, UTILITY_COIN_NAME, UTILITY_COIN_SYMBOL,
            UTILITY_COIN_DECIMALS); // Initialize mock utility coin.
        if (!exists<CoinCapabilities<AC>>(address_of(account))) init_coin_type<AC>(account, b"A Coin", b"AC", 10); // Initialize A coin
        if (!exists<CoinCapabilities<DC>>(address_of(account))) init_coin_type<DC>(account, b"D Coin", b"DC", 10); // Initialize D coin
        if (!exists<CoinCapabilities<FC>>(address_of(account))) init_coin_type<FC>(account, b"F Coin", b"FC", 10); // Initialize F coin
        if (!exists<CoinCapabilities<EC>>(address_of(account))) init_coin_type<EC>(account, b"E Coin", b"EC", 10); // Initialize E coin
        if (!exists<CoinCapabilities<GC>>(address_of(account))) init_coin_type<GC>(account, b"G Coin", b"GC", 10); // Initialize G coin
        if (!exists<CoinCapabilities<HC>>(address_of(account))) init_coin_type<HC>(account, b"H Coin", b"HC", 10); // Initialize H coin
        if (!exists<CoinCapabilities<IC>>(address_of(account))) init_coin_type<IC>(account, b"I Coin", b"IC", 10); // Initialize I coin
        if (!exists<CoinCapabilities<JC>>(address_of(account))) init_coin_type<JC>(account, b"J Coin", b"JC", 10); // Initialize J coin
        if (!exists<CoinCapabilities<KC>>(address_of(account))) init_coin_type<KC>(account, b"K Coin", b"KC", 10); // Initialize K coin
        if (!exists<CoinCapabilities<LC>>(address_of(account))) init_coin_type<LC>(account, b"L Coin", b"LC", 10); // Initialize L coin
        if (!exists<CoinCapabilities<MC>>(address_of(account))) init_coin_type<MC>(account, b"M Coin", b"MC", 10); // Initialize M coin
        if (!exists<CoinCapabilities<NC>>(address_of(account))) init_coin_type<NC>(account, b"N Coin", b"NC", 10); // Initialize N coin
        if (!exists<CoinCapabilities<OC>>(address_of(account))) init_coin_type<OC>(account, b"O Coin", b"OC", 10); // Initialize O coin
        if (!exists<CoinCapabilities<PC>>(address_of(account))) init_coin_type<PC>(account, b"P Coin", b"PC", 10); // Initialize P coin
        if (!exists<CoinCapabilities<RC>>(address_of(account))) init_coin_type<RC>(account, b"R Coin", b"RC", 10); // Initialize R coin
        if (!exists<CoinCapabilities<SC>>(address_of(account))) init_coin_type<SC>(account, b"S Coin", b"SC", 10); // Initialize S coin
        if (!exists<CoinCapabilities<TC>>(address_of(account))) init_coin_type<TC>(account, b"T Coin", b"TC", 10); // Initialize T coin
        if (!exists<CoinCapabilities<VC>>(address_of(account))) init_coin_type<VC>(account, b"V Coin", b"VC", 10); // Initialize V coin
        if (!exists<CoinCapabilities<WC>>(address_of(account))) init_coin_type<WC>(account, b"W Coin", b"WC", 10); // Initialize W coin
        if (!exists<CoinCapabilities<XC>>(address_of(account))) init_coin_type<XC>(account, b"X Coin", b"XC", 10); // Initialize X coin
        if (!exists<CoinCapabilities<AAC>>(address_of(account))) init_coin_type<AAC>(account, b"AA Coin", b"AAC", 10); // Initialize AA coin
        if (!exists<CoinCapabilities<ABC>>(address_of(account))) init_coin_type<ABC>(account, b"AB Coin", b"ABC", 10); // Initialize AB coin
        if (!exists<CoinCapabilities<ACC>>(address_of(account))) init_coin_type<ACC>(account, b"AC Coin", b"ACC", 10); // Initialize AC coin
        if (!exists<CoinCapabilities<ADC>>(address_of(account))) init_coin_type<ADC>(account, b"AD Coin", b"ADC", 10); // Initialize AD coin
        if (!exists<CoinCapabilities<AEC>>(address_of(account))) init_coin_type<AEC>(account, b"AE Coin", b"AEC", 10); // Initialize AE coin
        if (!exists<CoinCapabilities<AFC>>(address_of(account))) init_coin_type<AFC>(account, b"AF Coin", b"AFC", 10); // Initialize AF coin
        if (!exists<CoinCapabilities<AGC>>(address_of(account))) init_coin_type<AGC>(account, b"AG Coin", b"AGC", 10); // Initialize AG coin
        if (!exists<CoinCapabilities<AHC>>(address_of(account))) init_coin_type<AHC>(account, b"AH Coin", b"AHC", 10); // Initialize AH coin
        if (!exists<CoinCapabilities<AIC>>(address_of(account))) init_coin_type<AIC>(account, b"AI Coin", b"AIC", 10); // Initialize AI coin
        if (!exists<CoinCapabilities<AJC>>(address_of(account))) init_coin_type<AJC>(account, b"AJ Coin", b"AJC", 10); // Initialize AJ coin
        if (!exists<CoinCapabilities<AKC>>(address_of(account))) init_coin_type<AKC>(account, b"AK Coin", b"AKC", 10); // Initialize AK coin
        if (!exists<CoinCapabilities<ALC>>(address_of(account))) init_coin_type<ALC>(account, b"AL Coin", b"ALC", 10); // Initialize AL coin
        if (!exists<CoinCapabilities<AMC>>(address_of(account))) init_coin_type<AMC>(account, b"AM Coin", b"AMC", 10); // Initialize AM coin
        if (!exists<CoinCapabilities<ANC>>(address_of(account))) init_coin_type<ANC>(account, b"AN Coin", b"ANC", 10); // Initialize AN coin
        if (!exists<CoinCapabilities<AOC>>(address_of(account))) init_coin_type<AOC>(account, b"AO Coin", b"AOC", 10); // Initialize AO coin
        if (!exists<CoinCapabilities<APC>>(address_of(account))) init_coin_type<APC>(account, b"AP Coin", b"APC", 10); // Initialize AP coin
        if (!exists<CoinCapabilities<AQC>>(address_of(account))) init_coin_type<AQC>(account, b"AQ Coin", b"AQC", 10); // Initialize AQ coin
        if (!exists<CoinCapabilities<ARC>>(address_of(account))) init_coin_type<ARC>(account, b"AR Coin", b"ARC", 10); // Initialize AR coin
        if (!exists<CoinCapabilities<ASC>>(address_of(account))) init_coin_type<ASC>(account, b"AS Coin", b"ASC", 10); // Initialize AS coin
        if (!exists<CoinCapabilities<ATC>>(address_of(account))) init_coin_type<ATC>(account, b"AT Coin", b"ATC", 10); // Initialize AT coin
        if (!exists<CoinCapabilities<AUC>>(address_of(account))) init_coin_type<AUC>(account, b"AU Coin", b"AUC", 10); // Initialize AU coin
        if (!exists<CoinCapabilities<AVC>>(address_of(account))) init_coin_type<AVC>(account, b"AV Coin", b"AVC", 10); // Initialize AV coin
        if (!exists<CoinCapabilities<AWC>>(address_of(account))) init_coin_type<AWC>(account, b"AW Coin", b"AWC", 10); // Initialize AW coin
        if (!exists<CoinCapabilities<AXC>>(address_of(account))) init_coin_type<AXC>(account, b"AX Coin", b"AXC", 10); // Initialize AX coin
        if (!exists<CoinCapabilities<AYC>>(address_of(account))) init_coin_type<AYC>(account, b"AY Coin", b"AYC", 10); // Initialize AY coin
        if (!exists<CoinCapabilities<AZC>>(address_of(account))) init_coin_type<AZC>(account, b"AZ Coin", b"AZC", 10); // Initialize AZ coin
        if (!exists<CoinCapabilities<BAC>>(address_of(account))) init_coin_type<BAC>(account, b"BA Coin", b"BAC", 10); // Initialize BA coin
        if (!exists<CoinCapabilities<BBC>>(address_of(account))) init_coin_type<BBC>(account, b"BB Coin", b"BBC", 10); // Initialize BB coin
        if (!exists<CoinCapabilities<BCC>>(address_of(account))) init_coin_type<BCC>(account, b"BC Coin", b"BCC", 10); // Initialize BC coin
        if (!exists<CoinCapabilities<BDC>>(address_of(account))) init_coin_type<BDC>(account, b"BD Coin", b"BDC", 10); // Initialize BD coin
        if (!exists<CoinCapabilities<BEC>>(address_of(account))) init_coin_type<BEC>(account, b"BE Coin", b"BEC", 10); // Initialize BE coin
        if (!exists<CoinCapabilities<BFC>>(address_of(account))) init_coin_type<BFC>(account, b"BF Coin", b"BFC", 10); // Initialize BF coin
        if (!exists<CoinCapabilities<BGC>>(address_of(account))) init_coin_type<BGC>(account, b"BG Coin", b"BGC", 10); // Initialize BG coin
        if (!exists<CoinCapabilities<BHC>>(address_of(account))) init_coin_type<BHC>(account, b"BH Coin", b"BHC", 10); // Initialize BH coin
        if (!exists<CoinCapabilities<BIC>>(address_of(account))) init_coin_type<BIC>(account, b"BI Coin", b"BIC", 10); // Initialize BI coin
        if (!exists<CoinCapabilities<BJC>>(address_of(account))) init_coin_type<BJC>(account, b"BJ Coin", b"BJC", 10); // Initialize BJ coin
        if (!exists<CoinCapabilities<BKC>>(address_of(account))) init_coin_type<BKC>(account, b"BK Coin", b"BKC", 10); // Initialize BK coin
        if (!exists<CoinCapabilities<BLC>>(address_of(account))) init_coin_type<BLC>(account, b"BL Coin", b"BLC", 10); // Initialize BL coin
        if (!exists<CoinCapabilities<BMC>>(address_of(account))) init_coin_type<BMC>(account, b"BM Coin", b"BMC", 10); // Initialize BM coin
        if (!exists<CoinCapabilities<BNC>>(address_of(account))) init_coin_type<BNC>(account, b"BN Coin", b"BNC", 10); // Initialize BN coin
        if (!exists<CoinCapabilities<BOC>>(address_of(account))) init_coin_type<BOC>(account, b"BO Coin", b"BOC", 10); // Initialize BO coin
        if (!exists<CoinCapabilities<BPC>>(address_of(account))) init_coin_type<BPC>(account, b"BP Coin", b"BPC", 10); // Initialize BP coin
        if (!exists<CoinCapabilities<BQC>>(address_of(account))) init_coin_type<BQC>(account, b"BQ Coin", b"BQC", 10); // Initialize BQ coin
        if (!exists<CoinCapabilities<BRC>>(address_of(account))) init_coin_type<BRC>(account, b"BR Coin", b"BRC", 10); // Initialize BR coin
        if (!exists<CoinCapabilities<BSC>>(address_of(account))) init_coin_type<BSC>(account, b"BS Coin", b"BSC", 10); // Initialize BS coin
        if (!exists<CoinCapabilities<BTC>>(address_of(account))) init_coin_type<BTC>(account, b"BT Coin", b"BTC", 10); // Initialize BT coin
        if (!exists<CoinCapabilities<BUC>>(address_of(account))) init_coin_type<BUC>(account, b"BU Coin", b"BUC", 10); // Initialize BU coin
        if (!exists<CoinCapabilities<BVC>>(address_of(account))) init_coin_type<BVC>(account, b"BV Coin", b"BVC", 10); // Initialize BV coin
        if (!exists<CoinCapabilities<BWC>>(address_of(account))) init_coin_type<BWC>(account, b"BW Coin", b"BWC", 10); // Initialize BW coin
        if (!exists<CoinCapabilities<BXC>>(address_of(account))) init_coin_type<BXC>(account, b"BX Coin", b"BXC", 10); // Initialize BX coin
        if (!exists<CoinCapabilities<BYC>>(address_of(account))) init_coin_type<BYC>(account, b"BY Coin", b"BYC", 10); // Initialize BY coin
        if (!exists<CoinCapabilities<BZC>>(address_of(account))) init_coin_type<BZC>(account, b"BZ Coin", b"BZC", 10); // Initialize BZ coin
        if (!exists<CoinCapabilities<CAC>>(address_of(account))) init_coin_type<CAC>(account, b"CA Coin", b"CAC", 10); // Initialize CA coin
        if (!exists<CoinCapabilities<CBC>>(address_of(account))) init_coin_type<CBC>(account, b"CB Coin", b"CBC", 10); // Initialize CB coin
        if (!exists<CoinCapabilities<CCC>>(address_of(account))) init_coin_type<CCC>(account, b"CC Coin", b"CCC", 10); // Initialize CC coin
        if (!exists<CoinCapabilities<CDC>>(address_of(account))) init_coin_type<CDC>(account, b"CD Coin", b"CDC", 10); // Initialize CD coin
        if (!exists<CoinCapabilities<CEC>>(address_of(account))) init_coin_type<CEC>(account, b"CE Coin", b"CEC", 10); // Initialize CE coin
        if (!exists<CoinCapabilities<CFC>>(address_of(account))) init_coin_type<CFC>(account, b"CF Coin", b"CFC", 10); // Initialize CF coin
        if (!exists<CoinCapabilities<CGC>>(address_of(account))) init_coin_type<CGC>(account, b"CG Coin", b"CGC", 10); // Initialize CG coin
        if (!exists<CoinCapabilities<CHC>>(address_of(account))) init_coin_type<CHC>(account, b"CH Coin", b"CHC", 10); // Initialize CH coin
        if (!exists<CoinCapabilities<CIC>>(address_of(account))) init_coin_type<CIC>(account, b"CI Coin", b"CIC", 10); // Initialize CI coin
        if (!exists<CoinCapabilities<CJC>>(address_of(account))) init_coin_type<CJC>(account, b"CJ Coin", b"CJC", 10); // Initialize CJ coin
        if (!exists<CoinCapabilities<CKC>>(address_of(account))) init_coin_type<CKC>(account, b"CK Coin", b"CKC", 10); // Initialize CK coin
        if (!exists<CoinCapabilities<CLC>>(address_of(account))) init_coin_type<CLC>(account, b"CL Coin", b"CLC", 10); // Initialize CL coin
        if (!exists<CoinCapabilities<CMC>>(address_of(account))) init_coin_type<CMC>(account, b"CM Coin", b"CMC", 10); // Initialize CM coin
        if (!exists<CoinCapabilities<CNC>>(address_of(account))) init_coin_type<CNC>(account, b"CN Coin", b"CNC", 10); // Initialize CN coin
        if (!exists<CoinCapabilities<COC>>(address_of(account))) init_coin_type<COC>(account, b"CO Coin", b"COC", 10); // Initialize CO coin
        if (!exists<CoinCapabilities<CPC>>(address_of(account))) init_coin_type<CPC>(account, b"CP Coin", b"CPC", 10); // Initialize CP coin
        if (!exists<CoinCapabilities<CQC>>(address_of(account))) init_coin_type<CQC>(account, b"CQ Coin", b"CQC", 10); // Initialize CQ coin
        if (!exists<CoinCapabilities<CRC>>(address_of(account))) init_coin_type<CRC>(account, b"CR Coin", b"CRC", 10); // Initialize CR coin
        if (!exists<CoinCapabilities<CSC>>(address_of(account))) init_coin_type<CSC>(account, b"CS Coin", b"CSC", 10); // Initialize CS coin
        if (!exists<CoinCapabilities<CTC>>(address_of(account))) init_coin_type<CTC>(account, b"CT Coin", b"CTC", 10); // Initialize CT coin
        if (!exists<CoinCapabilities<CUC>>(address_of(account))) init_coin_type<CUC>(account, b"CU Coin", b"CUC", 10); // Initialize CU coin
        if (!exists<CoinCapabilities<CVC>>(address_of(account))) init_coin_type<CVC>(account, b"CV Coin", b"CVC", 10); // Initialize CV coin
        if (!exists<CoinCapabilities<CWC>>(address_of(account))) init_coin_type<CWC>(account, b"CW Coin", b"CWC", 10); // Initialize CW coin
        if (!exists<CoinCapabilities<CXC>>(address_of(account))) init_coin_type<CXC>(account, b"CX Coin", b"CXC", 10); // Initialize CX coin
        if (!exists<CoinCapabilities<CYC>>(address_of(account))) init_coin_type<CYC>(account, b"CY Coin", b"CYC", 10); // Initialize CY coin
        if (!exists<CoinCapabilities<CZC>>(address_of(account))) init_coin_type<CZC>(account, b"CZ Coin", b"CZC", 10); // Initialize CZ coin
        if (!exists<CoinCapabilities<DAC>>(address_of(account))) init_coin_type<DAC>(account, b"DA Coin", b"DAC", 10); // Initialize DA coin
        if (!exists<CoinCapabilities<DBC>>(address_of(account))) init_coin_type<DBC>(account, b"DB Coin", b"DBC", 10); // Initialize DB coin
        if (!exists<CoinCapabilities<DCC>>(address_of(account))) init_coin_type<DCC>(account, b"DC Coin", b"DCC", 10); // Initialize DC coin
        if (!exists<CoinCapabilities<DDC>>(address_of(account))) init_coin_type<DDC>(account, b"DD Coin", b"DDC", 10); // Initialize DD coin
        if (!exists<CoinCapabilities<DEC>>(address_of(account))) init_coin_type<DEC>(account, b"DE Coin", b"DEC", 10); // Initialize DE coin
        if (!exists<CoinCapabilities<DFC>>(address_of(account))) init_coin_type<DFC>(account, b"DF Coin", b"DFC", 10); // Initialize DF coin
        if (!exists<CoinCapabilities<DGC>>(address_of(account))) init_coin_type<DGC>(account, b"DG Coin", b"DGC", 10); // Initialize DG coin
        if (!exists<CoinCapabilities<DHC>>(address_of(account))) init_coin_type<DHC>(account, b"DH Coin", b"DHC", 10); // Initialize DH coin
        if (!exists<CoinCapabilities<DIC>>(address_of(account))) init_coin_type<DIC>(account, b"DI Coin", b"DIC", 10); // Initialize DI coin
        if (!exists<CoinCapabilities<DJC>>(address_of(account))) init_coin_type<DJC>(account, b"DJ Coin", b"DJC", 10); // Initialize DJ coin
        if (!exists<CoinCapabilities<DKC>>(address_of(account))) init_coin_type<DKC>(account, b"DK Coin", b"DKC", 10); // Initialize DK coin
        if (!exists<CoinCapabilities<DLC>>(address_of(account))) init_coin_type<DLC>(account, b"DL Coin", b"DLC", 10); // Initialize DL coin
        if (!exists<CoinCapabilities<DMC>>(address_of(account))) init_coin_type<DMC>(account, b"DM Coin", b"DMC", 10); // Initialize DM coin
        if (!exists<CoinCapabilities<DNC>>(address_of(account))) init_coin_type<DNC>(account, b"DN Coin", b"DNC", 10); // Initialize DN coin
        if (!exists<CoinCapabilities<DOC>>(address_of(account))) init_coin_type<DOC>(account, b"DO Coin", b"DOC", 10); // Initialize DO coin
        if (!exists<CoinCapabilities<DPC>>(address_of(account))) init_coin_type<DPC>(account, b"DP Coin", b"DPC", 10); // Initialize DP coin
        if (!exists<CoinCapabilities<DQC>>(address_of(account))) init_coin_type<DQC>(account, b"DQ Coin", b"DQC", 10); // Initialize DQ coin
        if (!exists<CoinCapabilities<DRC>>(address_of(account))) init_coin_type<DRC>(account, b"DR Coin", b"DRC", 10); // Initialize DR coin
        if (!exists<CoinCapabilities<DSC>>(address_of(account))) init_coin_type<DSC>(account, b"DS Coin", b"DSC", 10); // Initialize DS coin
        if (!exists<CoinCapabilities<DTC>>(address_of(account))) init_coin_type<DTC>(account, b"DT Coin", b"DTC", 10); // Initialize DT coin
        if (!exists<CoinCapabilities<DUC>>(address_of(account))) init_coin_type<DUC>(account, b"DU Coin", b"DUC", 10); // Initialize DU coin
        if (!exists<CoinCapabilities<DVC>>(address_of(account))) init_coin_type<DVC>(account, b"DV Coin", b"DVC", 10); // Initialize DV coin
        if (!exists<CoinCapabilities<DWC>>(address_of(account))) init_coin_type<DWC>(account, b"DW Coin", b"DWC", 10); // Initialize DW coin
        if (!exists<CoinCapabilities<DXC>>(address_of(account))) init_coin_type<DXC>(account, b"DX Coin", b"DXC", 10); // Initialize DX coin
        if (!exists<CoinCapabilities<DYC>>(address_of(account))) init_coin_type<DYC>(account, b"DY Coin", b"DYC", 10); // Initialize DY coin
        if (!exists<CoinCapabilities<DZC>>(address_of(account))) init_coin_type<DZC>(account, b"DZ Coin", b"DZC", 10); // Initialize DZ coin
    }

    // Private functions <<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<

    // Test-only functions >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>

    #[test_only]
    /// Wrapper for `init_module()`, not requiring signature.
    ///
    /// Similarly initializes the Aptos coin, destroying capabilities.
    public fun init_coin_types_test() {
        // Initialize Econia test coin types.
        init_module(&account::create_signer_with_capability(
            &account::create_test_signer_cap(@econia)));
        // Initialize Aptos coin type, storing capabilities.
        let (burn_cap, mint_cap) = aptos_coin::initialize_for_test(
            &account::create_signer_with_capability(
                &account::create_test_signer_cap(@aptos_framework)));
        // Destroy Aptos coin burn capability.
        coin::destroy_burn_cap(burn_cap);
        // Destroy Aptos coin mint capability.
        coin::destroy_mint_cap(mint_cap);
    }

   #[test_only]
    /// Wrapper for `mint()`, not requiring signature.
    public fun mint_test<CoinType>(
        amount: u64
    ): coin::Coin<CoinType>
    acquires CoinCapabilities {
        // Get Econia account.
        let econia = account::create_signer_with_capability(
            &account::create_test_signer_cap(@econia));
        // Initialize coin types if they have not been initialized yet.
        if (!exists<CoinCapabilities<CoinType>>(@econia)) init_module(&econia);
        mint(&econia, amount) // Mint and return amount.
    }

    // Test-only functions <<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<

    // Tests >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>

    #[test(econia = @econia)]
    #[expected_failure(abort_code = E_HAS_CAPABILITIES)]
    /// Verify failure for capabilities already registered.
    fun test_init_has_caps(
        econia: &signer
    ) {
        init_module(econia); // Initialize coin types.
        init_module(econia); // Attempt invalid re-init.
    }

    #[test(account = @user)]
    #[expected_failure(abort_code = E_NOT_ECONIA)]
    /// Verify failure for unauthorized caller.
    fun test_init_not_econia(
        account: &signer
    ) {
        init_module(account); // Attempt invalid init.
    }

    #[test(account = @econia)]
    /// Verify successful mint, then burn.
    fun test_mint_and_burn(
        account: &signer
    ) acquires CoinCapabilities {
        init_module(account); // Initialize coin types.
        let base_coin = mint<BC>(account, 20); // Mint base coin.
        // Assert correct value minted.
        assert!(coin::value(&base_coin) == 20, 0);
        burn<BC>(base_coin); // Burn coins.
        // Assert can burn another coin that has now been initialized.
        burn<QC>(mint(account, 1));
    }

    #[test(account = @user)]
    #[expected_failure(abort_code = E_NOT_ECONIA)]
    /// Verify failure for unauthorized caller.
    fun test_mint_not_econia(
        account: &signer
    ): coin::Coin<BC>
    acquires CoinCapabilities {
        mint<BC>(account, 20) // Attempt invalid mint.
    }

    // Tests <<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<

}