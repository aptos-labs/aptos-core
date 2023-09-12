# Prominent coin distribution methods

- Airdrop
  - Requires either centralized KYC or is prone to exploitation by bots.
  - Does not distribute supply in pro rata fashion.
- Initial coin offering (ICO)
  - Constitutes a sale.
  - Immediate liquidity floods the market, leading to volatility.

# The lockstream

## Locking and streaming mechanics

In a lockstream, a creator seeds a pool with an initial base asset supply (e.g. `PRO`, an unlaunched protocol coin), and specifies several time windows.
During the first time window, the locking period, anyone can permissionlessly lock a quote asset (e.g. `APT`) in the pool.
Lockers can lock multiple times during the locking period, and the pool automatically tracks the total quote locked amount for each locker across all such locking events.

|                    | Ace            | Bee            |
| ------------------ | -------------- | -------------- |
| Lock event 1       | Locks 20 `APT` | Locks 15 `APT` |
| Lock event 2       | -              | Locks 65 `APT` |
| Total quote locked | 20 `APT`       | 80 `APT`       |

Here, the total quote locked for the pool is $20 + 80 = 100$ `APT`.
For a pool created with 2000 `PRO`, this means the pool has:

| Property             | Amount     |
| -------------------- | ---------- |
| Initial base locked  | 2000 `PRO` |
| Initial quote locked | 100 `APT`  |

After the locking period is over the streaming period begins, and lockers can start claiming their pro rata portion of the base asset supply in the pool, based on their individual contribution to the total quote locked for the entire pool.

|                        | Ace       | Bee        |
| ---------------------- | --------- | ---------- |
| Pro rata share of base | 400 `PRO` | 1600 `PRO` |

Moreover, lockers can also claim back their original locked quote amount, but not all at once:
like a constant stream of water slowly filling up a bucket, the lockstream only permits base and quote assets to be claimed in proportion to how much of the streaming window has elapsed.
Lockers can claim whenever they want and as many times as they want, but the lockstream tracks the total amount that each locker has already claimed and only returns the difference between their eligible claim amount and total claims so far:

|               | % Streaming period elapsed | Ace receives        | Bee receives         |
| ------------- | -------------------------- | ------------------- | -------------------- |
| Claim event 1 | 25                         | 100 `PRO`, 5 `APT`  | -                    |
| Claim event 2 | 50                         | 100 `PRO`, 5 `APT`  | -                    |
| Claim event 3 | 100                        | 200 `PRO`, 10 `APT` | 1600 `PRO`, 80 `APT` |

After the streaming period has completed, lockers have an additional "claim last call period" during which they can claim any eligible assets that they didn't claim during the streaming period:
the claim last call period prevents the otherwise contentious situation of everyone trying to submit claim transactions right at the end of the streaming period, which would be expected in the absence of a claim last call period.

## Lockstream implications

The lockstream incorporates the Zahavi handicap principle, a game theoretic concept from the field of sexual selection theory, which can be summarized as:

> Costly signals are reliable signals

