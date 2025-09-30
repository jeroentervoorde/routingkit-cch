//! High level Rust wrapper around RoutingKit's Customizable Contraction Hierarchies (CCH).
//!
//! Customizable Contraction Hierarchies accelerate repeated shortest-path queries on large
//! directed road graphs while allowing comparatively fast re-weighting. A CCH based workflow
//! has three phases:
//!
//! 1. Preprocessing (build the structural index from topology + a node order)
//! 2. Customization (inject current arc weights, possibly in parallel)
//! 3. Query (run many multi-source / multi-target shortest path searches)
//!
//! The heavy preprocessing only depends on the graph (tail/head) and a fill‑in reducing node
//! permutation. After that you can cheaply rebuild metrics for different weight vectors
//! (e.g. per user profile or after live traffic updates) by re-running customization.
//!
//! Key types exposed:
//! * [`CCH`] – immutable structural index (topology + order + internal upward/downward graph)
//! * [`CCHMetric`] – binds a weight array to a CCH and performs (parallel) customization
//! * [`CCHQuery`] – reusable query object supporting multi-source & multi-target searches
//!
//! Typical usage (self‑contained runnable example):
//! ```rust
//! use routingkit_cch::{CCH, CCHMetric, CCHQuery, compute_order_degree};
//!
//! // Small toy graph: 0 -> 1 -> 2 -> 3
//! let tail = vec![0, 1, 2];
//! let head = vec![1, 2, 3];
//! let weights = vec![10, 5, 7]; // total 22 from 0 to 3
//! let node_count = 4u32;
//!
//! // 1) Compute a (cheap) order; for real data prefer compute_order_inertial (requires lat,lon).
//! let order = compute_order_degree(node_count, &tail, &head);
//! let cch = CCH::new(&order, &tail, &head, false);
//!
//! // 2) Bind weights & customize (done inside CCHMetric::new here).
//! let metric = CCHMetric::new(&cch, &weights);
//!
//! // 3) Run a shortest path query 0 -> 3.
//! let mut q = CCHQuery::new(&metric);
//! q.add_source(0, 0);
//! q.add_target(3, 0);
//! q.run();
//! assert_eq!(q.distance(), Some(22));
//! let node_path = q.node_path();
//! assert_eq!(node_path, vec![0, 1, 2, 3]);
//! let arc_path = q.arc_path();
//! assert_eq!(arc_path, vec![0, 1, 2]);
//! ```
//!
//! Thread-safety summary:
//! * [`CCH`] & [`CCHMetric`] are `Send + Sync` after construction/customization (read-only)
//! * [`CCHQuery`] is `Send` only (internal mutable labels) – never share it concurrently.
//!
//! For theory & background see the original publication: Dibbelt / Strasser / Wagner (JEA 2016).
//! The design of this wrapper intentionally mirrors the C++ API while using lifetimes to tie
//! metrics to underlying weight storage.
// Expose test support utilities
pub mod shp_utils;

