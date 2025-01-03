// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::server_args::ServerConfig;
use crate::{
    bypasser::{Bypasser, BypasserConfig},
    checkers::{CaptchaManager, Checker, CheckerConfig, CheckerTrait},
    endpoints::{
        build_openapi_service, convert_error, mint, BasicApi, CaptchaApi, FundApi,
        FundApiComponents,
    },
    funder::{ApiConnectionConfig, FunderConfig, MintFunderConfig, TransactionSubmissionConfig},
    middleware::middleware_log,
};
use anyhow::{anyhow, Context, Result};
use aptos_config::keys::ConfigKey;
use aptos_faucet_metrics_server::{run_metrics_server, MetricsServerConfig};
use aptos_logger::info;
use aptos_sdk::{
    crypto::ed25519::Ed25519PrivateKey,
    types::{account_config::aptos_test_root_address, chain_id::ChainId},
};
use clap::Parser;
use futures::{channel::oneshot::Sender as OneShotSender, lock::Mutex};
use poem::{http::Method, listener::TcpAcceptor, middleware::Cors, EndpointExt, Route, Server};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use std::{fs::File, io::BufReader, path::PathBuf, pin::Pin, str::FromStr, sync::Arc};
use tokio::{net::TcpListener, sync::Semaphore, task::JoinSet};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct HandlerConfig {
    /// Whether we should return helpful errors.
    pub use_helpful_errors: bool,

    /// Whether we should return rejections the moment a Checker returns any,
    /// or should instead run through all Checkers first. Generally prefer
    /// setting this to true, as it is less work on the tap, but setting it
    /// to false does give the user more immediate information.
    pub return_rejections_early: bool,

    /// The maximum number of requests the tap instance should handle at once.
    /// This allows the tap to avoid overloading its Funder, as well as to
    /// signal to a healthchecker that it is overloaded (via `/`).
    pub max_concurrent_requests: Option<usize>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RunConfig {
    /// API server config.
    pub server_config: ServerConfig,

    /// Metrics server config.
    metrics_server_config: MetricsServerConfig,

    /// Configs for any Bypassers we might want to enable.
    bypasser_configs: Vec<BypasserConfig>,

    /// Configs for any Checkers we might want to enable.
    checker_configs: Vec<CheckerConfig>,

    /// Config for the Funder component.
    funder_config: FunderConfig,

    /// General args for the runner / handler.
    handler_config: HandlerConfig,
}

impl RunConfig {
    pub async fn run(self) -> Result<()> {
        self.run_impl(None).await
    }

    pub async fn run_and_report_port(self, port_tx: OneShotSender<u16>) -> Result<()> {
        self.run_impl(Some(port_tx)).await
    }

    async fn run_impl(self, port_tx: Option<OneShotSender<u16>>) -> Result<()> {
        info!("Running with config: {:#?}", self);

        // Set whether we should use useful errors.
        // If it's already set, then we'll carry on
        #[cfg(not(test))]
        let _ = crate::endpoints::USE_HELPFUL_ERRORS.set(self.handler_config.use_helpful_errors);

        let concurrent_requests_semaphore = self
            .handler_config
            .max_concurrent_requests
            .map(|v| Arc::new(Semaphore::new(v)));

        // Build Funder.
        let funder = self
            .funder_config
            .build()
            .await
            .context("Failed to build Funder")?;

        // Build basic API.
        let basic_api = BasicApi {
            concurrent_requests_semaphore: concurrent_requests_semaphore.clone(),
            funder: funder.clone(),
        };

        // Create a CaptchaManager.
        let captcha_manager = Arc::new(Mutex::new(CaptchaManager::new()));

        // Build Bypassers.
        let mut bypassers: Vec<Bypasser> = Vec::new();
        for bypasser_config in &self.bypasser_configs {
            let bypasser = bypasser_config.clone().build().with_context(|| {
                format!("Failed to build Bypasser with args: {:?}", bypasser_config)
            })?;
            bypassers.push(bypasser);
        }

        // Create a periodic task manager.
        let mut join_set = JoinSet::new();

        // Build Checkers and let them spawn tasks on the periodic task
        // manager if they want.
        let mut checkers: Vec<Checker> = Vec::new();
        for checker_config in &self.checker_configs {
            let checker = checker_config
                .clone()
                .build(captcha_manager.clone())
                .await
                .with_context(|| {
                    format!("Failed to build Checker with args: {:?}", checker_config)
                })?;
            checker.spawn_periodic_tasks(&mut join_set);
            checkers.push(checker);
        }

        // Sort Checkers by cost, where lower numbers is lower cost, and lower
        // cost Checkers are at the start of the vec.
        checkers.sort_by_key(|a| a.cost());

        // Using those, build the fund API components.
        let fund_api_components = Arc::new(FundApiComponents {
            bypassers,
            checkers,
            funder,
            return_rejections_early: self.handler_config.return_rejections_early,
            concurrent_requests_semaphore,
        });

        let fund_api = FundApi {
            components: fund_api_components.clone(),
        };

        // Build the CaptchaApi.
        let mut tap_captcha_api_enabled = false;
        for checker in &self.checker_configs {
            if let CheckerConfig::TapCaptcha(_) = checker {
                tap_captcha_api_enabled = true;
                break;
            }
        }
        let captcha_api = CaptchaApi {
            enabled: tap_captcha_api_enabled,
            captcha_manager,
        };

        let api_service = build_openapi_service(basic_api, captcha_api, fund_api);
        let spec_json = api_service.spec_endpoint();
        let spec_yaml = api_service.spec_endpoint_yaml();

        let cors = Cors::new()
            // To allow browsers to use cookies (for cookie-based sticky
            // routing in the LB) we must enable this:
            // https://stackoverflow.com/a/24689738/3846032
            .allow_credentials(true)
            .allow_methods(vec![Method::GET, Method::POST]);

        // Collect futures that should never end.
        let mut main_futures: Vec<Pin<Box<dyn futures::Future<Output = Result<()>> + Send>>> =
            Vec::new();

        // Create a future for the metrics server.
        if !self.metrics_server_config.disable {
            main_futures.push(Box::pin(async move {
                run_metrics_server(self.metrics_server_config.clone())
                    .await
                    .context("Metrics server ended unexpectedly")
            }));
        }

        let listener = TcpListener::bind((
            self.server_config.listen_address.clone(),
            self.server_config.listen_port,
        ))
        .await?;
        let port = listener.local_addr()?.port();

        if let Some(tx) = port_tx {
            tx.send(port).map_err(|_| anyhow!("failed to send port"))?;
        }

        // Create a future for the API server.
        let api_server_future = Server::new_with_acceptor(TcpAcceptor::from_tokio(listener)?).run(
            Route::new()
                .nest(
                    &self.server_config.api_path_base,
                    Route::new()
                        .nest("", api_service)
                        .catch_all_error(convert_error),
                )
                .at("/spec.json", spec_json)
                .at("/spec.yaml", spec_yaml)
                .at("/mint", poem::post(mint.data(fund_api_components)))
                .with(cors)
                .around(middleware_log),
        );

        main_futures.push(Box::pin(async move {
            api_server_future
                .await
                .context("API server ended unexpectedly")
        }));

        // If there are any periodic tasks, create a future for retrieving
        // one so we know if any of them unexpectedly end.
        if !join_set.is_empty() {
            main_futures.push(Box::pin(async move {
                join_set.join_next().await.unwrap().unwrap()
            }));
        }

        // Wait for all the futures. We expect none of them to ever end.
        futures::future::select_all(main_futures)
            .await
            .0
            .context("One of the futures that were not meant to end ended unexpectedly")
    }

    /// Like `run` but manipulates the server config for a test environment.
    #[cfg(feature = "integration-tests")]
    pub async fn run_test(mut self, port: u16) -> Result<()> {
        self.server_config.listen_port = port;
        self.metrics_server_config.disable = true;
        self.run().await
    }

    /// Call this function to build a RunConfig to run a faucet alongside a node API
    /// run by the Aptos CLI.
    pub fn build_for_cli(
        api_url: Url,
        listen_address: String,
        listen_port: u16,
        funder_key: FunderKeyEnum,
        do_not_delegate: bool,
        chain_id: Option<ChainId>,
    ) -> Self {
        let (key_file_path, key) = match funder_key {
            FunderKeyEnum::KeyFile(key_file_path) => (key_file_path, None),
            FunderKeyEnum::Key(key) => (PathBuf::from_str("/dummy").unwrap(), Some(key)),
        };
        Self {
            server_config: ServerConfig {
                listen_address,
                listen_port,
                api_path_base: "".to_string(),
            },
            metrics_server_config: MetricsServerConfig {
                disable: true,
                listen_address: "0.0.0.0".to_string(),
                listen_port: 1,
            },
            bypasser_configs: vec![],
            checker_configs: vec![],
            funder_config: FunderConfig::MintFunder(MintFunderConfig {
                api_connection_config: ApiConnectionConfig::new(
                    api_url,
                    key_file_path,
                    key,
                    chain_id.unwrap_or_else(ChainId::test),
                ),
                transaction_submission_config: TransactionSubmissionConfig::new(
                    None,    // maximum_amount
                    None,    // maximum_amount_with_bypass
                    30,      // gas_unit_price_ttl_secs
                    None,    // gas_unit_price_override
                    500_000, // max_gas_amount
                    30,      // transaction_expiration_secs
                    35,      // wait_for_outstanding_txns_secs
                    false,   // wait_for_transactions
                ),
                mint_account_address: Some(aptos_test_root_address()),
                do_not_delegate,
            }),
            handler_config: HandlerConfig {
                use_helpful_errors: true,
                return_rejections_early: false,
                max_concurrent_requests: None,
            },
        }
    }
}

// This is just to make it a bit safer to express how you're providing the funder key.
pub enum FunderKeyEnum {
    KeyFile(PathBuf),
    Key(ConfigKey<Ed25519PrivateKey>),
}

#[derive(Clone, Debug, Parser)]
pub struct Run {
    #[clap(short, long, value_parser)]
    config_path: PathBuf,
}

impl Run {
    pub async fn run(&self) -> Result<()> {
        let run_config = self.get_run_config()?;
        run_config.run().await
    }

    pub fn get_run_config(&self) -> Result<RunConfig> {
        let file = File::open(&self.config_path).with_context(|| {
            format!(
                "Failed to load config at {}",
                self.config_path.to_string_lossy()
            )
        })?;
        let reader = BufReader::new(file);
        let run_config: RunConfig = serde_yaml::from_reader(reader).with_context(|| {
            format!(
                "Failed to parse config at {}",
                self.config_path.to_string_lossy()
            )
        })?;
        Ok(run_config)
    }
}

/// This is used for the run-simple command, which lets you run a faucet with a config
/// file, at the cost of less configurability.
#[derive(Clone, Debug, Parser)]
pub struct RunSimple {
    #[clap(flatten)]
    api_connection_config: ApiConnectionConfig,

    /// What address to listen on.
    #[clap(long, default_value = "0.0.0.0")]
    pub listen_address: String,

    /// What port to listen on.
    #[clap(long, default_value_t = 8081)]
    pub listen_port: u16,

    #[clap(long)]
    do_not_delegate: bool,
}

impl RunSimple {
    pub async fn run_simple(&self) -> Result<()> {
        let key = self
            .api_connection_config
            .get_key()
            .context("Failed to load private key")?;
        let run_config = RunConfig::build_for_cli(
            self.api_connection_config.node_url.clone(),
            self.listen_address.clone(),
            self.listen_port,
            FunderKeyEnum::Key(ConfigKey::new(key)),
            self.do_not_delegate,
            Some(self.api_connection_config.chain_id),
        );
        run_config.run().await
    }
}

// We hide these tests behind a feature flag because these are not standard unit tests,
// these are integration tests that rely on a variety of outside pieces such as a local
// testnet and a running Redis instance.
#[cfg(feature = "integration-tests")]
mod test {
    use super::*;
    use crate::{
        endpoints::{
            AptosTapError, AptosTapErrorCode, FundRequest, FundResponse, RejectionReasonCode,
        },
        helpers::get_current_time_secs,
    };
    use anyhow::{bail, Result};
    use aptos_sdk::{
        crypto::{ed25519::Ed25519PrivateKey, HashValue, PrivateKey, Uniform},
        types::{account_address::AccountAddress, transaction::authenticator::AuthenticationKey},
    };
    use once_cell::sync::OnceCell;
    use poem_openapi::types::{ParseFromJSON, ToJSON};
    use rand::{
        rngs::{OsRng, StdRng},
        Rng, SeedableRng,
    };
    use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, REFERER};
    use std::{collections::HashSet, io::Write, str::FromStr, time::Duration};
    use tokio::task::JoinHandle;

    // This is used to ensure the initialization function gets called only once.
    static INIT: std::sync::Once = std::sync::Once::new();

    // This is used to prevent certain tests from running concurrently in some
    // critical sections, e.g. for server startup.
    static MUTEX: OnceCell<Mutex<()>> = OnceCell::new();

    fn init() {
        INIT.call_once(|| {
            crate::endpoints::USE_HELPFUL_ERRORS
                .set(true)
                .expect("OnceCell somehow already set");
            MUTEX
                .set(Mutex::new(()))
                .expect("OnceCell somehow already set");
        });
    }

    fn get_root_endpoint(port: u16) -> String {
        format!("http://127.0.0.1:{}", port)
    }

    fn get_fund_endpoint(port: u16) -> String {
        format!("{}/fund", get_root_endpoint(port))
    }

    async fn start_server(config_content: &'static str) -> Result<(u16, JoinHandle<Result<()>>)> {
        // Load config.
        let run_config: RunConfig =
            serde_yaml::from_str(config_content).context("Failed to parse config content")?;

        // Spawn server.
        let runtime_handle = tokio::runtime::Handle::current();
        let port = aptos_config::utils::get_available_port();
        let join_handle = runtime_handle.spawn(async move { run_config.run_test(port).await });

        // Wait for the server to startup.
        let startup_timeout_secs = 30;
        for i in 0..startup_timeout_secs {
            match reqwest::get(get_root_endpoint(port)).await {
                Ok(_) => break,
                Err(e) => {
                    if i == startup_timeout_secs - 1 {
                        let msg = if join_handle.is_finished() {
                            format!("Server failed on startup: {:#?}", join_handle.await)
                        } else {
                            "Server was still starting up".to_string()
                        };
                        bail!(
                            "Server didn't come up within given timeout: {:#?} {}",
                            e,
                            msg
                        );
                    }
                },
            }
            if join_handle.is_finished() {
                bail!(
                    "Server returned error while starting up: {:#?}",
                    join_handle.await
                );
            }
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
        Ok((port, join_handle))
    }

    fn make_list_file(filename: &str, items: &[&str]) -> Result<()> {
        let mut file = File::create(filename)?;
        for item in items {
            writeln!(file, "{}", item)?;
        }
        Ok(())
    }

    fn make_auth_tokens_file(auth_tokens: &[&str]) -> Result<()> {
        make_list_file("/tmp/auth_tokens.txt", auth_tokens)
    }

    fn make_ip_allowlist(ip_ranges: &[&str]) -> Result<()> {
        make_list_file("/tmp/ip_allowlist.txt", ip_ranges)
    }

    fn make_ip_blocklist(ip_ranges: &[&str]) -> Result<()> {
        make_list_file("/tmp/ip_blocklist.txt", ip_ranges)
    }

    fn make_referer_blocklist_file(referers: &[&str]) -> Result<()> {
        make_list_file("/tmp/referer_blocklist.txt", referers)
    }

    fn get_fund_request(amount: Option<u64>) -> FundRequest {
        FundRequest {
            amount,
            address: Some(AccountAddress::random().to_string()),
            ..Default::default()
        }
    }

    async fn unwrap_reqwest_result(
        result: Result<reqwest::Response, reqwest::Error>,
    ) -> Result<reqwest::Response> {
        let result = result?;
        if result.status() != reqwest::StatusCode::OK {
            bail!(
                "Request failed with status code: {} {}",
                result.status(),
                result.text().await?
            );
        }
        Ok(result)
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_bypassers() -> Result<()> {
        init();
        make_auth_tokens_file(&["test_token"])?;
        make_ip_allowlist(&[])?;
        let config_content = include_str!("../../../configs/testing_bypassers.yaml");
        let (port, _handle) = start_server(config_content).await?;

        // See that a request that should fail (in this case because it is
        // missing the magic headers) succeeds because it passes an auth
        // token in the bypass list.
        unwrap_reqwest_result(
            reqwest::Client::new()
                .post(get_fund_endpoint(port))
                .body(get_fund_request(Some(10)).to_json_string())
                .header(CONTENT_TYPE, "application/json")
                .header(AUTHORIZATION, "Bearer test_token")
                .send()
                .await,
        )
        .await?;

        // See that it does fail if we don't pass the auth token.
        assert!(unwrap_reqwest_result(
            reqwest::Client::new()
                .post(get_fund_endpoint(port))
                .body(get_fund_request(Some(10)).to_json_string())
                .header(CONTENT_TYPE, "application/json")
                .send()
                .await
        )
        .await
        .is_err());

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_checkers() -> Result<()> {
        init();
        make_ip_blocklist(&[])?;
        make_auth_tokens_file(&["test_token"])?;
        make_referer_blocklist_file(&["https://mysite.com"])?;
        let config_content = include_str!("../../../configs/testing_checkers.yaml");
        let (port, _handle) = start_server(config_content).await?;

        // Assert that a normal request fails due to a rejection.
        let response = reqwest::Client::new()
            .post(get_fund_endpoint(port))
            .body(get_fund_request(Some(10)).to_json_string())
            .header(CONTENT_TYPE, "application/json")
            .send()
            .await?;
        let aptos_error = AptosTapError::parse_from_json_string(&response.text().await?)
            .expect("Failed to read response as AptosError");
        assert!(!aptos_error.rejection_reasons.is_empty());

        // Assert that a request that passes all the configured checkers passes.
        unwrap_reqwest_result(
            reqwest::Client::new()
                .post(get_fund_endpoint(port))
                .body(get_fund_request(Some(10)).to_json_string())
                .header(CONTENT_TYPE, "application/json")
                .header(AUTHORIZATION, "Bearer test_token")
                .header("what_wallet_my_guy", "the_wallet_that_rocks")
                .send()
                .await,
        )
        .await?;

        // Assert that the magic header and auth token checkers work.
        let response = reqwest::Client::new()
            .post(get_fund_endpoint(port))
            .body(get_fund_request(Some(10)).to_json_string())
            .header(CONTENT_TYPE, "application/json")
            .header(AUTHORIZATION, "Bearer wrong_token")
            .header("what_wallet_my_guy", "some_other_wallet")
            .send()
            .await?;
        let aptos_error = AptosTapError::parse_from_json_string(&response.text().await?)
            .expect("Failed to read response as AptosError");
        let rejection_reason_codes: HashSet<RejectionReasonCode> = aptos_error
            .rejection_reasons
            .into_iter()
            .map(|r| r.get_code())
            .collect();
        assert!(rejection_reason_codes.contains(&RejectionReasonCode::MagicHeaderIncorrect));
        assert!(rejection_reason_codes.contains(&RejectionReasonCode::AuthTokenInvalid));

        // Assert that the referer blocklist checker works.
        let response = reqwest::Client::new()
            .post(get_fund_endpoint(port))
            .body(get_fund_request(Some(10)).to_json_string())
            .header(CONTENT_TYPE, "application/json")
            .header(REFERER, "https://mysite.com")
            .send()
            .await?;
        let aptos_error = AptosTapError::parse_from_json_string(&response.text().await?)
            .expect("Failed to read response as AptosError");
        let rejection_reason_codes: HashSet<RejectionReasonCode> = aptos_error
            .rejection_reasons
            .into_iter()
            .map(|r| r.get_code())
            .collect();
        assert!(rejection_reason_codes.contains(&RejectionReasonCode::RefererBlocklisted));

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_redis_ratelimiter() -> Result<()> {
        // Assert that a localnet is alive.
        let aptos_node_api_client = aptos_sdk::rest_client::Client::new(
            reqwest::Url::from_str("http://127.0.0.1:8080").unwrap(),
        );
        aptos_node_api_client
            .get_index_bcs()
            .await
            .context("Localnet API couldn't be reached at port 8080, have you started one?")?;

        init();
        let config_content = include_str!("../../../configs/testing_redis.yaml");
        let (port, _handle) = start_server(config_content).await?;

        // Assert that the first 3 requests work.
        unwrap_reqwest_result(
            reqwest::Client::new()
                .post(get_fund_endpoint(port))
                .body(get_fund_request(Some(10)).to_json_string())
                .header(CONTENT_TYPE, "application/json")
                .header(AUTHORIZATION, "Bearer test_token")
                .header("what_wallet_my_guy", "the_wallet_that_rocks")
                .send()
                .await,
        )
        .await?;
        unwrap_reqwest_result(
            reqwest::Client::new()
                .post(get_fund_endpoint(port))
                .body(get_fund_request(Some(10)).to_json_string())
                .header(CONTENT_TYPE, "application/json")
                .header(AUTHORIZATION, "Bearer test_token")
                .header("what_wallet_my_guy", "the_wallet_that_rocks")
                .send()
                .await,
        )
        .await?;
        unwrap_reqwest_result(
            reqwest::Client::new()
                .post(get_fund_endpoint(port))
                .body(get_fund_request(Some(10)).to_json_string())
                .header(CONTENT_TYPE, "application/json")
                .header(AUTHORIZATION, "Bearer test_token")
                .header("what_wallet_my_guy", "the_wallet_that_rocks")
                .send()
                .await,
        )
        .await?;

        // But the fourth does not, specifically with a 429 and the correct
        // rejection reasons in the body.
        let response = reqwest::Client::new()
            .post(get_fund_endpoint(port))
            .body(get_fund_request(Some(10)).to_json_string())
            .header(CONTENT_TYPE, "application/json")
            .send()
            .await?;
        assert_eq!(response.status(), reqwest::StatusCode::TOO_MANY_REQUESTS);
        let aptos_error = AptosTapError::parse_from_json_string(&response.text().await?)
            .expect("Failed to read response as AptosError");
        let rejection_reason_codes: HashSet<RejectionReasonCode> = aptos_error
            .rejection_reasons
            .into_iter()
            .map(|r| r.get_code())
            .collect();
        assert!(rejection_reason_codes.contains(&RejectionReasonCode::UsageLimitExhausted));

        Ok(())
    }

    // We skip this for now since we have no current need to use the TransferFunder.
    #[ignore]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_transfer_health() -> Result<()> {
        // Create a local account and store its private key at the path expected by
        // the config for this test.
        let private_key = Ed25519PrivateKey::generate(&mut StdRng::from_seed(OsRng.gen()));
        let serialized_keys = aptos_sdk::bcs::to_bytes(&private_key)?;
        let mut key_file = std::fs::File::create("/tmp/transfer_funder_devnet.key")?;
        key_file.write_all(&serialized_keys)?;

        // Create it on chain using the prod devnet faucet. We fund it with
        // exactly the minimum funds set in the config.
        let account_address =
            AuthenticationKey::ed25519(&private_key.public_key()).account_address();
        unwrap_reqwest_result(
            reqwest::Client::new()
                .post("https://faucet.devnet.aptoslabs.com/fund")
                .body(
                    FundRequest {
                        amount: Some(10_000_000),
                        address: Some(account_address.to_string()),
                        ..Default::default()
                    }
                    .to_json_string(),
                )
                .header(CONTENT_TYPE, "application/json")
                .send()
                .await,
        )
        .await?;

        // Wait a few seconds for all the fullnodes to catch up.
        tokio::time::sleep(Duration::from_secs(5)).await;

        // Start the server, using the account we just created.
        init();
        let config_content = include_str!("../../../configs/testing_transfer_funder.yaml");
        let (port, _handle) = start_server(config_content).await?;

        // Assert that `/` returns healthy.
        unwrap_reqwest_result(
            reqwest::Client::new()
                .get(get_root_endpoint(port))
                .send()
                .await,
        )
        .await?;

        // Make a request to the tap in this test (not the prod one) and assert that it works.
        unwrap_reqwest_result(
            reqwest::Client::new()
                .post(get_fund_endpoint(port))
                .body(get_fund_request(None).to_json_string())
                .header(CONTENT_TYPE, "application/json")
                .send()
                .await,
        )
        .await?;

        // Wait a few seconds for all the fullnodes to catch up.
        tokio::time::sleep(Duration::from_secs(5)).await;

        // Now check the health endpoint. It should now be unhealthy because the
        // account balance has dropped below the minimum.
        let response = reqwest::Client::new()
            .get(get_root_endpoint(port))
            .send()
            .await;
        assert_eq!(
            response.unwrap().status(),
            reqwest::StatusCode::SERVICE_UNAVAILABLE
        );

        // An additional fund request should fail.
        let response = reqwest::Client::new()
            .post(get_fund_endpoint(port))
            .body(get_fund_request(Some(10)).to_json_string())
            .header(CONTENT_TYPE, "application/json")
            .send()
            .await?;
        assert_eq!(response.status(), reqwest::StatusCode::SERVICE_UNAVAILABLE);
        let aptos_error = AptosTapError::parse_from_json_string(&response.text().await?)
            .expect("Failed to read response as AptosError");
        assert_eq!(
            aptos_error.error_code,
            AptosTapErrorCode::FunderAccountProblem
        );

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_mint_funder() -> Result<()> {
        // Assert that a localnet is alive.
        let aptos_node_api_client = aptos_sdk::rest_client::Client::new(
            reqwest::Url::from_str("http://127.0.0.1:8080").unwrap(),
        );
        aptos_node_api_client
            .get_index_bcs()
            .await
            .context("Localnet API couldn't be reached at port 8080, have you started one?")?;

        init();
        let (port, _handle) = {
            // Ensure this server and that for test_mint_funder_wait_for_txns
            // don't start up simultaneously, since they're using the same mint key.
            let _guard = MUTEX.get().unwrap().lock().await;
            let config_content = include_str!("../../../configs/testing_mint_funder_local.yaml");
            start_server(config_content).await?
        };

        // Make a request to fund a new account.
        let fund_request = get_fund_request(Some(10));
        let response = unwrap_reqwest_result(
            reqwest::Client::new()
                .post(get_fund_endpoint(port))
                .body(fund_request.to_json_string())
                .header(CONTENT_TYPE, "application/json")
                .send()
                .await,
        )
        .await?;
        let fund_response = FundResponse::parse_from_json_string(&response.text().await?)
            .expect("Failed to read response as FundResponse");

        // Wait for the transaction.
        let response = aptos_node_api_client
            .wait_for_transaction_by_hash(
                HashValue::from_str(&fund_response.txn_hashes[0])?,
                get_current_time_secs() + 30,
                None,
                None,
            )
            .await
            .context("Failed to wait for transaction")?;

        // Ensure it succeeded.
        assert!(
            response.inner().success(),
            "Transaction failed: {:#?}",
            response
        );

        // Assert that the account exists now with the expected balance.
        let response = aptos_node_api_client
            .view_apt_account_balance(
                AccountAddress::from_str(&fund_request.address.unwrap()).unwrap(),
            )
            .await?;

        assert_eq!(response.into_inner(), 10);

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_mint_funder_wait_for_txns() -> Result<()> {
        // Assert that a localnet is alive.
        let aptos_node_api_client = aptos_sdk::rest_client::Client::new(
            reqwest::Url::from_str("http://127.0.0.1:8080").unwrap(),
        );
        aptos_node_api_client
            .get_index_bcs()
            .await
            .context("Localnet API couldn't be reached at port 8080, have you started one?")?;

        init();
        let (port, _handle) = {
            // Ensure this server and that for test_mint_funder
            // don't start up simultaneously, since they're using the same mint key.
            let _guard = MUTEX.get().unwrap().lock().await;
            let config_content =
                include_str!("../../../configs/testing_mint_funder_local_wait_for_txns.yaml");
            start_server(config_content).await?
        };

        // Make a request to fund a new account.
        let fund_request = get_fund_request(Some(10));
        let response = unwrap_reqwest_result(
            reqwest::Client::new()
                .post(get_fund_endpoint(port))
                .body(fund_request.to_json_string())
                .header(CONTENT_TYPE, "application/json")
                .send()
                .await,
        )
        .await?;
        let fund_response = FundResponse::parse_from_json_string(&response.text().await?)
            .expect("Failed to read response as FundResponse");

        // Ensure the transaction was executed now that the tap request has finished.
        let response = aptos_node_api_client
            .get_transaction_by_hash(HashValue::from_str(&fund_response.txn_hashes[0])?)
            .await
            .context("Failed to get transaction, it should be on-chain now")?;

        // Ensure it succeeded.
        assert!(
            response.inner().success(),
            "Transaction failed: {:#?}",
            response
        );

        // Assert that the account exists now with the expected balance.
        let response = aptos_node_api_client
            .view_apt_account_balance(
                AccountAddress::from_str(&fund_request.address.unwrap()).unwrap(),
            )
            .await?;

        assert_eq!(response.into_inner(), 10);

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_maximum_amount_with_bypass() -> Result<()> {
        make_auth_tokens_file(&["test_token"])?;

        // Assert that a localnet is alive.
        let aptos_node_api_client = aptos_sdk::rest_client::Client::new(
            reqwest::Url::from_str("http://127.0.0.1:8080").unwrap(),
        );
        aptos_node_api_client
            .get_index_bcs()
            .await
            .context("Localnet API couldn't be reached at port 8080, have you started one?")?;

        init();
        let (port, _handle) = {
            // Ensure this server and that for test_mint_funder_*
            // don't start up simultaneously, since they're using the same mint key.
            let _guard = MUTEX.get().unwrap().lock().await;
            let config_content =
                include_str!("../../../configs/testing_mint_funder_local_wait_for_txns.yaml");
            start_server(config_content).await?
        };

        // Make a request for more than maximum_amount. This should be accepted as is
        // because we're including an auth token that lets us bypass the checkers,
        // meaning we're instead bound by maximum_amount_with_bypass.
        let fund_request = get_fund_request(Some(1000));
        unwrap_reqwest_result(
            reqwest::Client::new()
                .post(get_fund_endpoint(port))
                .body(fund_request.to_json_string())
                .header(CONTENT_TYPE, "application/json")
                .header(AUTHORIZATION, "Bearer test_token")
                .send()
                .await,
        )
        .await?;

        // Confirm that the account was given the full 1000 OCTA as requested.
        let response = aptos_node_api_client
            .view_apt_account_balance(
                AccountAddress::from_str(&fund_request.address.unwrap()).unwrap(),
            )
            .await?;

        assert_eq!(response.into_inner(), 1000);

        // This time, don't include the auth token. We request more than maximum_amount,
        // but later we'll see that the faucet will only give us maximum_amount, not
        // the amount we requested.
        let fund_request = get_fund_request(Some(1000));
        reqwest::Client::new()
            .post(get_fund_endpoint(port))
            .body(fund_request.to_json_string())
            .header(CONTENT_TYPE, "application/json")
            .send()
            .await?;

        // Confirm that the account was only given 100 OCTA (maximum_amount), not 1000.
        let response = aptos_node_api_client
            .view_apt_account_balance(
                AccountAddress::from_str(&fund_request.address.unwrap()).unwrap(),
            )
            .await?;

        assert_eq!(response.into_inner(), 100);

        Ok(())
    }
}
