This introduces the core for a potential Velor standard around marketplace for assets on-chain.

The goals of this package are to
* Separate core logical components for readability and expansion over time.
* Where possible leverage function APIs as the layer of compatibility instead of exposing data structures.
* Leverage of objects and resource groups to unify common logic without wasting storage.
* Single definition of a fee schedule for the marketplace where the listing was created.
* Unified framework for auctions and fixed-price listings.
* Support for TokenV1, TokenV2, and Object based assets.
* Support for receiving funds in either Coin or FungibleAsset.

FeeSchedule includes:
* Listing, bidding, and commission
* Clean interface that allows newer business logic to be added over time, by passing in current pricing information

All listings support:
* Ability to specify a fixed purchase price
* Define when purchasing may begin
* Embed a fee schedule for the hosting marketplace
* Holding container for tokenv1 if the recipient does not have direct deposit enabled

Auctions support:
* Buy-it-now
* Incremental end times based upon the last bid time
* Minimum bid increments

Fixed-price support:
* Seller can end at any time.

Collection offer:
* Offerer can end at any time.

This is intended as an exploration into the ideal marketplace framework. Please make pull requests to extend it and generalize our use cases. This may never actually be deployed on Mainnet unless the community rallies behind a common marketplace and harness.
