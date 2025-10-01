# routingkit-cch

![Crates.io](https://img.shields.io/crates/v/routingkit-cch.svg)
![Docs](https://img.shields.io/docsrs/routingkit-cch)
![License](https://img.shields.io/crates/l/routingkit-cch)

Rust bindings for the Customizable Contraction Hierarchies (CCH) implementation from [RoutingKit](https://github.com/RoutingKit/RoutingKit). CCH is a three‑phase shortest path acceleration technique for large directed graphs (e.g. road networks) that allows fast re-weighting while keeping very low query latency.

## Why CCH?
Traditional Contraction Hierarchies (CH) give extremely fast queries but require rebuilding (slow) when weights change. CCH cleanly separates:
1. **Preprocessing** – topology + node ordering only (expensive but weight agnostic)
2. **Customization** – inject / update edge weights (fast, parallelizable)
3. **Query** – run many multi-source / multi-target shortest paths (microseconds)

This enables scenarios like: per-user cost profiles, frequent traffic updates, dynamic restrictions, or “what-if” analysis without full rebuilds.

## Features
- Safe(ish) ergonomic Rust API on top of proven C++ core via `cxx`.
- Build indices from raw edge lists (tail/head arrays).
- Ordering helpers: nested dissection (inertial separator heuristic) and degree fallback.
- Sequential & parallel customization.
- Reusable query object supporting multi-source / multi-target searches.
- Path extraction: node sequence & original arc id sequence.
- Thread-safe sharing of immutable structures (`CCH`, `CCHMetric`).

## Installation
Stable release from [crates.io](https://crates.io/crates/routingkit-cch):
```toml
[dependencies]
routingkit-cch = "0.1"
```

Or track the repository:
```toml
[dependencies]
routingkit-cch = { git = "https://github.com/HellOwhatAs/routingkit-cch" }
```
For the git form ensure the `RoutingKit` submodule is present:
```bash
git submodule update --init --recursive
```
Requirements: C++17 compiler (MSVC / gcc / clang).  
OpenMP is enabled automatically by the build script; ensure your toolchain provides an OpenMP runtime (e.g. install libomp on macOS).
Without it the build may fail or customization will be single‑threaded in a future fallback.  

## Quick Start
```rust
use routingkit_cch::{CCH, CCHMetric, CCHQuery, compute_order_degree};

// Simple chain 0 -> 1 -> 2 -> 3
let tail = vec![0,1,2];
let head = vec![1,2,3];
let weights = vec![10,5,7];
let node_count = 4;

let order = compute_order_degree(node_count, &tail, &head); // O(m log n) light heuristic
let cch = CCH::new(&order, &tail, &head, false);
let metric = CCHMetric::new(&cch, weights.clone()); // customization inside

let mut q = CCHQuery::new(&metric);
q.add_source(0, 0);
q.add_target(3, 0);
q.run();
assert_eq!(q.distance(), Some(22));
```

## Building an Order
For production use prefer the inertial nested dissection based order:
```rust
use routingkit_cch::compute_order_inertial;
let order = compute_order_inertial(node_count, &tail, &head, &latitude, &longitude);
```
Better separators -> faster customization & queries. External advanced orderers (e.g. FlowCutter) could be integrated offline; you only need to supply the permutation.

## (Parallel) Customization
```rust
use routingkit_cch::{CCH, CCHMetric};
let metric = CCHMetric::new(&cch, weights.clone()); // single thread
let metric = CCHMetric::parallel_new(&cch, weights.clone(), 0); // 0 -> auto threads
```
Use when graphs are large enough; for tiny graphs overhead may outweigh benefit.

## Incremental (Partial) Weight Updates
If only a small subset of arc weights change (e.g. traffic incidents), you can avoid a full re-customization:
```rust
let mut metric = CCHMetric::parallel_new(&cch, weights.clone(), 0);
let mut updater = CCHMetricPartialUpdater::new(&cch);
// ... run queries ...
// Update two arcs (id 12 -> 900, id 77 -> 450)
updater.apply(&mut metric, &BTreeMap::from_iter([(12, 900), (77, 450)]));
// New queries now see updated weights.
```
Internally this uses RoutingKit's `CustomizableContractionHierarchyPartialCustomization`.

## Path Reconstruction
After `run()`:
- `distance()` -> `Option<u32>` (None = unreachable)
- `node_path()` -> `Vec<node_id>` (empty = unreachable)
- `arc_path()` -> `Vec<original_arc_id>` (empty = unreachable)

## Thread Safety
| Type        | Send | Sync | Notes                                         |
| ----------- | ---- | ---- | --------------------------------------------- |
| `CCH`       | yes  | yes  | Immutable after build                         |
| `CCHMetric` | yes  | yes  | Read-only after customization                 |
| `CCHQuery`  | yes  | no   | Internal mutable labels; reuse with `reset()` |

Create separate queries per thread for parallel batch querying.
