// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    get_stateful_set_image, make_k8s_label, K8sNode, ReadWrite, Result, Version,
    DEFAULT_TEST_SUITE_NAME, DEFAULT_USERNAME, REST_API_SERVICE_PORT,
    VALIDATOR_0_DATA_PERSISTENT_VOLUME_CLAIM_PREFIX, VALIDATOR_0_GENESIS_SECRET_PREFIX,
    VALIDATOR_0_STATEFUL_SET_NAME,
};
use anyhow::Context;
use aptos_config::{
    config::{
        ApiConfig, BaseConfig, DiscoveryMethod, ExecutionConfig, NetworkConfig, NodeConfig,
        OverrideNodeConfig, RoleType, WaypointConfig,
    },
    network_id::NetworkId,
};
use aptos_sdk::types::PeerId;
use aptos_short_hex_str::AsShortHexStr;
use k8s_openapi::{
    api::{
        apps::v1::{StatefulSet, StatefulSetSpec},
        core::v1::{
            ConfigMap, ConfigMapVolumeSource, Container, PersistentVolumeClaim,
            PersistentVolumeClaimSpec, PodSpec, PodTemplateSpec, ResourceRequirements,
            SecretVolumeSource, Service, ServicePort, ServiceSpec, Volume, VolumeMount,
        },
    },
    apimachinery::pkg::apis::meta::v1::LabelSelector,
};
use kube::api::{ObjectMeta, PostParams};
use log::info;
use std::{
    collections::BTreeMap,
    env,
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    path::PathBuf,
    sync::{atomic::AtomicU32, Arc},
};
use tempfile::TempDir;

// these are constants given by the aptos-node helm chart
// see terraform/helm/aptos-node/templates/validator.yaml

// the name of the NodeConfig for the PFN, as well as the key in the k8s ConfigMap
// where the NodeConfig is stored
const FULLNODE_CONFIG_MAP_KEY: &str = "fullnode.yaml";

// the path where the genesis is mounted in the validator
const GENESIS_CONFIG_VOLUME_NAME: &str = "genesis-config";
const GENESIS_CONFIG_VOLUME_PATH: &str = "/opt/aptos/genesis";
const GENESIS_CONFIG_WRITABLE_VOLUME_NAME: &str = "writable-genesis";

// the path where the config file is mounted in the fullnode
const APTOS_CONFIG_VOLUME_NAME: &str = "aptos-config";
const APTOS_CONFIG_VOLUME_PATH: &str = "/opt/aptos/etc";

// the path where the data volume is mounted in the fullnode
const APTOS_DATA_VOLUME_NAME: &str = "aptos-data";
const APTOS_DATA_VOLUME_PATH: &str = "/opt/aptos/data";

/// Derive the fullnode image from the validator image. They will share the same image repo (validator), but not necessarily the version (image tag)
fn get_fullnode_image_from_validator_image(
    validator_stateful_set: &StatefulSet,
    version: &Version,
) -> Result<String> {
    let fullnode_kube_image = get_stateful_set_image(validator_stateful_set)?;
    let fullnode_image_repo = fullnode_kube_image.name;

    // fullnode uses the validator image, with a different image tag
    Ok(format!("{}:{}", fullnode_image_repo, version))
}

/// Create a ConfigMap with the given NodeConfig, with a constant key
async fn create_node_config_configmap(
    node_config_config_map_name: String,
    node_config: &OverrideNodeConfig,
) -> Result<ConfigMap> {
    let mut data: BTreeMap<String, String> = BTreeMap::new();
    data.insert(
        FULLNODE_CONFIG_MAP_KEY.to_string(),
        serde_yaml::to_string(&node_config.get_yaml()?)?,
    );
    let node_config_config_map = ConfigMap {
        binary_data: None,
        data: Some(data.clone()),
        metadata: ObjectMeta {
            name: Some(node_config_config_map_name),
            ..ObjectMeta::default()
        },
        immutable: None,
    };
    Ok(node_config_config_map)
}

