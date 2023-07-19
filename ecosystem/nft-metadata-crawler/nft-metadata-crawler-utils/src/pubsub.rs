// Copyright © Aptos Foundation

use google_cloud_googleapis::pubsub::v1::{
    publisher_client::PublisherClient, subscriber_client::SubscriberClient, AcknowledgeRequest,
    PublishRequest, PublishResponse, PubsubMessage, PullRequest, PullResponse,
};
use tonic::{metadata::MetadataValue, transport::Channel, Request, Response};

#[allow(deprecated)]
pub async fn publish_uris(
    links: Vec<String>,
    force: bool,
    grpc_client: &mut PublisherClient<Channel>,
    topic_name: String,
    token: String,
) -> anyhow::Result<Response<PublishResponse>> {
    let messages = links.iter().map(|link| PubsubMessage {
        data: format!("{},{}", link, force).as_bytes().to_vec(),
        ..Default::default()
    });

    let mut request = Request::new(PublishRequest {
        topic: topic_name,
        messages: messages.into_iter().collect(),
    });

    request.metadata_mut().insert(
        "authorization",
        MetadataValue::from_str(format!("Bearer {}", token).as_str())
            .expect("Unable to create metadata"),
    );

    grpc_client
        .publish(request)
        .await
        .map_err(|e| anyhow::anyhow!(e))
}

#[allow(deprecated)]
pub async fn consume_uris(
    count: i32,
    grpc_client: &mut SubscriberClient<Channel>,
    subscription_name: String,
    token: String,
) -> anyhow::Result<Response<PullResponse>> {
    let mut request = Request::new(PullRequest {
        subscription: subscription_name,
        max_messages: count,
        return_immediately: true,
    });

    request.metadata_mut().insert(
        "authorization",
        MetadataValue::from_str(format!("Bearer {}", token).as_str())
            .expect("Unable to create metadata"),
    );

    grpc_client
        .pull(request)
        .await
        .map_err(|e| anyhow::anyhow!(e))
}

#[allow(deprecated)]
pub async fn send_acks(
    ack_ids: Vec<String>,
    grpc_client: &mut SubscriberClient<Channel>,
    subscription: String,
    token: String,
) -> anyhow::Result<Response<()>> {
    let mut request = Request::new(AcknowledgeRequest {
        subscription,
        ack_ids,
    });

    request.metadata_mut().insert(
        "authorization",
        MetadataValue::from_str(format!("Bearer {}", token).as_str())
            .expect("Unable to create metadata"),
    );

    grpc_client
        .acknowledge(request)
        .await
        .map_err(|e| anyhow::anyhow!(e))
}
