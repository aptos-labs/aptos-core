import ffi from 'ffi-napi';
import ref from 'ref-napi';


const lib = ffi.Library('../../../../../target/release/libaptos', {
    print_from_ts: [ref.types.CString, []],
    run_aptos_from_ts: [ref.types.CString, ['string']],
});

const args = ['aptos', 'node', 'run-local-testnet', '--with-faucet'];
const args1 = ['aptos', 'info'];

(async () => {
    console.log("Running print from typescript");
    const result = lib.print_from_ts();
    console.log(result);
    console.log("Finish");

    console.log("Running aptos from typescript");
    const result2 = lib.run_aptos_from_ts(args1.join(' '));
    console.log(`Result: ${result2}`);
    console.log("Finish aptos");
})();
