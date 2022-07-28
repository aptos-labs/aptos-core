mod graphql_root_queries;

use aptos_indexer::database::new_db_pool;
use async_graphql::{
    http::{playground_source, GraphQLPlaygroundConfig},
    EmptyMutation, EmptySubscription, Schema,
};
use async_graphql_poem::GraphQL;
use clap::Parser;
use graphql_root_queries::{ContextData, QueryRoot};
use poem::{get, handler, listener::TcpListener, web::Html, IntoResponse, Route, Server};

#[handler]
async fn graphql_playground() -> impl IntoResponse {
    Html(playground_source(GraphQLPlaygroundConfig::new("/")))
}

#[derive(Debug, Parser)]
#[clap(author, version, about, long_about = None)]
struct IndexerArgs {
    /// Postgres database uri, ex: "postgresql://user:pass@localhost/postgres"
    #[clap(long)]
    pg_uri: String,
}

#[tokio::main]
async fn main() {
    let args: IndexerArgs = IndexerArgs::parse();

    let conn_pool = new_db_pool(&args.pg_uri).unwrap();

    let schema = Schema::build(QueryRoot, EmptyMutation, EmptySubscription)
        .data(ContextData { pool: conn_pool })
        .finish();
    let app = Route::new().at("/", get(graphql_playground).post(GraphQL::new(schema)));
    println!("Playground: http://localhost:4005");
    Server::new(TcpListener::bind("0.0.0.0:4005"))
        .run(app)
        .await
        .unwrap();
}
