use indicatif::ProgressIterator;
use pathfinding::prelude::dijkstra;
use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::{Rng, SeedableRng};
use routingkit_cch::{CCH, CCHMetric, CCHQuery, compute_order_degree};
use std::collections::HashSet;
use std::sync::LazyLock;

const STYLE: LazyLock<indicatif::ProgressStyle> = LazyLock::new(|| {
    indicatif::ProgressStyle::with_template(
        "[{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})",
    )
    .unwrap()
});

// Build a strongly connected random directed graph (no mandatory reverse arcs).
// Method:
// 1. Create a random permutation and add a Hamiltonian cycle to guarantee strong connectivity.
// 2. Add random arcs until reaching target_arcs ≈ node_count * edge_factor (avg out-degree ≈ edge_factor).
// 3. Deduplicate arcs; skip self-loops; assign random weights in [1,1000). Returns (tail, head, weights).
fn build_random_strong_connected_graph(
    node_count: usize,
    edge_factor: usize,
    rng: Option<&mut StdRng>,
) -> (Vec<u32>, Vec<u32>, Vec<u32>) {
    let rng = match rng {
        Some(r) => r,
        None => &mut StdRng::from_entropy(),
    };
    assert!(node_count >= 2);
    let target_arcs = node_count * edge_factor; // 期望总有向边数 (平均 out-degree ≈ edge_factor)
    let mut arcs = HashSet::<(u32, u32)>::new();

    // 1. Hamiltonian cycle ensures strong connectivity
    let mut perm: Vec<u32> = (0..node_count as u32).collect();
    perm.shuffle(rng);
    for i in 0..node_count {
        let u = perm[i];
        let v = perm[(i + 1) % node_count];
        arcs.insert((u, v));
    }

    // 2. Add random arcs until target size
    while arcs.len() < target_arcs {
        let u = rng.gen_range(0..node_count as u32);
        let v = rng.gen_range(0..node_count as u32);
        if u == v {
            continue;
        }
        arcs.insert((u, v));
    }

    // 3. Materialize vectors
    let mut tail = Vec::with_capacity(arcs.len());
    let mut head = Vec::with_capacity(arcs.len());
    let mut weights = Vec::with_capacity(arcs.len());
    for (u, v) in arcs.iter() {
        tail.push(*u);
        head.push(*v);
        weights.push(rng.gen_range(1..1000));
    }
    (tail, head, weights)
}

// Helper to build symmetric directed graph (each undirected edge becomes two arcs) with random weights.
// Returns (tail, head, weights)
fn build_random_symmetric_graph(
    node_count: usize,
    edge_factor: usize,
    rng: Option<&mut StdRng>,
) -> (Vec<u32>, Vec<u32>, Vec<u32>) {
    let rng = match rng {
        Some(r) => r,
        None => &mut StdRng::from_entropy(),
    };
    assert!(node_count >= 2);
    // target average undirected degree = edge_factor
    let target_edges = node_count * edge_factor / 2; // undirected edges
    let mut edges = HashSet::<(u32, u32)>::new();

    // ensure connectivity by a random spanning tree
    for u in 1..node_count as u32 {
        // connect u to random <u
        let v = rng.gen_range(0..u);
        let a = (u, v);
        let b = (v, u);
        if u != v {
            edges.insert(a);
            edges.insert(b);
        }
    }

    while edges.len() / 2 < target_edges {
        // each undirected contributes 2
        let u = rng.gen_range(0..node_count as u32);
        let v = rng.gen_range(0..node_count as u32);
        if u == v {
            continue;
        }
        edges.insert((u, v));
        edges.insert((v, u));
    }

    let mut tail = Vec::with_capacity(edges.len());
    let mut head = Vec::with_capacity(edges.len());
    let mut weights = Vec::with_capacity(edges.len());

    for (u, v) in edges.iter() {
        tail.push(*u);
        head.push(*v);
        weights.push(rng.gen_range(1..1000));
    }

    (tail, head, weights)
}

fn random_pairs(
    node_count: usize,
    query_count: usize,
    rng: Option<&mut StdRng>,
) -> Vec<(u32, u32)> {
    let rng = match rng {
        Some(r) => r,
        None => &mut StdRng::from_entropy(),
    };
    let mut pairs = Vec::with_capacity(query_count);
    for _ in 0..query_count {
        let s = rng.gen_range(0..node_count as u32);
        let mut t = rng.gen_range(0..node_count as u32);
        while t == s {
            t = rng.gen_range(0..node_count as u32);
        }
        pairs.push((s, t));
    }
    pairs
}

#[test]
fn compare_with_pathfinding() {
    let node_count = 20_000;
    let edge_factor = 4;
    let query_count = 2000;
    let seed: u64 = 42;

    let mut rng = StdRng::seed_from_u64(seed);
    let (tail, head, weights) =
        build_random_symmetric_graph(node_count, edge_factor, Some(&mut rng));
    eprintln!(
        "Graph has {} nodes, {} edges, with {} queries.",
        node_count,
        tail.len(),
        query_count
    );

    // Compute simple degree order as a baseline (fast). Could use inertial order with coords if provided.
    eprintln!("Computing order...");
    let order = compute_order_degree(node_count as u32, &tail, &head);

    eprintln!("Building CCH...");
    let cch = CCH::new(&order, &tail, &head, false);
    eprintln!("Building metric + customization...");
    let metric = CCHMetric::parallel_new(&cch, &weights, 0);

    // Build adjacency for reference dijkstra using pathfinding crate
    eprintln!("Building adjacency for pathfinding reference...");
    let mut adj = vec![Vec::<(u32, u32)>::new(); node_count];
    for i in 0..tail.len() {
        adj[tail[i] as usize].push((head[i], weights[i]));
    }

    // Preselect random queries (s,t) distinct
    eprintln!("Selecting random queries...");
    let pairs = random_pairs(node_count, query_count, Some(&mut rng));

    eprintln!("Running {} queries...", query_count);
    let mut query = CCHQuery::new(&metric);
    for (i, &(s, t)) in pairs.iter().enumerate().progress_with_style(STYLE.clone()) {
        query.reset();
        query.add_source(s, 0);
        query.add_target(t, 0);
        query.run();
        let dist_cch = query.distance();

        // pathfinding dijkstra
        let result = dijkstra(&s, |&u| adj[u as usize].iter().cloned(), |&u| u == t);
        let dist_ref = result.map(|(_, cost)| cost).expect("ref path should exist");

        assert_eq!(
            dist_cch, dist_ref,
            "distance mismatch on query #{i} s={s} t={t}"
        );
    }
}
