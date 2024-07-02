const rust = import("./pkg"); 
const {Serializer, Ed25519Account,TransactionPayloadScript, Deserializer, RawTransaction, ChainId, Aptos, AptosConfig, SimpleTransaction, } = require("@wgb5445/aptos-labs-ts-sdk");
const {Buffer} = require("buffer");
let account = Ed25519Account.fromDerivationPath({mnemonic: "divide rule mad goose wolf grab cliff milk visit tag floor join", path: "m/44'/637'/0'/0'/0'"});
rust
  .then(async (m) => {
    const config = new AptosConfig({ network: "custom" ,fullnode : "http://127.0.0.1:8080/v1",faucet: "http://127.0.0.1:8081",indexer: "http://127.0.0.1:8090"});
    const aptos = new Aptos(config);
    aptos.faucet.fundAccount({
      accountAddress: account.accountAddress,
      amount: 100000000
    })
    let builder = m.BatchedFunctionCallBuilder.single_signer();
    
    let ser = new Serializer();
    ser.serializeFixedBytes(Buffer.from("77201cdd810bbd83fad933ee490104e384579b937573a949ebc65264da243a12","hex"));
    let address = ser.toUint8Array();
    console.log(address)
    let ser2 = new Serializer();
    ser2.serializeU64(1);
    let amount = ser2.toUint8Array();
    return builder.load_module("testnet", "0x0000000000000000000000000000000000000000000000000000000000000001 aptos_account")
      .then(async () => {
      
        builder.add_batched_call(
          "0x0000000000000000000000000000000000000000000000000000000000000001 aptos_account",
          "transfer",
          [],
          [m.BatchArgument.new_signer(0),m.BatchArgument.new_bytes(address), m.BatchArgument.new_bytes(amount)]
        )
        let i = builder.generate_batched_calls();
        console.log(Buffer.from(i).toString("hex"))

        let rawTransaction = new RawTransaction(
          account.accountAddress,
          (await aptos.account.getAccountInfo(
            {
              accountAddress: account.accountAddress
            }
          )).sequence_number,
          TransactionPayloadScript.load(new Deserializer(i)),
          200000n,
          100n,
          50000000000000000n,
          new ChainId(4)
        );

         aptos.signAndSubmitTransaction(
        {
          signer: account,
          transaction:new SimpleTransaction(rawTransaction),
        }
        ).then((data)=>{
          console.log(data.hash);
          aptos.waitForTransaction({transactionHash:data.hash}).then((data)=>{
            console.log(data)
          })
        });
      })
      .catch(console.error);
  })
  .catch(console.error);