#[cxx::bridge]
mod ffi {
    extern "C++" {
        include!("routingkit_cch_wrapper.h");

        type CCH; // CustomizableContractionHierarchy
        type CCHMetric; // CustomizableContractionHierarchyMetric
        type CCHQuery; // CustomizableContractionHierarchyQuery

        /// Build a Customizable Contraction Hierarchy.
        /// Arguments:
        /// * order: node permutation (size = number of nodes)
        /// * tail/head: directed arc lists (same length)
        /// * filter_always_inf_arcs: if true, arcs with infinite weight placeholders are stripped
        unsafe fn cch_new(
            order: &[u32],
            tail: &[u32],
            head: &[u32],
            filter_always_inf_arcs: bool,
        ) -> UniquePtr<CCH>;

        /// Create a metric (weights binding) for an existing CCH.
        /// Copies the weight slice once; length must equal arc count.
        unsafe fn cch_metric_new(cch: &CCH, weights: &[u32]) -> UniquePtr<CCHMetric>;

        /// Run customization to compute upward/downward shortcut weights.
        /// Must be called after creating a metric and before queries.
        /// Cost: Depends on separator quality; usually near-linear in m * small constant; may allocate temporary buffers.
        unsafe fn cch_metric_customize(metric: Pin<&mut CCHMetric>);

        /// Parallel customization; thread_count==0 picks an internal default (#procs if OpenMP, else 1).
        unsafe fn cch_metric_parallel_customize(metric: Pin<&mut CCHMetric>, thread_count: u32);

        /// Allocate a new reusable query object bound to a metric.
        unsafe fn cch_query_new(metric: &CCHMetric) -> UniquePtr<CCHQuery>;

        /// Reset a query so it can be reused with the (same or updated) metric.
        /// Clears internal state, sources/targets, distance labels.
        unsafe fn cch_query_reset(query: Pin<&mut CCHQuery>, metric: &CCHMetric);

        /// Add a source node with initial distance (0 for standard shortest path).
        /// Multiple sources allowed (multi-source query).
        unsafe fn cch_query_add_source(query: Pin<&mut CCHQuery>, s: u32, dist: u32);

        /// Add a target node with initial distance (0 typical).
        /// Multiple targets allowed (multi-target query).
        unsafe fn cch_query_add_target(query: Pin<&mut CCHQuery>, t: u32, dist: u32);

        /// Execute the shortest path search (multi-source / multi-target if several added).
        /// Must be called after adding at least one source & target.
        unsafe fn cch_query_run(query: Pin<&mut CCHQuery>);

        /// Get shortest distance after run(). Undefined if run() not called.
        unsafe fn cch_query_distance(query: &CCHQuery) -> u32;

        /// Extract the node path for the current query result.
        /// Reconstructs path; may traverse parent pointers.
        unsafe fn cch_query_node_path(query: &CCHQuery) -> Vec<u32>;

        /// Extract the arc (edge) path corresponding to the shortest path.
        /// Each entry is an original arc id (after shortcut unpacking).
        unsafe fn cch_query_arc_path(query: &CCHQuery) -> Vec<u32>;
    }

    unsafe extern "C++" {
        include!("routingkit_cch_wrapper.h");
        /// Compute a high-quality nested dissection order using inertial flow separators.
        /// Inputs:
        /// * node_count: number of nodes
        /// * tail/head: directed arcs (treated as undirected for ordering)
        /// * latitude/longitude: per-node coords (len = node_count)
        /// Returns: order permutation (position -> node id).
        fn cch_compute_order_inertial(
            node_count: u32,
            tail: &[u32],
            head: &[u32],
            latitude: &[f32],
            longitude: &[f32],
        ) -> Vec<u32>;

        /// Fast fallback order: nodes sorted by (degree, id) ascending.
        /// Lower quality than nested dissection but zero extra data needed.
        fn cch_compute_order_degree(node_count: u32, tail: &[u32], head: &[u32]) -> Vec<u32>;
    }
}

// Thread-safety markers
// Safety rationale:
// - CCH (CustomizableContractionHierarchy) after construction is immutable.
// - CCHMetric after successful customize() is read-only for queries; underlying RoutingKit code
//   does not mutate metric during query operations (queries keep their own state).
// - CCHQuery holds mutable per-query state and must not be shared across threads concurrently;
//   we allow moving it between threads (Send) but not Sync.
// If RoutingKit later introduces internal mutation or lazy caching inside CCHMetric, these impls
// would need re-audit.
unsafe impl Send for ffi::CCH {}
unsafe impl Sync for ffi::CCH {}
unsafe impl Send for ffi::CCHMetric {}
unsafe impl Sync for ffi::CCHMetric {}
unsafe impl Send for ffi::CCHQuery {}
// (No Sync for CCHQuery)

// Rust wrapper over FFI
use cxx::UniquePtr;
use ffi::*;

pub use ffi::{
    cch_compute_order_degree as compute_order_degree,
    cch_compute_order_inertial as compute_order_inertial,
};