/// Create a PFN data volume by using the validator data volume as a template
fn create_fullnode_persistent_volume_claim(
    validator_data_volume: PersistentVolumeClaim,
) -> Result<PersistentVolumeClaim> {
    let volume_requests = validator_data_volume
        .spec
        .as_ref()
        .expect("Could not get volume spec from validator data volume")
        .resources
        .as_ref()
        .expect("Could not get volume resources from validator data volume")
        .requests
        .clone();

    Ok(PersistentVolumeClaim {
        metadata: ObjectMeta {
            name: Some(APTOS_DATA_VOLUME_NAME.to_string()),
            ..ObjectMeta::default()
        },
        spec: Some(PersistentVolumeClaimSpec {
            access_modes: Some(vec!["ReadWriteOnce".to_string()]),
            resources: Some(ResourceRequirements {
                requests: volume_requests,
                ..ResourceRequirements::default()
            }),
            ..PersistentVolumeClaimSpec::default()
        }),
        ..PersistentVolumeClaim::default()
    })
}

fn create_fullnode_labels(fullnode_name: String) -> BTreeMap<String, String> {
    // if present, tag the node with the test suite name and username
    let suite_name = env::var("FORGE_TEST_SUITE").unwrap_or(DEFAULT_TEST_SUITE_NAME.to_string());
    let username = env::var("FORGE_USERNAME").unwrap_or(DEFAULT_USERNAME.to_string());

    [
        ("app.kubernetes.io/name".to_string(), "fullnode".to_string()),
        ("app.kubernetes.io/instance".to_string(), fullnode_name),
        ("forge-test-suite".to_string(), make_k8s_label(suite_name)),
        ("forge-username".to_string(), make_k8s_label(username)),
        (
            "app.kubernetes.io/part-of".to_string(),
            "forge-pfn".to_string(),
        ),
    ]
    .iter()
    .cloned()
    .collect()
}

fn create_fullnode_service(fullnode_name: String) -> Result<Service> {
    Ok(Service {
        metadata: ObjectMeta {
            name: Some(fullnode_name.clone()),
            ..ObjectMeta::default()
        },
        spec: Some(ServiceSpec {
            selector: Some(create_fullnode_labels(fullnode_name)),
            // for now, only expose the REST API
            ports: Some(vec![ServicePort {
                port: REST_API_SERVICE_PORT as i32,
                ..ServicePort::default()
            }]),
            ..ServiceSpec::default()
        }),
        ..Service::default()
    })
}

fn create_fullnode_container(
    fullnode_image: String,
    validator_container: &Container,
) -> Result<Container> {
    Ok(Container {
        image: Some(fullnode_image),
        args: Some(vec![
            "/usr/local/bin/aptos-node".to_string(),
            "-f".to_string(),
            format!("/opt/aptos/etc/{}", FULLNODE_CONFIG_MAP_KEY),
        ]),
        volume_mounts: Some(vec![
            VolumeMount {
                mount_path: APTOS_CONFIG_VOLUME_PATH.to_string(),
                name: APTOS_CONFIG_VOLUME_NAME.to_string(),
                ..VolumeMount::default()
            },
            VolumeMount {
                mount_path: APTOS_DATA_VOLUME_PATH.to_string(),
                name: APTOS_DATA_VOLUME_NAME.to_string(),
                ..VolumeMount::default()
            },
            VolumeMount {
                mount_path: GENESIS_CONFIG_VOLUME_PATH.to_string(),
                name: GENESIS_CONFIG_WRITABLE_VOLUME_NAME.to_string(),
                ..VolumeMount::default()
            },
        ]),
        name: "fullnode".to_string(),
        // specifically, inherit resources, env,ports, securityContext from the validator's container
        ..validator_container.clone()
    })
}

