# Introduction

The word Aptos means \"the people\" in Ohlone. The economics and
governance processes of the Aptos blockchain are openly and
transparently decided by its community and subsequently encoded in the
underlying technology. The Aptos blockchain [@aptos_blockchain] is
designed to be safe, scalable, and upgradeable infrastructure for web3
applications. To build a people-first, secure, fair, and sustainable
ecosystem, Aptos defines a set of core principles and a corresponding
set of incentive and governance mechanisms to achieve them. In addition
to building the safest and most scalable blockchain, Aptos coordinates
major changes through on-chain configurability without no downtime.

With a community-first philosophy and principle of rapid iteration
combined with ever-evolving best practices in economics and governance,
the Aptos blockchain will continue to make improvements to mechanisms
and parameters over time.[^1]

# Principles {#sec:principles}

Below are the principles that underlie the community mechanisms of
incentives and governance in the Aptos economy.

-   People-first - The Aptos blockchain intends to be a community-driven
    project in every dimension. From an open source codebase, a diverse
    set of node operators, open policies and decision making, the
    network will evolve according to the will of the participants.

-   Transparency - The initial distribution, genesis construction,
    economic mechanisms and their goals are designed to be openly shared
    with everyone and open to feedback from the community. All changes
    will be made publicly through well-defined and open governance
    processes.

-   Equality - Provide the same terms among groups of participants, for
    example, uniform lockup terms across investors and core
    contributors. All validator operators are subject to the same rules.

-   Simplicity - Each participant should be able to understand the
    incentive mechanisms in order to be able to reason about how to
    optimize best for their own usage of the network. Simplicity is
    prioritized over more complex mechanisms in order to promote
    participant fairness and increase overall network security. While
    additional complexity can potentially enable more flexible
    mechanisms, it increases the difficulty of implementation and
    testing as well as opens up a wider attack surface that is hard to
    understand and remediate.

