use routingkit_cch::{CCH, CCHMetric};

fn main() {
    // Small example graph: 4 nodes, edges: 0->1 (1), 1->2 (1), 0->2 (3), 2->3 (1)
    let order = vec![3u32, 2, 1, 0, 4, 5, 6];
    let tail = vec![0u32, 1, 2, 3, 4, 5, 6];
    let head = vec![1u32, 2, 3, 4, 5, 6, 0];
    let weights_a = vec![1u32, 1, 3, 1, 5, 3, 1]; // metric A
    let weights_b = vec![2u32, 2, 2, 2, 2, 2, 2]; // metric B (penalizes some arcs differently)

    // Build CCH
    let cch = CCH::new(&order, &tail, &head, |_| {}, false);

    // Metric A
    let metric_a = CCHMetric::new(&cch, weights_a);
    let mut q = routingkit_cch::CCHQuery::new(&metric_a);
    q.add_source(0, 0);
    q.add_target(6, 0);
    let res = q.run();
    let dist_a = res.distance();
    let arc_path_unpacked = res.arc_path();
    let cch_arc_path = res.cch_arc_path();
    println!(
        "Metric A: distance={:?}, arc_path={:?}",
        dist_a, arc_path_unpacked
    );
    println!("CCH arc path (shortcuts): {:?}", cch_arc_path);

    // Now reuse cch_arc_path under metric B by unpacking via the CCH's unpacking (arc_path)
    // Build metric B and a fresh query to demonstrate re-evaluation is possible.
    let metric_b = CCHMetric::new(&cch, weights_b);

    // Instead of re-running a query, unpack the same CCH-level path under metric B.
    let unpacked_under_b = res.unpack_arc_path_with_metric(&metric_b);
    println!(
        "Unpacked path under Metric B (without rerunning): {:?}",
        unpacked_under_b
    );

    // Compute the weight of the CCH-level path under metric B directly (no unpacking needed).
    let weight_cch_path_b = metric_b.weight_of_cch_arc_path(&cch_arc_path, &metric_b);
    println!(
        "Weight of CCH-level path under Metric B (summed): {}",
        weight_cch_path_b
    );

    // Note: This example demonstrates `cch_arc_path()` and the new
    // `unpack_arc_path_with_metric()` helper that allows re-evaluating the same
    // CCH-level path under a different metric without re-running the CH search.
}
