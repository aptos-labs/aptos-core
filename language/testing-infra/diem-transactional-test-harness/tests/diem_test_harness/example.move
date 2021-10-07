//# init --addresses A=0x4777eb94491650dd3f095ce6f778acb6



// To create new accounts for testing, you need to call the account creation scripts in
// `module AccountCreationScripts`.
//
//# run --signers DiemRoot
//#     --private-key 1b8c20cde2dbb43cd3c709b290ac50dcd2be2a87a3a24544b5a5109bc76ea7fb
//#     --args 0 0x4777eb94491650dd3f095ce6f778acb6 x"f75daa73fc071f93593335eb9033da80" x"40"
//#     -- 0x1::AccountCreationScripts::create_validator_operator_account



// To publish a module, sign the transaction using the private key associated with the address
// under which the module will be published.
//
//# publish --private-key 56a26140eb233750cd14fb168c3eb4bd0782b099cde626ec8aff7f3cceb6364f
module A::M {
    public(script) fun foo() {
        abort 42
    }
}



// In order to get authenticated and run a transaction script successfully, you *must* provide
// the correct private key that corresponds to the address and auth key prefix used to create
// the account, as an additional argument to the run command.
//
// Note: regular transaction scripts are no longer allowed. This is to be consistent with the
// the real world use cases. If you want to execute custom code as a normal user, wrap your
// code in a script function, publish it, and then call the script function instead.
//
//# run --signers A
//#     --private-key 56a26140eb233750cd14fb168c3eb4bd0782b099cde626ec8aff7f3cceb6364f
//#     -- 0x4777eb94491650dd3f095ce6f778acb6::M::foo



// Use the view command to inspect on-chain resources.
//
//# view --address A --resource 0x1::DiemAccount::DiemAccount



// To send an admin script transaction, append the `--admin-script` option to the run command.
// Admin scripts do not require a private key -- they are signed using the (test) genesis key pair.
//
//# run --signers A
//#     --admin-script
script {
    fun main() {}
}
