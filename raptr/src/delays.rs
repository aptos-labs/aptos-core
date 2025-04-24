use crate::framework::NodeId;
use rand::{rngs::SmallRng, thread_rng, Rng, SeedableRng};
use rand_distr::Distribution;
use std::{
    cmp::{max, min},
    collections::{hash_map::DefaultHasher, BTreeMap, BTreeSet},
    hash::{Hash, Hasher},
    sync::Arc,
};

pub trait DelayFunction: Fn(NodeId, NodeId) -> f64 + Clone + Send + Sync + 'static {}

impl<F> DelayFunction for F where F: Fn(NodeId, NodeId) -> f64 + Clone + Send + Sync + 'static {}

pub fn uniformly_random_delay(
    distr: impl Distribution<f64> + Send + Sync + Copy + 'static,
) -> impl DelayFunction {
    move |_from, _to| thread_rng().sample(distr)
}

pub fn spacial_delay_2d(
    max_distr: impl Distribution<f64> + Send + Sync + Copy + 'static,
) -> impl DelayFunction {
    let sqrt2 = f64::sqrt(2.);
    move |from: NodeId, to| {
        let from_coordinate = coordinate_2d_from_hash(from);
        let to_coordinate = coordinate_2d_from_hash(to);
        distance_2d(from_coordinate, to_coordinate) / sqrt2 * thread_rng().sample(max_distr)
    }
}

/// `base_distr` is sampled once per pair of nodes.
/// `mul_noise_distr` and `add_noise_distr` are sampled for each message.
/// The delay is computed as `base * mul_noise + add_noise`.
pub fn heterogeneous_symmetric_delay(
    link_base_distr: impl Distribution<f64> + Send + Sync + Copy + 'static,
    mul_noise_distr: impl Distribution<f64> + Send + Sync + Copy + 'static,
    add_noise_distr: impl Distribution<f64> + Send + Sync + Copy + 'static,
) -> impl DelayFunction {
    move |from: NodeId, to: NodeId| {
        if from == to {
            // TODO: check if 0 delays actually break anything.
            return 0.00001;
        }

        let mut base_seed = [0; 16];
        base_seed[..8].copy_from_slice(&hash((min(from, to), max(from, to))).to_le_bytes());
        let mut base_rng = SmallRng::from_seed(base_seed);
        let base = base_rng.sample(link_base_distr);

        let mul_noise = thread_rng().sample(mul_noise_distr);
        let add_noise = thread_rng().sample(add_noise_distr);

        base * mul_noise + add_noise
    }
}

pub fn clustered_delay(
    within_cluster_distr: impl Distribution<f64> + Send + Sync + Copy + 'static,
    between_cluster_distr: impl Distribution<f64> + Send + Sync + Copy + 'static,
    clusters: Vec<Vec<NodeId>>,
) -> impl DelayFunction {
    // Perform a sanity check that no node is missing or present in multiple clusters.
    let max_id = clusters.iter().flatten().max().unwrap();
    let n_nodes = clusters.iter().map(|cluster| cluster.len()).sum::<usize>();
    let n_unique_nodes = clusters.iter().flatten().collect::<BTreeSet<_>>().len();
    assert_eq!(n_nodes, n_unique_nodes);
    assert_eq!(n_nodes, max_id + 1);

    let clusters: Arc<BTreeMap<NodeId, usize>> = Arc::new(
        clusters
            .into_iter()
            .enumerate()
            .flat_map(|(cluster, nodes)| nodes.into_iter().map(move |node| (node, cluster)))
            .collect(),
    );

    move |from: NodeId, to: NodeId| {
        let from_cluster = clusters.get(&from).unwrap();
        let to_cluster = clusters.get(&to).unwrap();
        if from_cluster == to_cluster {
            thread_rng().sample(within_cluster_distr)
        } else {
            thread_rng().sample(between_cluster_distr)
        }
    }
}

fn hash<T: Hash>(value: T) -> u64 {
    let mut state = DefaultHasher::new();
    value.hash(&mut state);
    state.finish()
}

fn coordinate_2d_from_hash(node: NodeId) -> (f64, f64) {
    let h = hash(node);
    (
        (h % 1000) as f64 / 1000.0,
        ((h / 1000) % 1000) as f64 / 1000.0,
    )
}

fn sqr(x: f64) -> f64 {
    x * x
}

fn distance_2d(a: (f64, f64), b: (f64, f64)) -> f64 {
    f64::sqrt(sqr(a.0 - b.0) + sqr(a.1 - b.1))
}
