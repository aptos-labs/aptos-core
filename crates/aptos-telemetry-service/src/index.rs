use std::convert::Infallible;

use crate::{auth, context::Context};
use warp::{
    http::{HeaderValue, StatusCode},
    reply, Filter, Rejection, Reply,
};

pub fn routes(context: Context) -> impl Filter<Extract = impl Reply, Error = Infallible> + Clone {
    auth::auth(context).recover(handle_rejection)
}

async fn handle_rejection(err: Rejection) -> Result<impl Reply, Infallible> {
    println!("Rejected request {:?}", err);
    let code = StatusCode::INTERNAL_SERVER_ERROR;
    let body = reply::json(&"Not Fine".to_owned());
    let mut rep = reply::with_status(body, code).into_response();
    rep.headers_mut()
        .insert("access-control-allow-origin", HeaderValue::from_static("*"));
    Ok(rep)
}
