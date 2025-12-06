//! Example: Using PHAST to compute distances from a source to a set of targets
use routingkit_cch::{CCH, CCHMetric, CCHQuery};

fn main() {
    // Build a tiny graph: 0 -> 1 -> 2, weights 1
    let order = vec![0, 2, 1];
    let tail = vec![0, 1];
    let head = vec![1, 2];
    let weights = vec![1, 1];
    let cch = CCH::new(&order, &tail, &head, |_| {}, false);
    let mut metric = CCHMetric::new(&cch, weights.clone());
    let query = CCHQuery::new(&metric);
    // PHAST from node 0 to nodes 0, 1, 2
    let targets = vec![0, 1, 2];
    let dists = query.phast_to_targets(0, &targets);
    println!("Distances from 0: {:?}", dists); // Should print: [0, 1, 2]
}
