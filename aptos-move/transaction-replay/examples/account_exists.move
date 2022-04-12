script {
// Usage: bisect-transaction <Path_to_this_file> <Account_to_query> <begin_version> <end_version>
// Find the first version where the account is created.
use AptosFramework::Account;
use Std::Signer;
fun main(_dr_account: signer, sender: signer) {
    let addr = Signer::address_of(&sender);
    if(Account::exists_at(addr)) {
        abort 1
    };
    return
}
}
