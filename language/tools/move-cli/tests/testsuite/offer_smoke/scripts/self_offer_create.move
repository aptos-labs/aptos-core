// Script 1, seder: alice
script {
use Std::Offer;
use Std::Signer;

// Create a self offer containing a u64
fun self_offer_create(account: signer) {
    let sender = Signer::address_of(&account);
    Offer::create(&account, 7, @0xA11CE);
    assert!(Offer::address_of<u64>(sender) == @0xA11CE , 100);
    assert!(Offer::redeem(&account, @0xA11CE) == 7, 101);
}
}
