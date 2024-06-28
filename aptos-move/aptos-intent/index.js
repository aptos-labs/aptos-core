const rust = import("./pkg");

rust
  .then((m) => {
    return  m.BatchedFunctionCallBuilder.single_signer().load_module("testnet","0x0000000000000000000000000000000000000000000000000000000000000001 account").then((data) => {
      console.log(data);
    });
  })
  .catch(console.error);
