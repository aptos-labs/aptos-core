// Copyright © Aptos Foundation

use crate::dag::types::DAGNetworkMessage;

#[test]
fn test_dag_network_message() {
    let short_data = vec![10; 10];
    let long_data = vec![20; 30];

    let short_message = DAGNetworkMessage {
        epoch: 1,
        data: short_data,
    };

    assert_eq!(
        format!("{:?}", short_message),
        "DAGNetworkMessage { epoch: 1, data: \"0a0a0a0a0a0a0a0a0a0a\" }"
    );

    let long_message = DAGNetworkMessage {
        epoch: 2,
        data: long_data,
    };

    assert_eq!(
        format!("{:?}", long_message),
        "DAGNetworkMessage { epoch: 2, data: \"1414141414141414141414141414141414141414\" }"
    );
}
