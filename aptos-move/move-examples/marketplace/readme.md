Aptos NFT Marketplace Example
------------

NOTE: THIS IS AN EXAMPLE AND HAS NOT BEEN FULLY AUDITED. THESE CONTRACTS ARE FOR COLLECTING FEEDBACK FROM OUR BUILDERS. ONCE WE ARE CONFIDENT THAT IT IS BENEFICIAL TO THE TOKEN ECOSYSTEM, WE WILL ADD IT TO the 0x3::aptos-token PACKAGE.

Introduction
------------

The package contains two parts:

-   marketplace utility functions: these contracts specify the basic function and data structures for building a fixed-price marketplace and the auction house. The two contracts are: (1) marketplace_bid_utils and (2) marketplace_listing_utils
-   example marketplace contracts: these contracts show two examples of building a marketplace leveraging the marketplace utility functions.

Design principles
-----------------

We want to have a minimal required example to improve the liquidity of the token ecosystem

-   Provide a unified Listing so that the same listing can be used across different marketplaces and aggregators.
-   Provide a unified buy and bid functions so that people can buy or bid for listed NFT across different marketplaces
-   Provide unified events so that downstream applications can have a clear overview of what is happening in the token ecosystem across all marketplaces

We want app developers to be creative with how they run their marketplace and auction house.

-   We separate the listing, buy and bid from other business logic to put them in utility functions.
-   We only provide example marketplace contracts for demonstration. Each marketplace is supposed to deploy its own contracts to its account. They decide how to charge the fee and how to select the bids that win the auction.

**Design Choices**
------------------

We also made the following design choices when we implemented the marketplace utility contracts. Any feedback on these choices is highly appreciated and helps the community.

-   Escrow-less listing: the seller can keep their tokens in their token stores and use the token (eg: show in the wallet, use the token, etc) before their token is sold.
-   The seller can choose who owns their listings. The listing can be stored under a marketplace account or stored under sellers' accounts. If the seller wants to work with a particular marketplace, they can give the listing to the marketplace to store after creating the listing. The marketplace can then decide how to expose the listing to the buyers. If the seller stores the listing under their own account, anyone can buy from these listings and these listings can be aggregated by aggregators.
-   Bidders have to lock their coins during the auction and can only withdraw the coin after the auction expires. Bidder can only increase their bid while the auction is still active. This ensures the bid is valid and the bidder cannot withdraw the coin while the auction is still active.

FAQ:
----

**Why not store the token in the listing to guarantee the token exists?**

We want to achieve two goals here, first, the token exists in the owner's token store before it is sold. second, the listed token should be available for transfer.

It is important to keep the token in the token store so that downstream apps, indexer, wallets can easily know the tokens owned by an account. The owner of the token can then use these listed tokens before the token is sold, as a listing can exist for a long time.

To check whether the listed token is available, there are many ways to handle this problem. For example, tracking the lister's token store events or using an indexer to verify if the owner still has the listed tokens. The marketplace can cancel the listing if the token balance is not enough.

Meanwhile, we will enhance the token store in our token standard to provide options to lock the token so that these tokens cannot be transferred out during the locking period.

**How to support new features in this marketplace?**

We will continuously collect new common features from the community and add them to the contracts in a backward-compatible way.

**What is the plan for this package?**

We plan to have these contracts in the move-example and collect the feedbacks from community.
Once we have gone through enough iterations and be confident that it is beneficial to the token ecosystem, we will propose it to include them in the 0x3 aptos-token package.
