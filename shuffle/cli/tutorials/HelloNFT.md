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

> let args: any[] = [contentUri];

> let txn = await helpers.invokeScriptFunction(scriptFunction, typeArguments, args);

> txn = await devapi.waitForTransaction(txn.hash);
{
  type: "user_transaction",
  version: "264",
  hash: "0x55cbf81f080018a8a1f6a572cf75ddc64ad57d414791ddcef1dc803675b12996",
  state_root_hash: "0xf9b9a4f2a0d05988e91b11e4f8bc3d870ec64f333ec836dbd726ed60c04b5ee4",
  event_root_hash: "0x414343554d554c41544f525f504c414345484f4c4445525f4841534800000000",
  gas_used: "78",
  success: true,
  vm_status: "Executed successfully",
  sender: "0x6215bb1c111e8943794f21eb7e559656",
  sequence_number: "5",
  max_gas_amount: "1000000",
  gas_unit_price: "0",
  gas_currency_code: "XUS",
  expiration_timestamp_secs: "99999999999",
  payload: {
    type: "script_function_payload",
    function: "0x6215bb1c111e8943794f21eb7e559656::MyNFT::mint_nft",
    type_arguments: [],
    arguments: [ "0x68747470733a2f2f706c6163656b697474656e2e636f6d2f3230302f333030" ]
  },
  signature: {
    type: "ed25519_signature",
    public_key: "0x1fc99baab80ddc62a30cc4a780198be946d4fe1c5ef0320e2d665e88aebcddb0",
    signature: "0x41f578f7d9f3f05c9b4064f017d86ae08b479112cb7ebc6c7d9163fbd36cd7c2d5ae711a0f0d74b24069c70e80fc15c017..."
  },
  events: []
}

> let resource = await devapi.resourcesWithName("NFTStandard");

> console.log(resource);
[
  {
    type: "0x6215bb1c111e8943794f21eb7e559656::NFTStandard::NFTCollection<0x6215bb1c111e8943794f21eb7e559656::M...",
    data: { nfts: [ [Object] ] }
  },
  {
    type: "0x6215bb1c111e8943794f21eb7e559656::NFTStandard::NFTCollection<0x6215bb1c111e8943794f21eb7e559656::T...",
    data: { nfts: [ [Object] ] }
  }
]

> resource[0].type
"0x6215bb1c111e8943794f21eb7e559656::NFTStandard::NFTCollection<0x6215bb1c111e8943794f21eb7e559656::MyNFT::MyNFT>"

> resource[0].data.nfts
[
  {
    content_uri: "0x68747470733a2f2f706c6163656b697474656e2e636f6d2f3230302f333030",
    id: { id: { addr: "0x6215bb1c111e8943794f21eb7e559656", creation_num: "5" } },
    type: { dummy_field: false }
  }
]
```

## Transfer MyNFT

We will use e2e testing to try transferring MyNFT from one account to another.
In `nft/e2e`, create a new file `nft.test.ts`.
