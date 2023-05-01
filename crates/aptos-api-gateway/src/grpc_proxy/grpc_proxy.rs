use anyhow::Ok;
use hyper::{self, Request as HyperRequest, Version};
use poem::{
    Endpoint, error::IntoResult, Request as PoemRequest, Response, Result as PoemResult,
};
use url::Url;

pub struct GrpcProxy {
    upstream_host: String,
}

impl GrpcProxy {
    pub fn new(upstream_host: String) -> Self {
        Url::parse(upstream_host.as_str())
            .expect(format!("invalid upstream host: {upstream_host}").as_str());
        Self { upstream_host }
    }
}

#[poem::async_trait]
impl Endpoint for GrpcProxy {
    type Output = Response;

    async fn call(&self, req: PoemRequest) -> PoemResult<Self::Output> {
        async {
            let client = hyper::Client::builder()
                .http2_only(true)
                .build_http::<hyper::Body>();
            let uri = format!(
                "{}{}",
                self.upstream_host,
                req.uri().path_and_query().unwrap()
            );
            println!("proxying call to uri: {}", uri);

            let mut client_req = HyperRequest::builder()
                .method(req.method())
                .version(Version::HTTP_2)
                .uri(uri);

            for (key, value) in req.headers() {
                client_req = client_req.header(key, value);
            }

            let client_req = client_req.body(req.into_body().into())?;
            let client_res = client.request(client_req).await?;

            let (parts, body) = client_res.into_parts();
            let mut poem_response = Response::builder().body(body);

            poem_response.set_status(parts.status);
            *poem_response.headers_mut() = parts.headers;

            Ok(poem_response)
        }
        .await
        .into_result()
    }
}

// #[handler]
// pub async fn proxy(req: &PoemRequest, body: Body) -> AnyResult<impl IntoResponse> {
//     let mut client = hyper::Client::builder()
//         .http2_only(true)
//         .build_http::<hyper::Body>();
//     let uri = format!(
//         "http://localhost:50902{}",
//         req.uri().path_and_query().unwrap()
//     );
//     println!("proxying call to uri: {}", uri);

//     let mut client_req = HyperRequest::builder()
//         .method(req.method())
//         .version(Version::HTTP_2)
//         .uri(uri)
//         .version(req.version());

//     for (key, value) in req.headers() {
//         client_req = client_req.header(key, value);
//     }

//     let client_req = client_req.body(body.into())?;
//     let client_res = client.request(client_req).await?;

//     let (parts, body) = client_res.into_parts();
//     let mut poem_response = Response::builder().body(body);

//     poem_response.set_status(parts.status);
//     *poem_response.headers_mut() = parts.headers;

//     Ok(poem_response)
// }
