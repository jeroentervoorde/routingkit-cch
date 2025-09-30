use indicatif::ProgressIterator;
use pathfinding::prelude::dijkstra;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use routingkit_cch::{CCH, CCHMetric, CCHQuery, compute_order_inertial, shp_utils};
use std::sync::LazyLock;

const STYLE: LazyLock<indicatif::ProgressStyle> = LazyLock::new(|| {
    indicatif::ProgressStyle::with_template(
        "[{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})",
    )
    .unwrap()
});

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
#[ignore] // requires data files
fn compare_with_pathfinding() {
    let edges = shp_utils::load_edges(&"data/beijing/edges.shp").unwrap();
    let nodes = shp_utils::load_nodes(&"data/beijing/nodes.shp").unwrap();
    let shp_utils::GraphArrays {
        osmids: _,
        xs,
        ys,
        tail,
        head,
        weight,
    } = shp_utils::build_graph_arrays(&nodes, &edges).unwrap();
    let node_count = nodes.len();
    let query_count = 2000;
    let seed: u64 = 42;
    let mut rng = StdRng::seed_from_u64(seed);
    let tail = tail.into_iter().map(|x| x as u32).collect::<Vec<u32>>();
    let head = head.into_iter().map(|x| x as u32).collect::<Vec<u32>>();
    let weights = weight
        .into_iter()
        .map(|x| (x * 1e3) as u32)
        .collect::<Vec<u32>>();
    let lat = xs.into_iter().map(|x| x as f32).collect::<Vec<f32>>();
    let lon = ys.into_iter().map(|x| x as f32).collect::<Vec<f32>>();

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