-   Decentralization - Blockchains can be considered decentralized
    databases that can support different safety and liveness guarantees.
    While the Aptos blockchain supports safety and liveness with up to
    $f$ malicious stake (see Section
    [4](#proof-of-stake){reference-type="ref"
    reference="proof-of-stake"}, it is up to society to determine the
    ultimate security of the network. Validators can fork the mainnet,
    and participants can choose which fork to follow. Adding support for
    more participants to join in the security of the network is a
    priority.

-   Stability - Large and frequent changes in economic policy can lead
    to demand volatility. Economic policies that lead to substantial
    changes in supply can also lead to demand volatility. Significant
    changes must capture the voice of all participants while leaving
    flexibility to quickly handle critical situations such as a
    correctness bug or a safety attack.

-   Sustainability - The Aptos ecosystem is built to last. The
    mechanisms must support major changes though evolving governance
    processes in order to adapt to new improvements in technology and
    ideas from the community.

# Token supply

As a result of the selected incentive mechanisms and implementations,
APT will have a variable token supply. Additionally, the ability of
on-chain governance to change these economic and governance rules
necessitates the lack of constraints. The Aptos blockchain requires a
variable token supply, rather than a fixed token supply, to have more
flexibility with respect to the economic levers to ensure that validator
operators, token holders, and users can be incentivized to achieve the
principles of the Aptos network.

In order to model the projected total supply, assumptions are made
regarding the average validator performance rate and the transaction
fees in the early days of the network. Governance proposals accepted by
the community may also affect the transaction fees and staking rewards,
and introduce new mechanisms such as storage capacity incentives and
multi-dimensional resource bidding. Any change to the underlying
mechanisms and/or related parameters will affect the project token
supply.

## Coin and token

For digital assets, Aptos provides two Move modules:[@aptos_cointoken]

Aptos Coin - The coin.move Move module is a lightweight standard meant
for simple, typesafe, and fungible assets. The coin standard is
separated out into its own Move module to ensure that:

The coin standard can be used to create a token with an emphasis on
simplicity and performance and with minimal metadata. The coin module
remains a part of the Aptos core framework and be used for currencies,
for example the gas currency, thereby enhancing the core functionality
of the Aptos framework. See Aptos Coin \>

Aptos Token - The token.move Move module, on the other hand:

Encapsulates rich, flexible assets, fungible and nonfungible, and
collectibles. The token standard is deployed as a separate package at
the Aptos blockchain address 0x3. The token standard is designed to
create an NFT or a semi-fungible or a fungible non-decimal token, with
rich metadata and functionalities. A token definition of this type can
be iterated rapidly to respond to the platform and user requirements.

## Initial token distribution totals

The native Aptos token for transaction and networks fees, governance
voting, and staking will have the ticker APT. At genesis, there will be
an initial distribution of 1,000,000,000 APT. One APT is divisible to
eight decimal places where the minimal unit is called an *octa* (i.e.
$10^{-8}$ APT = 1 octa). Octa is both the singular and plural form of
the unit. The distribution will be as follows:

::: center
  Category            \% of initial token distribution   Initial APT tokens
  ------------------- ---------------------------------- --------------------
  Community           51.02%                             510,217,359.7670
  Core contributors   19.00%                             190,000,000.0000
  Foundation          16.50%                             165,000,000.0000
  Investors           13.48%                             134,782,640.2330
:::

This distribution reflects the goals of the network.

-   Community - Sustainable growth of the diverse and global ecosystem
    is the most important priority for the network. To that end, the
    ecosystem category represents both a majority of total allocation
    and is by far the largest allocation. This includes airdrops,
    funding for communities and ecosystem projects, and diverse
    development of Aptos infrastructure (e.g. explorers, wallets, API
    services, blockchain services, etc.).

-   Core contributors - World class research and development of the
    network and its infrastructure is paramount to the mission of the
    Aptos blockchain to provide a safe, scalable, and upgradeable web3
    infrastructure for billions of people worldwide.

-   Foundation - The Aptos Foundation is a non-profit organization
    dedicated to the decentralization, security, and growth of the Aptos
    network and its surrounding ecosystem. The Aptos Foundation will
    have staff and operational expenses and in

-   Investors - This group has supported the early development of the
    Aptos blockchain and represents the smallest category of
    distribution.

In recent history, this distribution represents the largest initial
distribution toward community and ecosystem development for a layer 1
blockchain. Additionally, the investor bucket is the smallest category.
In terms of private and public sales, the investor category is the
lowest among layer 1 blockchains.

## Initial token distribution schedule

Tokens from the initial distribution follow the unlock schedule as
described below. Token unlock schedules follow two options:

-   4-year unlock schedule - 3/48 month month on month 13 for the next 6
    months, 1/48 on month 19 - month 48.

-   10-year unlock schedule - linear monthly unlock for 10 years
    starting from the genesis event

::: center
  Category            Initial unlocked tokens   Locked tokens    Start of unlock                          Unlock schedule
  ------------------- ------------------------- ---------------- ---------------------------------------- -----------------
  Community           125,000,000               385,217,359.77   Mainnet                                  10 year
  Core contributors   0                         190,000,000      max(Mainnet, Start date) $+$ 13 months   4 year
  Foundation          5,000,000                 160,000,000      Mainnet                                  10 year
  Investors           0                         134,782,640.23   Mainnet $+$ 13 months                    4 year
:::

During genesis, all accounts are initialized according to the initial
distribution. This process is fully transparent and can be independently
verified with the on-chain account data. The tokens allocated to core
contributors and investors are automatically locked up according to the
distribution schedule with most of them being enforced in smart
contracts.

In the Community category, 125,000,000 tokens are available initially to
provide immediate supply for airdrops, grants, and other incentives for
community members and builders (115,000,000 from the Aptos Foundation
and 10,000,000 from the core contributors). The remaining 385,217,359.77
tokens will be subject to the 10-year unlock schedule in order to
provide the ability to support these community initiatives for at least
a decade and beyond.

In the Core contributors category, as new developers join the Aptos
developer community after the launch of the Aptos blockchain, their
unlock schedules from 13 months after their start date. Developers who
started prior to mainnet launch begin their unlock schedules 13 months
from mainnet launch. Investors follow the same unlock schedule as
developers who joined prior to mainnet launch (also beginning their
unlock schedule 13 months after mainnet launch). A minimum of a 13 month
delayed unlock period followed by a two gradual unlock periods over a 4
year time frame emits smaller supply changes over a longer period of
time.

Foundation - A maximum of 5,000,000 tokens are initially unlocked to
support operational expenses and explore new avenues of decentralization
and security with the remainder subject to a linear unlock over 10
years.

Staking rewards are not part of the initial token distribution as there
are no staking rewards prior to the mainnet launch. Staking rewards are
detailed in [5.4](#staking-rewards){reference-type="ref"
reference="staking-rewards"} and are under no restrictions.

# Proof-of-stake model {#proof-of-stake}

The Aptos blockchain is a Byzantine fault-tolerant (BFT) proof-of-stake
network where the liveness and safety of the on-chain network is
protected by its token holders. The total token supply will gradually
change over time, and every token is owned by a token holder. Token
holders can decide whether to stake their tokens in order to secure the
network. Staked tokens earn rewards when helping liveness and safety and
could face losses if provably hurting the network. Staked tokens are
locked up for a period of time in order to encourage longer periods of
correct behavior.

The Aptos technology will rapidly change over time; therefore, the
initial rules of the system described below will be updated as
appropriate. The Aptos proof-of-stake security model depends on token
holders to stake their tokens with secure and responsive validator
operators. The total staked tokens at any point of time are used in the
Aptos consensus protocols and on-chain voting. Every staked token
constitutes a single vote; so participants who stake more tokens gain
proportionately more voting weight. The Aptos consensus protocol ensures
safety and liveness when there is at most $f < n/3$ Byzantine stake
[@aptosbft_v4].

# Incentive mechanisms {#sec:incentive_mechanisms}

Operating the Aptos blockchain has hardware, software, coordination, and
non-technical costs. There are several groups of participants involved
including token holders, validator operators, researchers/developers,
and network users. Non-technical operations such as business, human
resources, and legal are also required.

-   Token holders are a broad set of participants (many of whom overlap
    with other participant groups) that help secure the Aptos blockchain
    through their token holdings. They may acquire tokens to use for
    staking, governance, and/or network usage.

-   Validator operators incur operational costs for running validators.
    They have setup and maintenance costs to rent or acquire hardware
    and attain access to high-speed Internet and reliable power.
    Validator operators must keep up-to-date with the latest software
    upgrades, develop experience and tooling to run on different
    platforms, train up on disaster recovery, and handle planned and
    unplanned incidents.

-   Researchers and developers work on ecosystem tools and applications,
    smart contact languages, blockchain development, and hardware.

-   Network users read and write data to the Aptos blockchain.
    Transaction workloads dependent on network usage are difficult to
    predict. Transactions consume short-term and longer-term resources
    for validators. Short-term resource consumption includes CPU,
    memory, and I/O operations and bandwidth. Long-term resource
    consumption includes storage usage and sunk costs (such as hardware
    purchases).

Below we describe the initial mechanisms to distribute and consume
tokens to the appropriate participants. As the network develops, new
mechanisms will be added and parameters will be adjusted through the
appropriate governance processes to fully enable the principles in
Section [2](#sec:principles){reference-type="ref"
reference="sec:principles"}.

## Network fees

Gas metering is a concept fundamental to Aptos and many other
blockchains --- it defines an abstract measurement of the amount of
computational and storage resources required to execute and store a
transaction. This is similar to running a car with gasoline or heating
with natural gas. The gas schedule then codifies these costs across all
operations and is used to compute the amount of gas used during the
execution of a transaction.

The cost for an operation should be directly related to the available
resources on the network (e.g. CPU, memory, network, storage I/Os and
space usage, etc.). Moreover, this cost should reflect the evolution of
resource cost changes over time due to new technology and process
improvements. Gas should be set by the on-chain governance and be
seamlessly configurable. Gas prevents distributed denial of service
(DDoS) attacks on a fixed set of resources in the network and may need
to be adjusted swiftly through governance proposals depending on network
situation. Aptos Gas Price reflects the Aptos Foundation's desire to
accelerate growth and keep the blockchain accessible to everyone. The
Aptos gas model motivates good choices in design --- such as
prioritizing safety, modularity, assertions, and leveraging events.

Network fees will be collected and dispersed to the validators over time
in order to encourage them to prioritize the highest value transactions.

## Gas measurements

Aptos transactions by default charge a base gas fee, regardless of
market conditions. For each transaction, this \"base gas\" amount is
based on three conditions:[@aptos_basegas]

-   Instructions

-   Storage

-   Payload

The more function calls, branching conditional statements, etc. that a
transaction requires, the more instruction gas it will cost. Likewise,
the more reads from and writes into global storage that a transaction
requires, the more storage gas it will cost. Finally, the more bytes in
a transaction payload, the more it will cost.

When a user submits a transaction, they must specify two quantities in
the transaction:

Max gas amount - Measured in gas units. This is the maximum number of
gas units that the user (i.e., transaction sender) is willing to spend
to execute the transaction.

Gas unit price - Measured in octa per gas unit, where 1 octa =
0.00000001 APT (=$10^{-8}$). [@aptos_gasblog] This is the gas price the
user is willing to pay.

During execution, a transaction will be charged:

-   An intrinsic cost, with a fixed base plus extra for big
    transactions.

-   Execution costs, for executing Move instructions.

-   Read costs, for reading the data from the persistent storage.

-   Write costs, for writing the data into the persistent storage.

The final transaction fee can be calculated by multiplying the total
amount of gas consumed (measured in gas units) and the gas unit price.
For example, if a transaction consumes 670 gas units and the gas unit
price specified by the user in the transaction is 100 Octa per unit,
then the final transaction fee would be 670 \* 100 = 67000 Octa =
0.00067 APT.

If a transaction runs out of gas during execution, then the sender will
be charged based on the max gas amount and all changes made by this
transaction will be reverted.

## Gas schedule

Basic configuration - There are several components of the gas schedule
that do not relate to the specifics of an individual operation. These
include transaction size and maximum gas units (different from the
maximum gas amount that the user specifies in the transaction).

Transaction size - For most transactions, the transaction size will
likely be on the order of a kilobyte. However, Move module publishing
can easily be several kilobytes and the Aptos Framework is on the order
of 100 KB. Also, most user modules tend to be between 4KB and 40KB.
Initially, we set the value of the transaction size to 32KB but the
community responded quickly and asked for more space to make application
development easy, so it was adjusted to 64KB.

Very large transactions induce bandwidth costs on the entire network and
can have a negative impact on the performance. If abused, mempools are
incentivized to ignore larger transactions, so our approach is to strike
a balance between maximum transaction size and the accessibility.

Maximum gas units - The maximum gas units in the gas schedule defines
how many operations a transaction can perform. Note that this is
different from the maximum gas amount that the user specifies in the
transaction.

The gas schedule's maximum gas units has direct implications on how long
a transaction can execute. Setting it too high can result in
transactions that can have negative performance implications on the
blockchain. For example, a user may forget to have an increment in a
while loop resulting in an infinite loop, an unfortunately common bug.
We found that even with our largest framework upgrade, we were still at
less than 90% of gas schedule's maximum gas units, which is set at
1,000,000.

## Staking rewards

Validator operators and stakers are rewarded for participating in
consensus through on-chain data. Rewards are proportional to staked APT.
Validator operators and stakers split rewards according to on-chain
and/or off-chain methods, where each validator may have different reward
splits. The validator rewards schedule is planned to begin at 7%
initially and decline 1.5% a year until reaching a fixed rate of 3.25%.
In order to keep reward calculations simple in Move, the yearly reward
rate does not include re-staking validator operator rewards, and a year
is assumed to be 365 days. Since it is possible that validators do not
perform well, the maximum reward rate is an upper bound on the increased
token supply. Additionally, staked tokens ratios can vary depending on
transaction movement and other token usages. In the early days, rewards
can not go negative (i.e. slashing). However, negative reward rates and
the reward schedule can be revisited in the future through on-chain
governance.

Rewards accumulate during an *epoch*, a period of time that the
blockchain configuration is static, and can be claimed by validator
operators and/or stakers after a *lockup duration*. Initially, the
typical epoch duration is 2 hours, and the lockup duration is 30 days.
Stakers can elect to reset the lockup duration start at any time (e.g.
in order to vote on governance proposals). All of these parameters are
configurable through on-chain governance. Staked tokens automatically
re-enroll until the token holder decides to stop staking.

Below are the three formulas used to calculate the per-epoch reward for
each validator operator individually:

-   $fixed\ reward\ rate = Yearly\ reward\ rate\ /\ \#\ of\ estimated\ epochs\ per\ year$

-   $epoch\ proposal\ performance = \#\ of\ success\ proposals\ /\ \#\ of\ total\ proposals$

-   $epoch\ reward = total\ stake\ *\ epoch\ proposal\ performance\ *\ fixed\ rewards\ rate$

In order for a validator operator to participate in the network, it must
have at least the *minimum stake* requirements and cannot exceed a
*maximum stake*, current at 1,000,000 and 50,000,000 APT, respectively.
Over time, these parameters will evolve according to the desires of the
community. The minimum stake required likely would decrease over time as
the network grows to reduce the barrier of entry for running a validator
node and encourage further network distribution.

# Governance

The Aptos blockchain is built for, operated by, and governed by the
people. Ultimately, the participants of a system decide the state of
that system. Participants are always the ultimate decision-makers on
what constitutes value and value transfer. Blockchain technology
provides another system for agreement on value and value transfer. At
its core, the Aptos blockchain is a permissionless database where
participants can read and write state that follows a set of rules.
Ultimately, it is the participants who decide whether they want to
interact with the blockchain, a fork of the blockchain, or an entirely
different system.

This design produces a balanced decision-making process with respect to
different participants:

-   Core developers

    -   Research, propose and implement functionality for the blockchain

-   Token holders

    -   Can propose, vote on, and execute on-chain governance proposals

    -   Can delegate their voting rights to other on-chain accounts

    -   Can decide to increase or decrease voting rights by changing the
        amount of locked/staked tokens and/or modifying their individual
        token supply

-   Validator operators

    -   Decide which version of the software binaries are deployed,
        underlying hardware, deployment procedures, security processes,
        and when changes are made

    -   Ultimately decide on what the rules of the network are (and
        therefore also the state)

    -   Can hard fork if they choose to

-   Full node operators

    -   Verify and notify other participants that the validator
        operators are performing correctly (e.g. executing transactions
        as expected and following the rules of consensus)

-   Everyone

    -   Can leave the network

    -   Can create or join another fork of the network

## Decentralized control

All parties share responsibility for this system of value and value
transfer. Core developers implement changes. Token holders select
validators to stake in, vote or delegate votes, can liquidate their
tokens, or leave the network. Validator operators may elect to fork and
can choose whether to change binaries. On-chain governance is merely a
method in which validator operators effectively delegate their
decision-making to token holders for practical purposes - to coordinate
more quickly and in a transparent manner. Full nodes that follow the
chain will observe when the rules are not being followed and alert the
other participants of this issue. In this design, no one set of
participants determines the rules of the network. Changes and
decision-making are open to participation and transparent by default.

## Coordination

Governance processes are designed to be open, flexible, clear, and
provide adequate time for community feedback. There are governance
processes that are organizational as well as code changes that alter the
network directly (e.g. upgrading the consensus protocol, changing the
signature scheme, changing the gas schedule, upgrading a core Move
module, or deploying a new binary). This document focuses on
code-related changes rather than organizational changes.

When new code must be added to the software binary, there must be a
binary update by validator operators. Many changes, however, such as a
modification of the gas schedule parameters and generic new Aptos
framework updates, can be accomplished with an existing software binary.
New code added to the software binary must also be enabled at some point
in the future, and this can be triggered through:

-   An off-chain method (e.g. changing a configuration flag)

-   On-chain governance (see Section
    [6.3](#on-chain-governance){reference-type="ref"
    reference="on-chain-governance"})

-   On-chain synchronization point (e.g. At an agreed upon logical or
    physical time in the system)

On-chain coordination is more transparent than off-chain coordination
and is also verifiable. It is clear which participants voted and how
they voted (although future work can support private token voting if the
community desires). However, ultimately, on-chain governance does not
change the governance hierarchy; it simply exposes the governance
processes transparently and with well-defined and enforceable rules. For
example, validator operators still have the power to reject such
actions, and full nodes ensure validator operators are behaving as
expected. And as mentioned previously, validator operators often
delegate their coordination of network changes (that already exist in
the software) to the token holders.

## On-chain governance

On-chain governance proposals first should acquire support from the
community via public forum posts before moving to the voting stage. This
allows the community to discuss the rationale and details of a proposal
and voice their opinions. Once there's enough support, a community
member can proceed with creating the official on-chain proposal. The
proposer needs to lock up a certain amount of stake, *required proposer
stake*, currently set at 10M APT, in order to ensure their incentives
are aligned with the network's. After the proposal is created, any APT
token holder can lock up their tokens and vote.

On chain governance supports both a normal mode of operation with a
voting period of 7 days and an instant resolution mode (e.g. for
emergency upgrades). The threshold of normal operation requires that a
*minimum voting threshold* is exceeded for a governance proposal to be
accepted or rejected, currently at 400M APT). For instant resolution,
50% of the total token supply must vote with an additional 1 APT.
Through this dual mode of resolution, on-chain governance can be used to
support a broad set of changes in an open and transparent way, governed
by token holders.