pub struct CCH {
    inner: UniquePtr<ffi::CCH>,
}

impl CCH {
    /// Construct a new immutable Customizable Contraction Hierarchy index.
    ///
    /// Parameters:
    /// * `order` – permutation of node ids (length = node count) produced by a fill‑in reducing
    ///   nested dissection heuristic (e.g. [`compute_order_inertial`]) or a lightweight fallback.
    /// * `tail`, `head` – parallel arrays encoding each directed arc `i` as `(tail[i], head[i])`.
    ///   The ordering routine treats them as undirected; here they stay directed for queries.
    /// * `filter_always_inf_arcs` – if `true`, arcs whose weight will always be interpreted as
    ///   an application defined 'infinity' placeholder may be removed during construction to
    ///   reduce index size. (Typically keep `false` unless you prepared such a list.)
    ///
    /// Cost: preprocessing is more expensive than a single customization but usually far cheaper
    /// than building a full classical CH of the same quality. Construction copies the input
    /// slices; you may drop them afterwards.
    ///
    /// Thread-safety: resulting object is `Send + Sync` and read-only.
    ///
    /// Panics: never (undefined behavior if input slices have inconsistent lengths – guarded by `cxx`).
    pub fn new(order: &[u32], tail: &[u32], head: &[u32], filter_always_inf_arcs: bool) -> Self {
        let cch = unsafe { cch_new(order, tail, head, filter_always_inf_arcs) };
        CCH { inner: cch }
    }
}

pub struct CCHMetric<'a> {
    inner: UniquePtr<ffi::CCHMetric>,
    // Borrow both the CCH and the weight slice (zero-copy). The weight memory must
    // outlive the metric because C++ now only keeps a raw pointer.
    _marker: std::marker::PhantomData<(&'a CCH, &'a [u32])>,
}

impl<'a> CCHMetric<'a> {
    /// Create and customize a metric (weight binding) for a given [`CCH`].
    ///
    /// This allocates internal arrays and immediately runs the (sequential) customization step,
    /// computing upward/downward shortcut weights. The provided `weights` slice must have length
    /// equal to the number of original arcs (`tail.len()`). The slice is *not* copied: the C++
    /// code keeps a raw pointer. Therefore the caller must guarantee that `weights` outlives the
    /// metric (lifetime `'a`). If you modify entries of `weights` afterwards they are **not**
    /// reflected until you rebuild or (future) recustomize support is added in Rust – currently
    /// the safe wrapper does not expose an incremental update API.
    ///
    /// For multi-core machines consider [`CCHMetric::parallel_new`] which invokes the OpenMP based
    /// parallel customization routine.
    ///
    /// Cost: near-linear in arc count times a small constant dependent on separator quality.
    ///
    /// Panics: never (UB if `weights` length mismatches underlying CCH – prevented by C++ asserts).
    pub fn new(cch: &'a CCH, weights: &'a [u32]) -> Self {
        let metric = unsafe {
            let mut metric = cch_metric_new(&cch.inner, weights);
            cch_metric_customize(metric.as_mut().unwrap());
            metric
        };
        CCHMetric {
            inner: metric,
            _marker: std::marker::PhantomData,
        }
    }

    /// Like [`CCHMetric::new`] but performs the customization using the parallel routine.
    ///
    /// `thread_count` selects how many worker threads the underlying OpenMP implementation may
    /// utilize. Use `0` to let the library pick a default (usually the hardware concurrency).
    /// Parallel speedups saturate quickly; for small graphs the overhead can dominate.
    ///
    /// Safety & lifetime invariants match [`CCHMetric::new`].
    pub fn parallel_new(cch: &'a CCH, weights: &'a [u32], thread_count: u32) -> Self {
        let metric = unsafe {
            let mut metric = cch_metric_new(&cch.inner, weights);
            cch_metric_parallel_customize(metric.as_mut().unwrap(), thread_count);
            metric
        };
        CCHMetric {
            inner: metric,
            _marker: std::marker::PhantomData,
        }
    }
}