For example, the male Peacock's extravagant tail is in one sense tantamount to declaring, "look here female, my genes are *so good* and I am *so fit* that no predator will hunt me down, and I can *afford* to have this wasteful plumage, so mate with me and your offspring will prosper."
Or more generally, in the words of evolutionary biologist Geoffrey Miller [\[ref\]](#ref), "the handicap principle suggests that prodigious waste is a necessary feature of courtship."

Similarly, in a lockstream, lockers are essentially signalling their worth as holders of the base asset by surrendering the opportunity cost of the locked quote asset:
"I have so much `APT` that I can *afford* to lock this much up, so give me `PRO` because I make sound asset allocation decisions and will treat the coin with as much care as I treat my `APT` bags."
Here, total `APT` holdings is taken as a proxy for ecosystem engagement, faith in the longevity of the Aptos blockchain, etc.

Notably, since lockers end up getting all of their locked quote asset back, the lockstream does not constitute token *sale*, because the pool creator gets nothing in return for the base asset they seed the pool with:
the pool creator is *giving away* the base asset.
Moreover, the streaming mechanism that gradually introduces the unlocked base asset into circulation smooths out volatility traditionally associated with large coin distribution events.

### Reference

Miller, Geoffrey. The Mating Mind: How Sexual Choice Shaped the Evolution of Human Nature. New York: Anchor Books, 2001. pp. 123—135

## Sweeping and the premier locker

By the time the claim last call period has ended it is expected that not all lockers will have claimed all of their quote principal or pro rata base amount:
people lose their keys, forget, become apathetic, die, etc.
Hence the lockstream implements a sweep function, which allows a single locker to drain all unclaimed assets from the pool after the claim last call period has ended, in winner-take-all fashion.

A priority claim to the sweep function is afforded to the "premier locker", a privileged role that is assigned during the locking period (before the streaming period even begins):
the first locker in the lockstream automatically becomes the premier locker, but each time someone elses locks a higher total quote amount, they become the new premier locker.
This mechanism sets up a bidding war between the largest participants in the lockstream during the initial locking period:

|              | Ace             | Bee             | Premier locker                 |
| ------------ | --------------- | --------------- | ------------------------------ |
| Lock event 1 | Locks 20 `APT`  | -               | Ace                            |
| Lock event 2 | -               | Locks 21 `APT`  | Bee                            |
| Lock event 3 | Locks 2 `APT`   | -               | Ace                            |
| Lock event 4 | -               | Locks 1.5 `APT` | Bee                            |
| Lock event 4 | Locks 0.5 `APT` |                 | Still Bee, Ace did not outlock |

The premier locker, as a coveted position with the right to obtain more than one's principal quote or pro rata share of base assets, plays to the psychological factors underpinning much of capitalism:
in other words, the premier locker position is designed to set up an ego battle between crypto whales.

Note however, that even crypto whales are prone to lose their keys, forget, become apathetic, die, etc., hence the premier locker is also bounded by a last call time.
After the "premier sweep last call time", before which only the premier locker is entitled to execute the sweep operation, *anyone* can invoke the sweep operations.
This means that in the bizarre event that the largest locker in the lockstream does not invoke their sweep priority, things are made even more interesting by all the remaining lockers blasting the pool with sweep transactions at the end of the premier sweep last call time.

## Configuration

During pool creation, the creator transfers in the base asset locked amount and configures the following time parameters, all specified in UNIX seconds:

| Time                         | Meaning                                                  |
| ---------------------------- | -------------------------------------------------------- |
| Stream start time            | When lockers can start claiming                          |
| Stream end time              | When lockers can claim all quote and pro rata base       |
| Claim last call time         | The last chance to claim                                 |
| Premier sweep last call time | The last chance for the premier locker to sweep the pool |

These result in the following periods:

| Period                 | Period time bounds                                           |
| ---------------------- | ------------------------------------------------------------ |
| Locking period         | $t <$ Stream start time                                      |
| Streaming period       | Stream start time $\leq t \leq$ Stream end time              |
| Claiming grace period  | Stream end time $< t \leq$ Claim last call time              |
| Premier sweep period   | Claim last call time $< t \leq$ Premier sweep last call time |
| Mercenary sweep period | Premier sweep last call time $< t$                           |

# Demo script

Build the Docker image:

> This might take a while, since it compiles several Aptos binaries from source.

```sh
# From aptos-core root
EXAMPLE_DIR=./aptos-move/move-examples/lockstream
docker build . -f "$EXAMPLE_DIR/Dockerfile" -t lockstream-example
```

Start a local chain, then run the Python script against it:

```sh
chain_container=$(docker run \
    --detach \
    --publish 8080:8080 \
    lockstream-example \
    aptos node run-local-testnet --test-dir /app/data)
docker run \
    --volume $EXAMPLE_DIR:/app/scripts \
    --network host \
    --workdir /app \
    lockstream-example \
    poetry run python scripts/example.py
docker stop $chain_container
```