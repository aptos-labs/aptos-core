//# init --addresses Alice=0xf75daa73fc071f93593335eb9033da804777eb94491650dd3f095ce6f778acb6 Bob=0x9c3b634ac05d0af393e0f93b9b19b61e7cac1c519f566276aa0c6fd15dac12aa
//#      --private-keys Alice=56a26140eb233750cd14fb168c3eb4bd0782b099cde626ec8aff7f3cceb6364f Bob=952aaf3a98a27903dd078d76fc9e411740d2ae9dd9ecb87b96c7cd6b791ffc69
//#      --initial-coins 10000

//# run --script --signers Alice  --args @Bob 100
script {
    use velor_framework::coin;
    use velor_framework::velor_coin::VelorCoin;

    fun main(sender: &signer, receiver: address, amount: u64) {
        coin::transfer<VelorCoin>(sender, receiver, amount);
    }
}

//# view --address Bob  --resource 0x1::coin::CoinStore<0x1::velor_coin::VelorCoin>
