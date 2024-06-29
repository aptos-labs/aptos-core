const { BatchArgument } = require("./pkg");

const rust = import("./pkg");

rust
  .then((m) => {
    let builder = m.BatchedFunctionCallBuilder.single_signer();
    return builder.load_module("testnet", "0x0000000000000000000000000000000000000000000000000000000000000001 aptos_account")
      .then(() => {
        builder.add_batched_call(
          "0x0000000000000000000000000000000000000000000000000000000000000001 aptos_account",
          "transfer",
          [],
          [BatchArgument.new_bytes(Buffer.from( "0x77201cdd810bbd83fad933ee490104e384579b937573a949ebc65264da243a12", "hex")), BatchArgument.new_bytes(Buffer.from("1"))]
        ).then(console.log)
      })
      .catch(console.error);
  })
  .catch(console.error);
