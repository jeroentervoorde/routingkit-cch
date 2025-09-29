pub mod ffi;

#[cfg(test)]
mod tests {
    use super::ffi::*;

    #[test]
    fn basic() {
        // 只是演示: 构造一个极小的图 (线形 0-1-2)
        // 有向弧 tail/head (0->1,1->0,1->2,2->1)
        let tail = vec![0, 1, 1, 2];
        let head = vec![1, 0, 2, 1];
        let node_count = 3u32;
        // 通过度启发式计算顺序 (小图与 identity 相同, 但演示 API 用法)
        let order = cch_compute_order_degree(node_count, &tail, &head);
        println!("computed order (degree heuristic): {:?}", order);
        let weights = vec![1, 1, 1, 1];
        let cch = cch_new(&order, &tail, &head, false);
        let mut metric = cch_metric_new(cch.as_ref().unwrap(), &weights);
        cch_metric_customize(metric.as_mut().unwrap());
        let mut query = cch_query_new(metric.as_ref().unwrap());

        // 第一次查询 0 -> 2
        cch_query_add_source(query.as_mut().unwrap(), 0, 0);
        cch_query_add_target(query.as_mut().unwrap(), 2, 0);
        cch_query_run(query.as_mut().unwrap());
        let dist1 = cch_query_distance(query.as_ref().unwrap());
        let path1 = cch_query_node_path(query.as_ref().unwrap());
        let arc_path1 = cch_query_arc_path(query.as_ref().unwrap());
        println!(
            "query1 distance: {} node_path: {:?} arc_path: {:?}",
            dist1, path1, arc_path1
        );

        // 复用同一个 query: reset 后执行第二次查询 2 -> 1
        cch_query_reset(query.as_mut().unwrap(), metric.as_ref().unwrap());
        cch_query_add_source(query.as_mut().unwrap(), 2, 0);
        cch_query_add_target(query.as_mut().unwrap(), 1, 0);
        cch_query_run(query.as_mut().unwrap());
        let dist2 = cch_query_distance(query.as_ref().unwrap());
        let path2 = cch_query_node_path(query.as_ref().unwrap());
        println!("query2 distance: {} path: {:?}", dist2, path2);

        // 多源示例: reset 后添加两个源 0,1 (其中 1 给一个偏移初始代价 5) 目标 2
        cch_query_reset(query.as_mut().unwrap(), metric.as_ref().unwrap());
        cch_query_add_source(query.as_mut().unwrap(), 0, 1);
        cch_query_add_source(query.as_mut().unwrap(), 1, 0); // 相当于已有一段成本
        cch_query_add_target(query.as_mut().unwrap(), 2, 0);
        cch_query_run(query.as_mut().unwrap());
        let dist3 = cch_query_distance(query.as_ref().unwrap());
        let path3 = cch_query_node_path(query.as_ref().unwrap());
        println!(
            "query3 (multi-source) distance: {} path: {:?}",
            dist3, path3
        );
    }

    #[test]
    fn parallel() {
        use rayon::prelude::*;
        // Build once
        let tail = vec![0, 1, 1, 2];
        let head = vec![1, 0, 2, 1];
        let node_count = 3u32;
        let order = cch_compute_order_degree(node_count, &tail, &head);
        println!("[parallel demo] order: {:?}", order);
        let weights = vec![1, 1, 1, 1];
        let cch = cch_new(&order, &tail, &head, false);
        let mut metric = cch_metric_new(cch.as_ref().unwrap(), &weights);
        cch_metric_customize(metric.as_mut().unwrap());
        let metric = metric.as_ref().unwrap();

        let tasks = vec![(0u32, 2u32), (2u32, 1u32), (1u32, 0u32), (0u32, 1u32)];
        let results: Vec<_> = tasks
            .par_iter()
            .map(|&(s, t)| {
                let mut q = cch_query_new(metric);
                cch_query_add_source(q.as_mut().unwrap(), s, 0);
                cch_query_add_target(q.as_mut().unwrap(), t, 0);
                cch_query_run(q.as_mut().unwrap());
                let d = cch_query_distance(q.as_ref().unwrap());
                let p = cch_query_node_path(q.as_ref().unwrap());
                (s, t, d, p)
            })
            .collect();
        println!("parallel queries:");
        for (s, t, d, p) in results {
            println!("  {} -> {} dist={} path={:?}", s, t, d, p);
        }
    }
}
