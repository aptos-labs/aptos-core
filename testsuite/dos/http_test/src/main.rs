use axum::{
    routing::get,
    Router,
};
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() {
    // build our application with a single route
    println!("Hi Sleeping for 2");
    let app = Router::new().route("/", get(|| async { println!("sleepinf for 2 ");sleep(Duration::from_secs(2)).await; "Hello, World!\n" }));

    // run it with hyper on localhost:3000
    axum::Server::bind(&"0.0.0.0:8080".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
