const ffi = require("ffi-napi");

const lib = ffi.Library("../../../../../target/release/libaptos", {
  run_aptos_sync: ["char *", ["string"]], // run the aptos CLI synchronously
  run_aptos_async: ["char *", ["string"]], // run the aptos CLI asynchronously
  free_cstring: ["void", ["char *"]], // free the return pointer memory allocated by the aptos CLI
});

const args_run_local_testnet = ["aptos", "node", "run-local-testnet", "--with-faucet"];
const args_aptos_info = ["aptos", "info"];

(async () => {
  console.log("Running aptos CLI from Typescript");
  const aptos_info = lib.run_aptos_sync(args_aptos_info.join(" "));
  const run_local_testnet = lib.run_aptos_async(args_run_local_testnet.join(" "));
  try {
    console.log(`Aptos Info: ${aptos_info.readCString()}`);
    console.log(`Run Local Testnet: ${run_local_testnet.readCString()}`);
  } catch (error) {
    console.error(error);
  } finally {
    // free the string pointer memory allocated by the aptos CLI
    lib.free_cstring(aptos_info);
    lib.free_cstring(run_local_testnet);
  }

  console.log("Finish aptos CLI");
})();
