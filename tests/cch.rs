use indicatif::ProgressIterator;
use pathfinding::prelude::dijkstra;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use routingkit_cch::{CCH, CCHMetric, CCHQuery, compute_order_inertial};
use std::collections::HashSet;
use std::sync::LazyLock;

const STYLE: LazyLock<indicatif::ProgressStyle> = LazyLock::new(|| {
    indicatif::ProgressStyle::with_template(
        "[{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})",
    )
    .unwrap()
});

fn build_random_grid_graph(
    node_count: usize,
    edge_count: usize,
    rng: Option<&mut StdRng>,
) -> (Vec<u32>, Vec<u32>, Vec<u32>, Vec<f32>, Vec<f32>) {
    let rng = match rng {
        Some(r) => r,
        None => &mut StdRng::from_entropy(),
    };
    let grid_size = (node_count as f32).sqrt().ceil() as u32;
    let mut edges = HashSet::new();
    while edges.len() < edge_count {
        let x1 = rng.gen_range(0..grid_size);
        let y1 = rng.gen_range(0..grid_size);
        let x2 = rng.gen_range(0..grid_size);
        let y2 = rng.gen_range(0..grid_size);
        if (x1 != x2 || y1 != y2)
            && x1 < grid_size
            && y1 < grid_size
            && x2 < grid_size
            && y2 < grid_size
        {
            let u = x1 + y1 * grid_size;
            let v = x2 + y2 * grid_size;
            if u < node_count as u32 && v < node_count as u32 {
                edges.insert((u, v));
            }
        }
    }
    let mut tail = Vec::with_capacity(edges.len());
    let mut head = Vec::with_capacity(edges.len());
    let mut weights = Vec::with_capacity(edges.len());
    for &(u, v) in &edges {
        tail.push(u);
        head.push(v);
        weights.push(rng.gen_range(1..20));
    }

    // Generate random coordinates for inertial order
    let mut lat = Vec::with_capacity(node_count);
    let mut lon = Vec::with_capacity(node_count);
    for i in 0..node_count {
        let x = (i as u32 % grid_size) as f32;
        let y = (i as u32 / grid_size) as f32;
        lat.push(y);
        lon.push(x);
    }

    (tail, head, weights, lat, lon)
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

#[ignore]
#[test]
fn compare_with_pathfinding() {
    let node_count = 10_000;
    let edge_count = 40_000;
    let query_count = 2000;
    let seed: u64 = 42;

    let mut rng = StdRng::seed_from_u64(seed);
    let (tail, head, weights, lat, lon) =
        build_random_grid_graph(node_count, edge_count, Some(&mut rng));
    eprintln!(
        "Graph has {} nodes, {} edges, with {} queries.",
        node_count,
        tail.len(),
        query_count
    );

    // Compute simple degree order as a baseline (fast). Could use inertial order with coords if provided.
    eprintln!("Computing order...");
    let order = compute_order_inertial(node_count as u32, &tail, &head, &lat, &lon);

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
        let dist_ref = result.map(|(_, cost)| cost);

        assert_eq!(
            dist_cch, dist_ref,
            "distance mismatch on query #{i} s={s} t={t}"
        );
    }
}
