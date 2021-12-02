# Hello NFT Tutorial

Welcome to Hello NFT tutorial!

The goal of this tutorial is to 1) create your own NFT, 2) mint one into your
account, and 3) transfer your NFT to another account.

## Create a New Project

To start a new Move project, run `shuffle new nft`, which will create a project
directory with the given path name and a template to start Move development.

Inside `nft/main/sources/nft` there should be three move modules:
`NFTStandard.move`, `NFTTests.move`, and `TestNFT.move`.

We have provided `NFTStandard.move` as a generic NFT class that can be extended
to create your own NFT class.

## Create Your Own NFT Class

Inside `nft/main/sources/nft`, create a new move file `MyNFT.move`.

```
module Sender::MyNFT {
    use Sender::NFTStandard;
    use Std::Signer;

    struct MyNFT has drop, store {}

}
```

## Add Minting Functionality

Here we have imported `NFTStandard` and created a new NFT type `MyNFT`, which
will be the `NFTType` we use when calling `NFTStandard` methods.

```
module Sender::MyNFT {
    use Sender::NFTStandard;
    use Std::Signer;

    struct MyNFT has drop, store {}

    public(script) fun mint_nft(account: signer, content_uri: vector<u8>) {
        NFTStandard::initialize<MyNFT>(&account);
        let token = MyNFT{};
        let instance = NFTStandard::create<MyNFT>(
            &account,
            token,
            content_uri,
        );
        NFTStandard::add(Signer::address_of(&account), instance);
    }
}
```

Now we add a wrapper for minting MyNFT, which 1) initializes the nft collection
of type `MyNFT`, 2) creates a `MyNFT` and 3) adds the created nft into the nft
collection resource.

## Publish

Now that you have a very simple NFT module, you can publish it to your network:

- Spin up your local node: `shuffle node`
- Create an account for publishing: `shuffle account`
- Deploy your module: `cd nft && shuffle deploy`

If deployed successfully, you should see MyNFT included in the list of published
modules:

```
Skipping Module: 00000000000000000000000000000001::Hash
Skipping Module: 00000000000000000000000000000001::Signer
Skipping Module: 00000000000000000000000000000001::Vector
Skipping Module: 00000000000000000000000000000001::Errors
Skipping Module: 00000000000000000000000000000001::BitVector
Skipping Module: 00000000000000000000000000000001::Capability
Skipping Module: 00000000000000000000000000000001::FixedPoint32
Skipping Module: 00000000000000000000000000000001::Option
Skipping Module: 00000000000000000000000000000001::BCS
Skipping Module: 00000000000000000000000000000001::GUID
Skipping Module: 00000000000000000000000000000001::Event
Deploying Module: 6215BB1C111E8943794F21EB7E559656::Message
Deploying Module: 6215BB1C111E8943794F21EB7E559656::NFTStandard
Deploying Module: 6215BB1C111E8943794F21EB7E559656::MyNFT
Deploying Module: 6215BB1C111E8943794F21EB7E559656::TestNFT
```

## Explore MyNFT with Console

You can now use shuffle console to interact with your MyNFT module! Start up
your console within your project folder: `shuffle console`

Let's try calling the `mint_nft` script function within the console.

```
> const contentUri = "https://placekitten.com/200/300"

> let scriptFunction: string = context.defaultUserContext.address + "::MyNFT::mint_nft";

> let typeArguments: string[] = [];

> let args: any[] = [move.Ascii(contentUri)];

> let txn = await helpers.invokeScriptFunction(scriptFunction, typeArguments, args);

> txn = await devapi.waitForTransaction(txn.hash);
{
  type: "user_transaction",
  version: "71",
  hash: "0x53996c3579ff6cd2d8f1b5599a32bbf812c262d19da3c97ea20237a3c69d14f6",
  state_root_hash: "0x41b4cb644efefaff1c52ec9b91823fbb59e5501222d82999825b0bed5463ab99",
  event_root_hash: "0x414343554d554c41544f525f504c414345484f4c4445525f4841534800000000",
  gas_used: "78",
  success: true,
  vm_status: "Executed successfully",
  sender: "0xe8eb92a7a924d5b16d012603a2294241",
  sequence_number: "4",
  max_gas_amount: "1000000",
  gas_unit_price: "0",
  gas_currency_code: "XUS",
  expiration_timestamp_secs: "99999999999",
  payload: {
    type: "script_function_payload",
    function: "0xe8eb92a7a924d5b16d012603a2294241::MyNFT::mint_nft",
    type_arguments: [],
    arguments: [ "0x68747470733a2f2f706c6163656b697474656e2e636f6d2f3230302f333030" ]
  },
  signature: {
    type: "ed25519_signature",
    public_key: "0xeb18d016abc18ffacd529c31401b2fec240c1c76b7fd95627f37d743f2ccf78c",
    signature: "0xf842733d9591f47948b0790d67bec19eac3621c05560c7a83382f9d60264c68d0597b7495d76250a57d2cb4dc37762099b..."
  },
  events: [],
  timestamp: "1638499244177902"
}

> let resource = await devapi.resourcesWithName("NFTStandard");

> console.log(resource);
[
  {
    type: "0xe8eb92a7a924d5b16d012603a2294241::NFTStandard::NFTCollection<0xe8eb92a7a924d5b16d012603a2294241::M...",
    data: { nfts: [ [Object] ] }
  }
]

> resource[0].type
"0xe8eb92a7a924d5b16d012603a2294241::NFTStandard::NFTCollection<0xe8eb92a7a924d5b16d012603a2294241::MyNFT::MyNFT>"

> resource[0].data.nfts
[
  {
    content_uri: "0x68747470733a2f2f706c6163656b697474656e2e636f6d2f3230302f333030",
    id: { id: { addr: "0xe8eb92a7a924d5b16d012603a2294241", creation_num: "4" } },
    type: { dummy_field: false }
  }
]

> await main.decodedNFTs()
[
  {
    id: { id: { addr: "0xe8eb92a7a924d5b16d012603a2294241", creation_num: "4" } },
    content_uri: "https://placekitten.com/200/300"
  }
]
```

