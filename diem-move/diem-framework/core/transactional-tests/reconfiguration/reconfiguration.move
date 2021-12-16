//# init --validators Vivian Valentina --parent-vasps Alice

//# block --proposer Vivian --time 2

// Reconfiguration can only be invoked by the diem root.
//# run --admin-script --signers DiemRoot Vivian --show-events
script {
use DiemFramework::DiemConfig;

fun main(_dr: signer, vv: signer) {
    DiemConfig::reconfigure(&vv);
}
}


// Reconfiguration can only be invoked by the diem root.
//# run --admin-script --signers DiemRoot DiemRoot --show-events
script {
use DiemFramework::DiemConfig;

fun main(dr: signer, _dr2: signer) {
    DiemConfig::reconfigure(&dr);
    DiemConfig::reconfigure(&dr);
}
}

//# block --proposer Vivian --time 3

// Make sure two reconfigurations will only trigger one reconfiguration event.
//# run --admin-script --signers DiemRoot DiemRoot --show-events
script {
use DiemFramework::DiemConfig;

fun main(dr: signer, _dr2: signer) {
    DiemConfig::reconfigure(&dr);
}
}