fn create_fullnode_volumes(
    fullnode_genesis_secret_name: String,
    fullnode_node_config_config_map_name: String,
) -> Vec<Volume> {
    vec![
        Volume {
            name: GENESIS_CONFIG_VOLUME_NAME.to_string(),
            secret: Some(SecretVolumeSource {
                secret_name: Some(fullnode_genesis_secret_name),
                ..SecretVolumeSource::default()
            }),
            ..Volume::default()
        },
        Volume {
            name: APTOS_CONFIG_VOLUME_NAME.to_string(),
            config_map: Some(ConfigMapVolumeSource {
                name: Some(fullnode_node_config_config_map_name),
                ..ConfigMapVolumeSource::default()
            }),
            ..Volume::default()
        },
        Volume {
            name: GENESIS_CONFIG_WRITABLE_VOLUME_NAME.to_string(),
            empty_dir: Some(Default::default()),
            ..Volume::default()
        },
    ]
}

/// Create a fullnode StatefulSet given some templates from the validator
fn create_fullnode_stateful_set(
    fullnode_name: String,
    fullnode_image: String,
    fullnode_genesis_secret_name: String,
    fullnode_node_config_config_map_name: String,
    validator_stateful_set: StatefulSet,
    validator_data_volume: PersistentVolumeClaim,
) -> Result<StatefulSet> {
    // extract some useful structs from the validator
    let validator_stateful_set_spec = validator_stateful_set
        .spec
        .as_ref()
        .context("Validator StatefulSet does not have spec")?
        .clone();
    let validator_stateful_set_pod_spec = validator_stateful_set_spec
        .template
        .spec
        .as_ref()
        .context("Validator StatefulSet does not have spec.template.spec")?
        .clone();

    let validator_container = validator_stateful_set_pod_spec
        .containers
        .first()
        .context("Validator StatefulSet does not have any containers")?;

    // common labels
    let labels_map: BTreeMap<String, String> = create_fullnode_labels(fullnode_name.clone());

    // create the fullnode data volume
    let data_volume = create_fullnode_persistent_volume_claim(validator_data_volume)?;

    // create the fullnode container
    let fullnode_container = create_fullnode_container(fullnode_image, validator_container)?;

    // create the fullnode volumes
    let fullnode_volumes = create_fullnode_volumes(
        fullnode_genesis_secret_name,
        fullnode_node_config_config_map_name,
    );

    // build the fullnode stateful set
    let mut fullnode_stateful_set = StatefulSet::default();
    fullnode_stateful_set.metadata.name = Some(fullnode_name.clone());
    fullnode_stateful_set.metadata.labels = Some(labels_map.clone());
    fullnode_stateful_set.spec = Some(StatefulSetSpec {
        service_name: fullnode_name, // the name of the service is the same as that of the fullnode
        selector: LabelSelector {
            match_labels: Some(labels_map.clone()),
            ..LabelSelector::default()
        },
        volume_claim_templates: Some(vec![data_volume]), // a PVC that is created directly by the StatefulSet, and owned by it
        template: PodTemplateSpec {
            metadata: Some(ObjectMeta {
                labels: Some(labels_map),
                ..ObjectMeta::default()
            }),
            spec: Some(PodSpec {
                containers: vec![fullnode_container],
                volumes: Some(fullnode_volumes),
                // specifically, inherit nodeSelector, affinity, tolerations, securityContext, serviceAccountName from the validator's PodSpec
                ..validator_stateful_set_pod_spec.clone()
            }),
        },
        ..validator_stateful_set_spec
    });
    Ok(fullnode_stateful_set)
}

/// Create a default PFN NodeConfig that uses the genesis, waypoint, and data paths expected in k8s
pub fn get_default_pfn_node_config() -> NodeConfig {
    let mut waypoint_path = PathBuf::from(GENESIS_CONFIG_VOLUME_PATH);
    waypoint_path.push("waypoint.txt");

    let mut genesis_path = PathBuf::from(GENESIS_CONFIG_VOLUME_PATH);
    genesis_path.push("genesis.blob");

    NodeConfig {
        base: BaseConfig {
            role: RoleType::FullNode,
            data_dir: PathBuf::from(APTOS_DATA_VOLUME_PATH),
            waypoint: WaypointConfig::FromFile(waypoint_path),
            ..BaseConfig::default()
        },
        execution: ExecutionConfig {
            genesis_file_location: genesis_path,
            ..ExecutionConfig::default()
        },
        full_node_networks: vec![NetworkConfig {
            network_id: NetworkId::Public,
            discovery_method: DiscoveryMethod::Onchain,
            // defaults to listening on "/ip4/0.0.0.0/tcp/6180"
            ..NetworkConfig::default()
        }],
        api: ApiConfig {
            // API defaults to listening on "127.0.0.1:8080". Override with 0.0.0.0:8080
            address: SocketAddr::V4(SocketAddrV4::new(
                Ipv4Addr::new(0, 0, 0, 0),
                REST_API_SERVICE_PORT as u16,
            )),
            ..ApiConfig::default()
        },
        ..NodeConfig::default()
    }
}

