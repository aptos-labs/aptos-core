use aptos_telemetry_service::index::routes;
use serde::{Deserialize, Serialize};
use tokio::time;
use aptos_types::on_chain_config::ValidatorSet;
use aptos_crypto::{x25519, traits::*};
use aptos_telemetry_service::{rest_client::RestClient,validator_cache::ValidatorCache};
use aptos_telemetry_service::context::Context;


#[tokio::main]
async fn main() {
    let mut rng = rand::thread_rng();
    let private_key = x25519::PrivateKey::generate(&mut rng);

    let api_url = String::from("https://devnet.aptoslabs.com");
    let rest_client = RestClient::new(api_url.clone());
    let cache = ValidatorCache::new(rest_client);

    let mut interval = time::interval(time::Duration::from_secs(60));
    let c_cache = cache.clone();
    tokio::spawn(async move {
        loop {
            println!("updating cache");
            c_cache.clone().update().await;
            interval.tick().await;
        }
    });

    let context = Context::new(private_key, cache.clone());

    warp::serve(routes(context)).run(([127, 0, 0, 1], 8000)).await;
}

#[derive(Serialize,Deserialize)]
pub(crate) struct APIResponse {
    resource_type: String,
    data: ValidatorSet
}