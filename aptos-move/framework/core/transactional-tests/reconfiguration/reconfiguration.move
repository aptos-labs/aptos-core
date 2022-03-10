//# init --validators Vivian Valentina --parent-vasps Alice

//# block --proposer Vivian --time 2

// Reconfiguration can only be invoked by the diem root.
//# run --admin-script --signers DiemRoot Vivian --show-events
script {
use DiemFramework::Reconfiguration;

fun main(_dr: signer, vv: signer) {
    Reconfiguration::reconfigure(&vv);
}
}


// Reconfiguration can only be invoked by the diem root.
//# run --admin-script --signers DiemRoot DiemRoot --show-events
script {
use DiemFramework::Reconfiguration;

fun main(dr: signer, _dr2: signer) {
    Reconfiguration::reconfigure(&dr);
    Reconfiguration::reconfigure(&dr);
}
}

//# block --proposer Vivian --time 3

// Make sure two reconfigurations will only trigger one reconfiguration event.
//# run --admin-script --signers DiemRoot DiemRoot --show-events
script {
use DiemFramework::Reconfiguration;

fun main(dr: signer, _dr2: signer) {
    Reconfiguration::reconfigure(&dr);
}
}
