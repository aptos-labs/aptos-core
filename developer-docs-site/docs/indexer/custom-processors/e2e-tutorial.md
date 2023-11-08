---
title: "End-to-End Tutorial"
---

# Creating a Custom Indexer Processor

In this tutorial, we're going to walk you through all the steps involved with creating a very basic custom indexer processor to track events and data on the Aptos blockchain.

We use a very simple smart contract called **Coin Flip** that has already emitted events for us.

The smart contract is already deployed, and you mostly don't need to understand it unless you're curious to mess with it or change things.

## Getting Started

To get started, clone the [aptos-indexer-processors](https://github.com/aptos-labs/aptos-indexer-processors) repo:
```
# HTTPS
https://github.com/aptos-labs/aptos-indexer-processors.git

# SSH
git@github.com:aptos-labs/aptos-indexer-processors.git
```

Navigate to the coin flip directory:
```
cd aptos-indexer-processors
cd python/processors/coin_flip
```

Processors consume a stream of transactions from the Transaction Stream Service. In order to use the Labs-Hosted Transaction Stream Service you need an auth token. Follow [this guide](/indexer/txn-stream/labs-hosted#auth-tokens) to guide to get one. Once you're done, you should have a token that looks like this:
```
aptoslabs_yj4bocpaKy_Q6RBP4cdBmjA8T51hto1GcVX5ZS9S65dx
```

You also need the following tools:
- The [Aptos CLI](/tools/aptos-cli/install-cli)
- Python 3.9+: [Installation Guide](https://docs.python-guide.org/starting/installation/#python-3-installation-guides).
- Poetry: [Installation Guide](https://python-poetry.org/docs/#installation).

We use postgresql as our database in this tutorial. You're free to use whatever you want, but this tutorial is geared towards postgresql for the sake of simplicity. We use the following database configuration and tools:
- [Postgresql](https://www.postgresql.org/download/)
    - We will use a database hosted on `localhost` on the port `5432`, which should be the default.
    - When you create your username, keep track of it and the password you use for it.
    - You can view a tutorial for installing postgresql and psql [here](https://www.digitalocean.com/community/tutorials/how-to-install-postgresql-on-ubuntu-22-04-quickstart) tool to set up your database more quickly.
    - If you want to easily view your database data, consider using a GUI like [DBeaver](https://dbeaver.io/), [pgAdmin](https://www.pgadmin.org/), or [Postico](https://eggerapps.at/postico2/).

Explaining how to create a database is beyond the scope of this tutorial. If you are not sure how to do it, consider checking out tutorials on how to create a database with the `psql` tool.

## Setup your environment

### Setup the postgresql database

Make sure to start the `postgresql` service:

The command for Linux/WSL might be something like:

```shell
sudo service postgresql start
```

For mac, if you're using brew, start it up with:

```shell
brew services start postgresql
```

Create your database with the name `coin_flip`, where our username is `user` and our password is `password`.

If your database is set up correctly, and you have the `psql` tool, you should be able to run the command `psql -d coin_flip`.

### Setup your local environment with poetry and grpc

If you haven't yet, make sure to read the introductory [custom processor guide](https://github.com/aptos-labs/aptos-indexer-processors).

You can also check out the python-specific broad overview of how to create an indexer processor [here](https://github.com/aptos-labs/aptos-indexer-processors/tree/main/python).

<!-- TODO: Move the above two docs into the docs site. -->

At the very least, make sure to install these tools and setup your poetry environment:

```shell
pip install grpcio-tools
poetry install

python3 -m grpc_tools.protoc --proto_path=./proto --python_out=python --pyi_out=python --grpc_python_out=python  \
          proto/aptos/bigquery_schema/v1/transaction.proto  \
          proto/aptos/indexer/v1/raw_data.proto \
          proto/aptos/internal/fullnode/v1/fullnode_data.proto \
          proto/aptos/transaction/v1/transaction.proto \
          proto/aptos/util/timestamp/timestamp.proto
```

## Configure your indexer processor

Now let's setup the configuration details for the actual indexer processor we're going to use.

### Setup your config.yaml file

Copy the contents below and save it to a file called `config.yaml`. Save it in the `coin_flip` folder. Your file directory structure should look something like this:

```
- indexer
    - proto
    - python
        - aptos_ambassador_token
        - aptos-tontine
        - coin_flip
            - move
                - sources
                    - coin_flip.move
                    - package_manager.move
                - Move.toml
            - config.yaml     <-------- Edit this config.yaml file
            - models.py
            - processor.py
            - README.md
        - example_event_processor
        - nft_marketplace_v2
        - nft_orderbooks
        __init__.py
        main.py
        README.md
    - rust
    - scripts
    - typescript
```

Once you have your config.yaml file open, you only need to change one field, if you just want to run the processor as is:
```yaml
grpc_data_stream_api_key: "<YOUR_API_KEY_HERE>"
```

### More customization with config.yaml

However, if you'd like to customize things further, you can change some of the other fields.

If you'd like to start at a specific version, you can specify that in the config.yaml file with:
```yaml
starting_version_default: 123456789
```

This is the transaction version the indexer starts looking for events at. If the indexer has already processed transactions past this version, **it will skip all of them and go to the latest version stored.**

The rows in `next_versions_to_process` are the `indexer_name` as the primary key and the `next_version` to process field, along with the `updated_at`.

If you want to **force** the indexer to backfill data (overwrite/rewrite data) from previous versions even though it's already indexed past it, you can specify this in the config.yaml file with:

```yaml
starting_version_backfill: 123456789
```

If you want to use a different network, change the `grpc_data_stream_endpoint` field to the corresponding desired value:

```yaml
devnet: 35.225.218.95:50051
testnet: 35.223.137.149:50051  # north america
testnet: 34.64.252.224:50051   # asia
mainnet: 34.30.218.153:50051
```

If these ip addresses don't work for you, they might be outdated. Check out the `README.md` at the root folder of the repository for the latest endpoints.

If you're using a different database name or processor name, change the `processor_name` field and the `db_connection_uri` to your specific needs. Here's the general structure of the field:

```yaml
db_connection_uri: "postgresql://username:password@database_url:port_number/database_name"
```

### Add your processor & schema names to the configuration files

First, let's create the name for the database schema we're going to use. We use `coin_flip` in our example, so we need to add it in two places:

1. We need to add it to our `python/utils/processor_name.py` file:
```python
    class ProcessorName(Enum):
        EXAMPLE_EVENT_PROCESSOR = "python_example_event_processor"
        NFT_MARKETPLACE_V1_PROCESSOR = "nft_marketplace_v1_processor"
        NFT_MARKETPLACE_V2_PROCESSOR = "nft_marketplace_v2_processor"
        COIN_FLIP = "coin_flip"
```
2. Add it to the constructor in the `IndexerProcessorServer` match cases in `utils/worker.py`:

```python
match self.config.processor_name:
    case ProcessorName.EXAMPLE_EVENT_PROCESSOR.value:
        self.processor = ExampleEventProcessor()
    case ProcessorName.NFT_MARKETPLACE_V1_PROCESSOR.value:
        self.processor = NFTMarketplaceProcesser()
    case ProcessorName.NFT_MARKETPLACE_V2_PROCESSOR.value:
        self.processor = NFTMarketplaceV2Processor()
    case ProcessorName.COIN_FLIP.value:
        self.processor = CoinFlipProcessor()
```

3. Add it to the `python/utils/models/schema_names.py` file:

```python
EXAMPLE = "example"
NFT_MARKETPLACE_SCHEMA_NAME = "nft_marketplace"
NFT_MARKETPLACE_V2_SCHEMA_NAME = "nft_marketplace_v2"
COIN_FLIP_SCHEMA_NAME = "coin_flip"
```

### Explanation of the event emission in the Move contract

In our Move contract (in `coin_flip/move/sources/coin_flip.move`), each user has an object associated with their account. The object has a `CoinFlipStats` resource on it that tracks the total number of wins and losses a user has and is in charge of emitting events.

```rust
// CoinFlipStats object/resource definition
#[resource_group_member(group = aptos_framework::object::ObjectGroup)]
struct CoinFlipStats has key {
    wins: u64,
    losses: u64,
    event_handle: EventHandle<CoinFlipEvent>,  //
    delete_ref: DeleteRef,
}

// event emission in `flip_coin`
fun flip_coin(
    user: &signer,
    prediction: bool,
    nonce: u64,
) acquires CoinFlipStats {
    // ...
    let (heads, correct_prediction) = flip(prediction, nonce);

    if (correct_prediction) {
        coin_flip_stats.wins = coin_flip_stats.wins + 1;
    } else {
        coin_flip_stats.losses = coin_flip_stats.losses + 1;
    };

    event::emit_event<CoinFlipEvent>(
        &mut coin_flip_stats.event_handle,
        CoinFlipEvent {
            prediction: prediction,
            result: heads,
            wins: coin_flip_stats.wins,
            losses: coin_flip_stats.losses,
        }
    );
}
```
The events emitted are of type `CoinFlipEvent`, shown below:
```rust
struct CoinFlipEvent has copy, drop, store {
    prediction: bool,     // true = heads, false = tails
    result: bool,
    wins: u64,
    losses: u64,
}
```

### Viewing and understanding how the event data is emitted and processed

When we submit a transaction that calls the `coin_flip` entry function, the indexer parses the events and records the data of each event that occurred in the transaction.

Within the `data` field of each `Event` type, we see the arbitrary event data emitted. We use this data to store the event data in our database.

The processor loops over each event in each transaction to process all event data. There are a *lot* of various types of events that can occur in a transaction- so we need to write a filtering function to deal with various events we don't want to store in our database.

This is the simple iterative structure for our event List:

```python
for event_index, event in enumerate(user_transaction.events):
    # Skip events that don't match our filter criteria
    if not CoinFlipProcessor.included_event_type(event.type_str):
        continue
```

where the `included_event_type` function is a static method in our `CoinFlipProcessor` class:

```python
@staticmethod
def included_event_type(event_type: str) -> bool:
    parsed_tag = event_type.split("::")
    module_address = parsed_tag[0]
    module_name = parsed_tag[1]
    event_type = parsed_tag[2]
    # Now we can filter out events that are not of type CoinFlipEvent
    # We can filter by the module address, module name, and event type
    # If someone deploys a different version of our contract with the same event type, we may want to index it one day.
    # So we could only check the event type instead of the full string
    # For our sake, check the full string
    return (
        module_address
        == "0xe57752173bc7c57e9b61c84895a75e53cd7c0ef0855acd81d31cb39b0e87e1d0"
        and module_name == "coin_flip"
        and event_type == "CoinFlipEvent"
    )
```

If you wanted to see the event data for yourself inside the processor loop, you could add something like this to your `processor.py` file:

```python
for event_index, event in enumerate(user_transaction.events):
    # Skip events that don't match our filter criteria
    if not CoinFlipProcessor.included_event_type(event.type_str):
        continue

    # ...

    # Load the data into a json object and then use/view it as a regular dictionary
    data = json.loads(event.data)
    print(json.dumps(data, indent=3))
```
In our case, a single event prints this out:


```json
{
    'losses': '49',
    'prediction': False,
    'result': True,
    'wins': '51'
}
```

So we'll get our data like this:

```python
prediction = bool(data["prediction"])
result = bool(data["result"])
wins = int(data["wins"])
losses = int(data["losses"])

# We have extra data to insert into the database, because we want to process our data.
# Calculate the total
win_percentage = wins / (wins + losses)
```

And then we add it to our event list with this:

```python
# Create an instance of CoinFlipEvent
event_db_obj = CoinFlipEvent(
    sequence_number=sequence_number,
    creation_number=creation_number,
    account_address=account_address,
    transaction_version=transaction_version,
    transaction_timestamp=transaction_timestamp,
    prediction=prediction,
    result=result,
    wins=wins,
    losses=losses,
    win_percentage=win_percentage,
    inserted_at=datetime.now(),
    event_index=event_index,  # when multiple events of the same type are emitted in a single transaction, this is the index of the event in the transaction
)
event_db_objs.append(event_db_obj)
```
### Creating your database model

Now that we know how we store our CoinFlipEvents in our database, let's go backwards a bit and clarify how we *create* this model for the database to use.

We need to structure the `CoinFlipEvent` class in `models.py` to reflect the structure in our Move contract:

```python
class CoinFlipEvent(Base):
    __tablename__ = "coin_flip_events"
    __table_args__ = ({"schema": COIN_FLIP_SCHEMA_NAME},)

    sequence_number: BigIntegerPrimaryKeyType
    creation_number: BigIntegerPrimaryKeyType
    account_address: StringPrimaryKeyType
    prediction: BooleanType     # from (event.data["prediction"]
    result: BooleanType         # from (event.data["result"]
    wins: BigIntegerType        # from (event.data["wins"]
    losses: BigIntegerType      # from (event.data["losses"]
    win_percentage: NumericType # calculated from the above
    transaction_version: BigIntegerType
    transaction_timestamp: TimestampType
    inserted_at: InsertedAtType
    event_index: BigIntegerType
```

The unmarked fields are from the default event data for every event emitted on Aptos. The marked fields are specifically from the fields we calculated above.

The other fields, __tablename__ and __table_args__, are indications to the python SQLAlchemy library as to what database and schema name we are using.

## Running the indexer processor

Now that we have our configuration files and our database and the python database model set up, we can run our processor.

Navigate to the `python` directory of your indexer repository:

```shell
cd ~/indexer/python
```

And then run the following command:

```shell
poetry run python -m processors.main -c processors/coin_flip/config.yaml
```

If you're processing events correctly, the events should now show up in your database.
