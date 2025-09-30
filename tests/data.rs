use routingkit_cch::shp_utils::{build_graph_arrays, load_edges, load_nodes};

#[test]
fn test_load_edges() {
    if let Ok(edges) = load_edges(&"data/beijing/edges.shp") {
        if !edges.is_empty() {
            println!("Loaded {} edges. Showing first 10:", edges.len());
            for (i, e) in edges.iter().take(10).enumerate() {
                println!(
                    "Edge[{i}]: fid={} u={} v={} len={} highway={:?} name={:?} oneway={:?} maxspeed={:?}",
                    e.fid, e.u, e.v, e.length, e.highway, e.name, e.oneway, e.maxspeed
                );
            }
        } else {
            println!("No edges loaded (file missing?)");
        }
    }
}

#[test]
fn test_load_nodes() {
    match load_nodes(&"data/beijing/nodes.shp") {
        Ok(nodes) => {
            if !nodes.is_empty() {
                println!("Loaded {} nodes. Showing first 10:", nodes.len());
                for (i, n) in nodes.iter().take(10).enumerate() {
                    println!(
                        "Node[{i}]: osmid={} x={:.6} y={:.6} highway={:?} ref={:?}",
                        n.osmid, n.x, n.y, n.highway, n.r#ref
                    );
                }
            } else {
                println!("No nodes loaded (file missing or no point geometries)");
            }
        }
        Err(e) => println!("Failed to load nodes: {e}"),
    }
}

#[test]
fn test_build_graph_arrays() {
    let nodes = match load_nodes(&"data/beijing/nodes.shp") {
        Ok(v) => v,
        Err(e) => {
            println!("Failed to load nodes: {e}");
            return;
        }
    };
    let edges = match load_edges(&"data/beijing/edges.shp") {
        Ok(v) => v,
        Err(e) => {
            println!("Failed to load edges: {e}");
            return;
        }
    };
    let g = match build_graph_arrays(&nodes, &edges) {
        Ok(g) => g,
        Err(e) => {
            println!("Build failed: {e}");
            return;
        }
    };

    println!("{:?}", g);
}