## Transfer MyNFT

Now we want to use `NFTStandard::transfer` to transfer your minted NFT into
another account. We will use e2e testing framework this time, so that you can
test the entire flow.

In `nft/e2e`, create a new file `nft.test.ts`. Set up your imports and a test
function.

```
import {
  assert,
  assertEquals,
} from "https://deno.land/std@0.85.0/testing/asserts.ts";
import * as devapi from "../main/devapi.ts";
import * as main from "../main/mod.ts";
import * as context from "../main/context.ts";
import * as helpers from "../main/helpers.ts";
import * as mv from "../main/move.ts";

Deno.test("Test NFTs", async () => {

});
```

Next you want to initialize the `nft_collection` resource of type MyNFT in both
sender and receiver accounts.

```
Deno.test("Test NFTs", async () => {
    // First user is initialized by default, but second account must be initialized within the test
    const secondUserContext = context.UserContext.fromEnv("test");

    // Initialize nft_collection resource for sender
    let senderInitializeTxn = await main.initializeNFTScriptFunction(
        "MyNFT", context.defaultUserContext, context.defaultUserContext.address
    );
    senderInitializeTxn = await devapi.waitForTransaction(
        senderInitializeTxn.hash,
    );

    // Initialize nft_collection resource for receiver
    let receiverInitializeTxn = await main.initializeNFTScriptFunction(
        "MyNFT", secondUserContext, context.defaultUserContext.address
    );
    receiverInitializeTxn = await devapi.waitForTransaction(
        receiverInitializeTxn.hash,
    );
    assert(senderInitializeTxn.success);
    assert(receiverInitializeTxn.success);
});
```

Let's try running `shuffle test e2e` to test the above code. This will run all
tests inside the `/nft/e2e` directory. You should see a test passed output:

```
running 1 test from file:///Users/sunmilee/diem/nft/e2e/nft.test.ts
test Test NFTs ... ok (4s)

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out (12s)
```

Now you can mint a NFT into the sender account resource similar to how we did it
in console above:

```
Deno.test("Test NFTs", async () => {
    ...

    // Choose a contentUri that the NFT will point to
    const contentUri = "https://placekitten.com/200/300";

    const mintScriptFunction: string = context.defaultUserContext.address + "::MyNFT::mint_nft";
    const mintTypeArguments: string[] = [];
    const mintArgs: any[] = [mv.Ascii(contentUri)];
    let txn = await helpers.invokeScriptFunctionForContext(
        context.defaultUserContext,
        mintScriptFunction,
        mintTypeArguments,
        mintArgs,
    );
    txn = await devapi.waitForTransaction(txn.hash);
    assert(txn.success);

    // main.decodedNFTs will fetch all NFTs of the address, decode its metadata, and display in json
    const nfts = await main.decodedNFTs(context.defaultUserContext.address);

    // Check the minted nft's content uri matches the one declared above
    assertEquals(nfts[0].content_uri, contentUri);
});
```

We can now transfer the minted NFT to the receiver we instantiated above:

```
Deno.test("Test NFTs", async () => {
    ...

    // Get the creator address and creation number of the NFT
    const creator = nfts[0].id.id.addr;
    const creationNum = nfts[0].id.id.creation_num;

    // Similar to mint, use invokeScriptFunctionForContext to call NFTStandard::transfer
    const transferScriptFunction: string = context.defaultUserContext.address + "::NFTStandard::transfer";
    const transferTypeArgument: string[] = [context.defaultUserContext.address + "::MyNFT::MyNFT"];
    const transferArgs: any[] = [mv.Address(secondUserContext.address), mv.Address(creator), mv.U64(creationNum)];
    let transferTxn = await helpers.invokeScriptFunctionForContext(
        context.defaultUserContext,
        transferScriptFunction,
        transferTypeArgument,
        transferArgs,
    );
    transferTxn = await devapi.waitForTransaction(transferTxn.hash);
    assert(transferTxn.success);

    // Check receiver has the nft
    const receiverNFTs = await main.decodedNFTs(secondUserContext.address);
    assertEquals(receiverNFTs[0].content_uri, contentUri);
    // Check sender nft_collection is empty
    const senderNFTs = await main.decodedNFTs(
        context.defaultUserContext.address,
    );
    assert(senderNFTs.length === 0);
});
```

Run `shuffle test e2e` to try the full transfer test!
