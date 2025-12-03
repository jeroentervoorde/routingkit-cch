Example: reuse_cch_arc_path.rs

This example builds a tiny graph, runs a shortest-path query under one metric (A),
prints the unpacked arc path and the CCH-level arc (shortcut) path, then runs the
same query under another metric (B) and prints the unpacked arc path. It demonstrates
how `cch_arc_path()` exposes the CCH-level shortcut ids which can be reused or
inspected for alternative metrics.

Run:

```bash
cd routingkit-cch
cargo run --example reuse_cch_arc_path
```