Once executed, a governance proposal takes effect immediately across the
entire validator set without the common time-based triggers often found
in blockchain software upgrades. This minimizes the risk of network
forks and allows the network to move and innovate more quickly and
safely.

Insert picture of this:

## Summary

The participants of the Aptos network determine value and value
transfer. The Aptos blockchain makes all transactions and state changes
transparent and verifiable under BFT guarantees. Participants all have
the option to continue using the Aptos blockchain, a fork of the Aptos
blockchain, or leave the network entirely. Decentralized control is
demonstrably shared among different participants without any one entity
having control over other participants in the network.

[^1]: Legal Disclaimer: This white paper and its contents are not an
    offer to sell, or the solicitation of an offer to buy, any tokens.
    We are publishing this white paper solely to receive feedback and
    comments from the public. Nothing in this document should be read or
    interpreted as a guarantee or promise of how the Aptos blockchain or
    its tokens (if any) will develop, be utilized, or accrue value.
    Aptos only outlines its current plans, which could change at its
    discretion, and the success of which will depend on many factors
    outside of its control. Such future statements necessarily involve
    known and unknown risks, which may cause actual performance and
    results in future periods to differ materially from what we have
    described or implied in this white paper. Aptos undertakes no
    obligation to update its plans. There can be no assurance that any
    statements in the white paper will prove to be accurate, as actual
    results and future events could differ materially. Please do not
    place undue reliance on future statements.
