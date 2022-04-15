use ed25519_dalek::{PublicKey, SecretKey};
use rand::{rngs::OsRng, Rng, SeedableRng};
// use reqwest;
use tiny_keccak::{Hasher, Sha3};


pub struct ArsAccount {
    signing_key: SecretKey,
}

impl ArsAccount{
    pub fn new(priv_key_bytes: Option<Vec<u8>>) -> ArsAccount{
        let signing_key = match priv_key_bytes{
            Some(key) => SecretKey::from_bytes(&key).unwrap(),
            None => SecretKey::generate(&mut rand::rngs::StdRng::from_seed(OsRng.gen()))
        };
        Self{ signing_key }
    }

    pub fn auth_key(&self) -> String{
        let mut sha3 = Sha3::v256();
        sha3.update(PublicKey::from(&self.signing_key).as_bytes());
        sha3.update(&vec![0u8]);
        let mut output = [0u8;32];
        sha3.finalize(&mut output);
        hex::encode(output)
    }
    
    pub fn address(&self) ->  String{
        self.auth_key()
    }

    pub fn pub_key(&self) -> String{
        hex::encode(PublicKey::from(&self.signing_key).as_bytes())
    }

}


//
// #[derive(Clone)]
// pub struct RestClient {
//     url: String,
// }
//
//
// impl RestClient{
//     pub fn new(url: String) -> Self{
//         Self{ url }
//     }
// }
//
// #[derive(Clone)]
// pub struct FaucetClient {
//     url: String,
//     rest_client: RestClient,
// }
//
// impl FaucetClient {
//     pub fn new(url: String, rest_client: RestClient) -> Self{
//         Self{
//             url,
//             rest_client,
//         }
//     }
//
//     // pub fn fund_account(&self, auth_key: &str, amount: u64){
//     //     let req = reqwest::blocking::Client::new().post(
//     //         format!(
//     //             "{}/mint?amount={}&auth_key={}",
//     //             self.url, amount, auth_key
//     //         )
//     //     ).send().unwrap();
//     //     if req.status != 200 {
//     //         assert_eq!(
//     //             req.status,
//     //             200,
//     //             "{}",
//     //             res.text().unwrap_or("".to_string()),
//     //         );
//     //     }
//     //     for txn_hash in res.json::<serde_json::Value>().unwrap().as_array().unwrap() {
//     //         self.rest_client
//     //             .wait_for_transaction(txn_hash.as_str().unwrap())
//     //     }
//     // }
// }