/// Create a PFN stateful set workload
/// This function assumes that the swarm has already been set up (e.g. there are already validators running) as it borrows
/// some artifacts such as genesis from the 0th validator
/// The given NodeConfig will be merged with the default PFN NodeConfig for Forge
pub async fn install_public_fullnode<'a>(
    stateful_set_api: Arc<dyn ReadWrite<StatefulSet>>,
    configmap_api: Arc<dyn ReadWrite<ConfigMap>>,
    persistent_volume_claim_api: Arc<dyn ReadWrite<PersistentVolumeClaim>>,
    service_api: Arc<dyn ReadWrite<Service>>,
    version: &'a Version,
    node_config: &'a OverrideNodeConfig,
    era: String,
    namespace: String,
    use_port_forward: bool,
    index: usize,
) -> Result<(PeerId, K8sNode)> {
    let node_peer_id = node_config
        .override_config()
        .get_peer_id()
        .unwrap_or_else(PeerId::random);
    let fullnode_name = format!("public-fullnode-{}-{}", index, node_peer_id.short_str());

    // create the NodeConfig configmap
    let fullnode_node_config_config_map_name = format!("{}-config", fullnode_name.clone());
    let fullnode_node_config_config_map =
        create_node_config_configmap(fullnode_node_config_config_map_name.clone(), node_config)
            .await?;
    configmap_api
        .create(&PostParams::default(), &fullnode_node_config_config_map)
        .await?;

    // assume that the validator workload (val0) has already been created (not necessarily running yet)
    // get its spec so we can inherit some of its properties
    let validator_stateful_set = stateful_set_api.get(VALIDATOR_0_STATEFUL_SET_NAME).await?;

    // get the fullnode image
    let fullnode_image_full =
        get_fullnode_image_from_validator_image(&validator_stateful_set, version)?;

    // borrow genesis secret from the first validator. it follows this naming convention
    let fullnode_genesis_secret_name = format!("{}-e{}", VALIDATOR_0_GENESIS_SECRET_PREFIX, era);
    let validator_data_persistent_volume_claim_name = format!(
        "{}-e{}",
        VALIDATOR_0_DATA_PERSISTENT_VOLUME_CLAIM_PREFIX, era
    );

    // create the data volume
    let validator_data_volume = persistent_volume_claim_api
        .get(validator_data_persistent_volume_claim_name.as_str())
        .await
        .map_err(|e| {
            anyhow::anyhow!(
                "Could not get validator data volume to inherit from {:?}: {:?}",
                validator_data_persistent_volume_claim_name,
                e
            )
        })?;

    let fullnode_stateful_set = create_fullnode_stateful_set(
        fullnode_name.clone(),
        fullnode_image_full,
        fullnode_genesis_secret_name,
        fullnode_node_config_config_map_name,
        validator_stateful_set,
        validator_data_volume,
    )?;

    // check that all the labels are the same
    let fullnode_metadata_labels = fullnode_stateful_set
        .metadata
        .labels
        .as_ref()
        .context("Validator StatefulSet does not have metadata.labels")?;
    let fullnode_spec_selector_match_labels = fullnode_stateful_set
        .spec
        .as_ref()
        .context("Validator StatefulSet does not have spec")?
        .selector
        .match_labels
        .as_ref()
        .context("Validator StatefulSet does not have spec.selector.match_labels")?;
    let fullnode_spec_template_metadata_labels = fullnode_stateful_set
        .spec
        .as_ref()
        .context("Validator StatefulSet does not have spec")?
        .template
        .metadata
        .as_ref()
        .context("Validator StatefulSet does not have spec.template.metadata")?
        .labels
        .as_ref()
        .context("Validator StatefulSet does not have spec.template.metadata.labels")?;

    let labels = [
        fullnode_metadata_labels,
        fullnode_spec_selector_match_labels,
        fullnode_spec_template_metadata_labels,
    ];
    for label1 in labels.into_iter() {
        for label2 in labels.into_iter() {
            assert_eq!(label1, label2);
        }
    }

    let fullnode_service = create_fullnode_service(fullnode_name.clone())?;

    // write the spec to file
    let tmp_dir = TempDir::new().expect("Could not create temp dir");
    let fullnode_config_path = tmp_dir.path().join("fullnode.yaml");
    let fullnode_config_file = std::fs::File::create(&fullnode_config_path)
        .with_context(|| format!("Could not create file {:?}", fullnode_config_path))?;
    serde_yaml::to_writer(fullnode_config_file, &fullnode_stateful_set)?;

    let fullnode_service_path = tmp_dir.path().join("service.yaml");
    let fullnode_service_file = std::fs::File::create(&fullnode_service_path)
        .with_context(|| format!("Could not create file {:?}", fullnode_service_path))?;
    serde_yaml::to_writer(fullnode_service_file, &fullnode_service)?;
    info!("Wrote fullnode k8s specs to path: {:?}", &tmp_dir);

    // create the StatefulSet
    let sts = stateful_set_api
        .create(&PostParams::default(), &fullnode_stateful_set)
        .await?;
    let fullnode_stateful_set_str = serde_yaml::to_string(&fullnode_stateful_set)?;
    info!(
        "Created fullnode StatefulSet:\n---{}\n---",
        &fullnode_stateful_set_str
    );
    // and its service
    service_api
        .create(&PostParams::default(), &fullnode_service)
        .await?;
    let fullnode_service_str = serde_yaml::to_string(&fullnode_service)?;
    info!(
        "Created fullnode Service:\n---{}\n---",
        fullnode_service_str
    );

    let service_name = &fullnode_service
        .metadata
        .name
        .context("Fullnode Service does not have metadata.name")?;

    let full_service_name = format!("{}.{}.svc", service_name, &namespace); // this is the full name that includes the namespace

    // Append the cluster name if its a multi-cluster deployment
    let full_service_name = if let Some(target_cluster_name) = sts
        .metadata
        .labels
        .as_ref()
        .and_then(|labels| labels.get("multicluster/targetcluster"))
    {
        format!("{}.{}", &full_service_name, &target_cluster_name)
    } else {
        full_service_name
    };

    let ret_node = K8sNode {
        name: fullnode_name.clone(),
        stateful_set_name: fullnode_stateful_set
            .metadata
            .name
            .context("Fullnode StatefulSet does not have metadata.name")?,
        peer_id: node_peer_id,
        index,
        service_name: full_service_name,
        version: version.clone(),
        namespace,
        haproxy_enabled: false,

        port_forward_enabled: use_port_forward,
        rest_api_port: AtomicU32::new(REST_API_SERVICE_PORT), // in the case of port-forward, this port will be changed at runtime
    };

    Ok((node_peer_id, ret_node))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MockK8sResourceApi;
    use aptos_config::config::Identity;
    use aptos_sdk::crypto::{x25519::PrivateKey, Uniform};
    use k8s_openapi::apimachinery::pkg::api::resource::Quantity;

    /// Get a dummy validator persistent volume claim that looks like one created by terraform/helm/aptos-node/templates/validator.yaml
    fn get_dummy_validator_persistent_volume_claim() -> PersistentVolumeClaim {
        PersistentVolumeClaim {
            metadata: ObjectMeta {
                name: Some("aptos-node-0-validator-e42069".to_string()),
                ..ObjectMeta::default()
            },
            spec: Some(PersistentVolumeClaimSpec {
                access_modes: Some(vec!["ReadWriteOnce".to_string()]),
                resources: Some(ResourceRequirements {
                    requests: Some(
                        [
                            ("storage".to_string(), Quantity("1Gi".to_string())),
                            ("storage2".to_string(), Quantity("2Gi".to_string())),
                        ]
                        .iter()
                        .cloned()
                        .collect(),
                    ),
                    ..ResourceRequirements::default()
                }),
                ..PersistentVolumeClaimSpec::default()
            }),
            ..PersistentVolumeClaim::default()
        }
    }

    /// Get a dummy validator stateful set that looks like one created by terraform/helm/aptos-node/templates/validator.yaml
    fn get_dummy_validator_stateful_set() -> StatefulSet {
        let labels: BTreeMap<String, String> = [
            (
                "app.kubernetes.io/name".to_string(),
                "validator".to_string(),
            ),
            (
                "app.kubernetes.io/instance".to_string(),
                "aptos-node-0-validator-0".to_string(),
            ),
            (
                "app.kubernetes.io/part-of".to_string(),
                "forge-pfn".to_string(),
            ),
        ]
        .iter()
        .cloned()
        .collect();
        StatefulSet {
            metadata: ObjectMeta {
                name: Some("aptos-node-0-validator".to_string()),
                labels: Some(labels.clone()),
                ..ObjectMeta::default()
            },
            spec: Some(StatefulSetSpec {
                replicas: Some(1),
                template: PodTemplateSpec {
                    metadata: Some(ObjectMeta {
                        labels: Some(labels),
                        ..ObjectMeta::default()
                    }),
                    spec: Some(PodSpec {
                        containers: vec![Container {
                            name: "validator".to_string(),
                            image: Some(
                                "banana.fruit.aptos/potato/validator:banana_image_tag".to_string(),
                            ),
                            command: Some(vec![
                                "/usr/local/bin/aptos-node".to_string(),
                                "-f".to_string(),
                                "/opt/aptos/etc/validator.yaml".to_string(),
                            ]),
                            volume_mounts: Some(vec![
                                VolumeMount {
                                    mount_path: APTOS_CONFIG_VOLUME_PATH.to_string(),
                                    name: APTOS_CONFIG_VOLUME_NAME.to_string(),
                                    ..VolumeMount::default()
                                },
                                VolumeMount {
                                    mount_path: APTOS_DATA_VOLUME_PATH.to_string(),
                                    name: APTOS_DATA_VOLUME_NAME.to_string(),
                                    ..VolumeMount::default()
                                },
                                VolumeMount {
                                    mount_path: GENESIS_CONFIG_VOLUME_PATH.to_string(),
                                    name: GENESIS_CONFIG_WRITABLE_VOLUME_NAME.to_string(),
                                    ..VolumeMount::default()
                                },
                            ]),
                            ..Container::default()
                        }],
                        ..PodSpec::default()
                    }),
                },
                ..StatefulSetSpec::default()
            }),
            ..StatefulSet::default()
        }
    }

    #[tokio::test]
    /// Test that we can create a node config configmap and that it contains the node config at a known data key
    async fn test_create_node_config_map() {
        let config_map_name = "aptos-node-0-validator-0-config".to_string();
        let node_config = NodeConfig::default();
        let override_config = OverrideNodeConfig::new_with_default_base(node_config.clone());

        // expect that the one we get is the same as the one we created
        let created_config_map =
            create_node_config_configmap(config_map_name.clone(), &override_config)
                .await
                .unwrap();

        let regenerated_node_config = serde_yaml::from_str::<NodeConfig>(
            created_config_map
                .data
                .unwrap()
                .get(FULLNODE_CONFIG_MAP_KEY)
                .unwrap(),
        )
        .unwrap();
        assert_eq!(regenerated_node_config, node_config);
    }

    #[test]
    /// Test that we can create a data volume from an existing validator data volume, and that we inherit the resource requests
    fn test_create_persistent_volume_claim() {
        let requests = Some(
            [
                ("storage".to_string(), Quantity("1Gi".to_string())),
                ("storage2".to_string(), Quantity("2Gi".to_string())),
            ]
            .iter()
            .cloned()
            .collect(),
        );
        let pvc = PersistentVolumeClaim {
            metadata: ObjectMeta {
                name: Some(APTOS_DATA_VOLUME_NAME.to_string()),
                ..ObjectMeta::default()
            },
            spec: Some(PersistentVolumeClaimSpec {
                access_modes: Some(vec!["ReadWriteOnce".to_string()]),
                resources: Some(ResourceRequirements {
                    requests,
                    ..ResourceRequirements::default()
                }),
                ..PersistentVolumeClaimSpec::default()
            }),
            ..PersistentVolumeClaim::default()
        };
        let created_pvc = create_fullnode_persistent_volume_claim(pvc.clone());

        // assert that the resources are the same
        assert_eq!(
            created_pvc.unwrap().spec.unwrap().resources,
            pvc.spec.unwrap().resources
        );
    }

    #[test]
    /// Test that the created StatefulSet and Service are connected
    fn test_create_fullnode_stateful_set_and_service_connected() {
        // top level args
        let era = 42069;
        let peer_id = PeerId::random();
        let fullnode_name = "fullnode-".to_string() + &peer_id.to_string(); // everything should be keyed on this
        let fullnode_image = "fruit.com/banana:latest".to_string();
        let fullnode_genesis_secret_name = format!("aptos-node-0-genesis-e{}", era);
        let fullnode_node_config_config_map_name = format!("{}-config", fullnode_name);

        let fullnode_stateful_set = create_fullnode_stateful_set(
            fullnode_name.clone(),
            fullnode_image,
            fullnode_genesis_secret_name,
            fullnode_node_config_config_map_name,
            get_dummy_validator_stateful_set(),
            get_dummy_validator_persistent_volume_claim(),
        )
        .unwrap();

        let fullnode_service = create_fullnode_service(fullnode_name.clone()).unwrap();

        // assert that the StatefulSet has the correct name
        assert_eq!(
            fullnode_stateful_set.metadata.name,
            Some(fullnode_name.clone())
        );
        // assert that the Service has the correct name
        assert_eq!(fullnode_service.metadata.name, Some(fullnode_name.clone()));
        // assert that the StatefulSet has a serviceName that matches the Service
        assert_eq!(
            fullnode_stateful_set.spec.unwrap().service_name,
            fullnode_name
        );
        // assert that the labels in the Service match the StatefulSet
        assert_eq!(
            fullnode_service.spec.unwrap().selector,
            fullnode_stateful_set.metadata.labels
        );
    }

    #[tokio::test]
    /// Full PFN installation test, checking that the resulting resources created are as expected
    async fn test_install_public_fullnode() {
        // top level args
        let peer_id = PeerId::random();
        let version = Version::new(0, "banana".to_string());

        // create APIs
        let stateful_set_api: Arc<MockK8sResourceApi<StatefulSet>> = Arc::new(
            MockK8sResourceApi::from_resource(get_dummy_validator_stateful_set()),
        );
        let configmap_api = Arc::new(MockK8sResourceApi::new());
        let persistent_volume_claim_api = Arc::new(MockK8sResourceApi::from_resource(
            get_dummy_validator_persistent_volume_claim(),
        ));
        let service_api = Arc::new(MockK8sResourceApi::new());

        // get the base config and mutate it
        let mut node_config = get_default_pfn_node_config();
        node_config.full_node_networks[0].identity =
            Identity::from_config(PrivateKey::generate_for_testing(), peer_id);
        let override_config = OverrideNodeConfig::new_with_default_base(node_config);

        let era = "42069".to_string();
        let namespace = "forge42069".to_string();

        let (created_peer_id, created_node) = install_public_fullnode(
            stateful_set_api,
            configmap_api,
            persistent_volume_claim_api,
            service_api,
            &version,
            &override_config,
            era,
            namespace,
            false,
            7,
        )
        .await
        .unwrap();

        // assert the created resources match some patterns
        assert_eq!(created_peer_id, peer_id);
        assert_eq!(
            created_node.name,
            format!(
                "public-fullnode-{}-{}",
                created_node.index,
                &peer_id.short_str()
            )
        );
        assert!(created_node.name.len() < 64); // This is a k8s limit
        assert_eq!(created_node.index, 7);
    }
}