pub struct CCHQuery<'a> {
    inner: UniquePtr<ffi::CCHQuery>,
    metric: &'a CCHMetric<'a>,
    runned: bool,
    _marker: std::marker::PhantomData<std::cell::Cell<()>>, // Not Sync
}

impl<'a> CCHQuery<'a> {
    /// Allocate a new reusable shortest-path query bound to a given customized [`CCHMetric`].
    ///
    /// The query object stores its own frontier / label buffers and can be reset and reused for
    /// many (s, t) pairs or multi-source / multi-target batches. You may have multiple query
    /// objects referencing the same metric concurrently (read-only access to metric data).
    ///
    /// Thread-safety: `Send` but not `Sync`; do not mutate from multiple threads simultaneously.
    pub fn new(metric: &'a CCHMetric<'a>) -> Self {
        let query = unsafe { cch_query_new(&metric.inner) };
        CCHQuery {
            inner: query,
            metric: metric,
            runned: false,
            _marker: std::marker::PhantomData,
        }
    }

    /// Reset internal state (clears sources, targets, labels) so the object can be reused.
    ///
    /// Call this before configuring a new multi-source / multi-target batch. Does *not* change
    /// the bound metric pointer – create a new query if you need a different metric.
    pub fn reset(&mut self) {
        unsafe {
            cch_query_reset(
                self.inner.as_mut().unwrap(),
                self.metric.inner.as_ref().unwrap(),
            );
        }
        self.runned = false;
    }

    /// Add a source node with an initial distance (normally 0). Multiple calls allow a multi-
    /// source query. Distances let you model already-traversed partial paths.
    pub fn add_source(&mut self, s: u32, dist: u32) {
        unsafe {
            cch_query_add_source(self.inner.as_mut().unwrap(), s, dist);
        }
    }

    /// Add a target node with an initial distance (normally 0). Multiple calls allow multi-target
    /// queries; the algorithm stops when the frontiers settle the optimal distance to any target.
    pub fn add_target(&mut self, t: u32, dist: u32) {
        unsafe {
            cch_query_add_target(self.inner.as_mut().unwrap(), t, dist);
        }
    }

    /// Execute the forward/backward upward/downward search to settle the shortest path between the
    /// added sources and targets. Must be called after at least one source and one target.
    pub fn run(&mut self) {
        unsafe {
            cch_query_run(self.inner.as_mut().unwrap());
        }
        self.runned = true;
    }

    /// Return the shortest path distance after [`CCHQuery::run`].
    ///
    /// Returns `None` if no target is reachable (internally distance equals `i32::MAX`).
    ///
    /// Panics: if called before `run()`.
    pub fn distance(&self) -> Option<u32> {
        if !self.runned {
            panic!("Query distance requested before run()");
        }
        let res = unsafe { cch_query_distance(self.inner.as_ref().unwrap()) };
        if res == (i32::MAX as u32) {
            None
        } else {
            Some(res)
        }
    }

    /// Reconstruct and return the node id sequence of the current best path.
    ///
    /// Returns empty vec if no target is reachable.
    ///
    /// Panics: if called before `run()`. Returns an empty vector only if source==target and the
    /// implementation chooses to represent a trivial path that way (normally length>=1).
    pub fn node_path(&self) -> Vec<u32> {
        if !self.runned {
            panic!("Query node_path requested before run()");
        }
        unsafe { cch_query_node_path(self.inner.as_ref().unwrap()) }
    }

    /// Reconstruct and return the original arc ids along the shortest path (after unpacking
    /// shortcuts). Useful if you need per-arc attributes (speed limits, geometry). Order matches
    /// the traversal direction from a chosen source to target.
    ///
    /// Returns empty vec if no target is reachable.
    ///
    /// Panics: if called before `run()`.
    pub fn arc_path(&self) -> Vec<u32> {
        if !self.runned {
            panic!("Query arc_path requested before run()");
        }
        unsafe { cch_query_arc_path(self.inner.as_ref().unwrap()) }
    }
}
