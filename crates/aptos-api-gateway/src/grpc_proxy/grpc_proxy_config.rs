use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GrpcProxyConfig {
    #[serde()]
    pub upstream_host: String,
}
