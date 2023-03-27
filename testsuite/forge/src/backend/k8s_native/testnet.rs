// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use kube::api::{Api, ListParams};

enum NodeType {
    Validator,
    ValidatorFullnode,
    PublicFullnode,
}

struct TestnetLayout {
    nodes: Vec<Vec<NodeType>>,
}

fn create_validator(
    node_name: usize,
    node_image_tag: String,
) -> Result<StatefulSet> {

}

async fn install_testnet_resources(
    kube_namespace: String,
    num_validators: usize,
    num_fullnodes: usize,
    node_image_tag: String,
    genesis_image_tag: String,
    genesis_modules_path: Option<String>,
) -> Result<(HashMap<PeerId, K8sNativeNode>, HashMap<PeerId, K8sNativeNode>)> {
    let kube_client = create_k8s_client().await;

    let mut testnet_layout = TestnetLayout {
        nodes: vec![],
    };
    for v in 0..num_validators {
        let mut node_types = vec![NodeType::Validator];
        if v < num_fullnodes {
            node_types.push(NodeType::ValidatorFullnode);
        }
        testnet_layout.nodes.push(node_types);
    }

    // create the testnet layout

}
