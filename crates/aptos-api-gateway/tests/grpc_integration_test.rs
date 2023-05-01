use anyhow::Result;
use aptos_api_gateway::{grpc_proxy::grpc_proxy_config::GrpcProxyConfig, server::run::RunConfig};
use futures::StreamExt;
use poem::{
    listener::{Acceptor, Listener, TcpListener},
    Server,
};
use poem_grpc::{ClientConfig, Request, Response, RouteGrpc, Status, Streaming};

poem_grpc::include_proto!("testing");

struct TestSvcImpl;

#[poem::async_trait]
impl TestSvc for TestSvcImpl {
    async fn say_hello(
        &self,
        request: Request<HelloRequest>,
    ) -> Result<Response<HelloReply>, Status> {
        let reply = HelloReply {
            message: format!("Hello {}!", request.into_inner().name),
        };
        Ok(Response::new(reply))
    }

    async fn stream_test(
        &self,
        _request: Request<StreamTestRequest>,
    ) -> std::result::Result<Response<Streaming<StreamTestResponse>>, Status> {
        let stream = Streaming::new(async_stream::stream! {
            for i in 0..2 {
                yield Ok(StreamTestResponse {
                    name: "fooo".into(),
                    message_counter: i,
                });
            }
        });
        Ok(Response::new(stream))
    }
}

struct TestBackend {
    host: String,
}

impl TestBackend {
    pub async fn spawn() -> Result<TestBackend> {
        let acceptor = TcpListener::bind("127.0.0.1:0").into_acceptor().await?;

        let address = acceptor
            .local_addr()
            .clone()
            .remove(0)
            .as_socket_addr()
            .unwrap()
            .clone();

        let endpoint = TestSvcServer::new(TestSvcImpl);

        let route = RouteGrpc::new().add_service(endpoint);
        let server = Server::new_with_acceptor(acceptor);

        tokio::spawn(async move {
            println!("starting server on port ");
            server.run(route).await.unwrap();
        });

        Ok(TestBackend {
            host: format!("http://{address}"),
        })
    }
}

// impl GreeterService {
//     pub fn new() -> Self {
//         Self {}
//     }

// #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
// async fn test_grpc_proxy() -> Result<()> {
//     let route = RouteGrpc::new().add_service(GreeterServer::new(GreeterService));
//     let listener = TcpListener::bind("127.0.0.1:3000");
//     let server = Server::new(listener);

//     let server_handle = tokio::spawn(async move {
//         println!("about to start server");
//         server.run(route).await.unwrap();
//     });

//     let client = GreeterClient::new(
//         ClientConfig::builder()
//             .uri("http://localhost:3000")
//             .build()
//             .unwrap(),
//     );
//     let request = Request::new(HelloRequest {
//         name: "Tonic".into(),
//     });
//     let response = client.say_hello(request).await?;
//     println!("RESPONSE={response:?}");
//     Ok(())
// }

#[cfg(test)]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_proxy_grpc_streaming() -> Result<()> {
    // let route = RouteGrpc::new().add_service(TestSvcServer::new(TestServiceImpl));
    // let listener = NativeTcpListener::bind("127.0.0.1:3000");
    // let server = Server::new(listener);

    //
    // tokio::spawn(async move {
    //     println!("about to start server");
    //     server.run(route).await.unwrap();
    // });
    let test_backend = TestBackend::spawn().await?;

    let gateway = RunConfig {
        grpc_proxy_config: GrpcProxyConfig {
            upstream_host: test_backend.host.to_string(),
        },
        handler_config: Default::default(),
        bypasser_configs: Default::default(),
        server_config: Default::default(),
        metrics_server_config: Default::default(),
        checker_configs: Default::default(),
    };

    let port = 3001;

    tokio::spawn(async move {
        println!("about to start grpc_proxy server");
        gateway.run_test(port).await.unwrap();
    });

    let client = TestSvcClient::new(
        ClientConfig::builder()
            .uri(format!("http://localhost:{}", port))
            .build()
            .unwrap(),
    );
    let request = Request::new(StreamTestRequest {
        name: "Tonic".into(),
    });
    let streaming_response = client.stream_test(request).await?.into_inner();

    let messages = streaming_response
        .map(|x| x.unwrap())
        .collect::<Vec<_>>()
        .await;

    assert_eq!(messages.len(), 2);
    assert_eq!(messages, vec![
        StreamTestResponse {
            name: "fooo".into(),
            message_counter: 0,
        },
        StreamTestResponse {
            name: "fooo".into(),
            message_counter: 1,
        }
    ]);
    // response.
    // println!("RESPONSE={response:?}");
    Ok(())
}

// // #[tokio::main]
// async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
//     let client = GreeterClient::new(
//         ClientConfig::builder()
//             .uri("http://localhost:3000")
//             .build()
//             .unwrap(),
//     );
//     let request = Request::new(HelloRequest {
//         name: "Tonic".into(),
//     });
//     let response = client.say_hello(request).await?;
//     println!("RESPONSE={response:?}");
//     Ok(())
// }
//
//
// // #[tokio::main]
// async fn main() -> Result<(), std::io::Error> {}
