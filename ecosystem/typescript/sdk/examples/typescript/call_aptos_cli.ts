const ffi = require('ffi-napi');

const lib = ffi.Library('../../../../../target/release/libaptos', {
    run_aptos_from_ts: ['char *', ['string']],
    free_cstring: ['void', ['char *']],
});

const args_run_local_testnet = ['aptos', 'node', 'run-local-testnet', '--with-faucet'];
const args_aptos_info = ['aptos', 'info'];

(async () => {
    console.log("Running aptos CLI from Typescript");
    const result2 = lib.run_aptos_from_ts(args_run_local_testnet.join(' '));
    try {
        console.log(`Result: ${result2.readCString()}`);
    } catch (error) {
        console.error(error);
    } finally {
        lib.free_cstring(result2);
    }

    console.log("Finish aptos CLI");
})();
