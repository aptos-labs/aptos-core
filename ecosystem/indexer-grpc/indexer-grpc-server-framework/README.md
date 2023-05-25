# Indexer GRPC server framework

A boilerplate server runtime for indexer grpc infra.

## Usage

```rust
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExampleConfig {
    pub name: String,
}

impl RunnableConfig for ExampleConfig {
    fn run(&self) -> Result<()> {
        println!("Hello, {}!", self.name);
        Ok(())
    }
    fn get_server_name(&self) -> String {
        "srv_exp".to_string()
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = ServerArgs::parse();
    args.run::<ExampleConfig>().await
}
```