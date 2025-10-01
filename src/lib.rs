#![doc = include_str!("../README.md")]

// Expose test support utilities
pub mod shp_utils;

#[cxx::bridge]
mod ffi {
    extern "C++" {
        include!("routingkit_cch_wrapper.h");

        type CCH; // CustomizableContractionHierarchy
        type CCHMetric; // CustomizableContractionHierarchyMetric
        type CCHQuery; // CustomizableContractionHierarchyQuery
        type CCHPartial; // CustomizableContractionHierarchyPartialCustomization

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

        // Partial customization API
        unsafe fn cch_partial_new(cch: &CCH) -> UniquePtr<CCHPartial>;
        unsafe fn cch_partial_reset(partial: Pin<&mut CCHPartial>);
        unsafe fn cch_partial_update_arc(partial: Pin<&mut CCHPartial>, arc: u32);
        unsafe fn cch_partial_customize(partial: Pin<&mut CCHPartial>, metric: Pin<&mut CCHMetric>);

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
    weights: Box<[u32]>, // owned stable backing storage (no reallocation)
    cch: &'a CCH,
}

impl<'a> CCHMetric<'a> {
    /// Create and customize a metric (weight binding) for a given [`CCH`].
    ///
    /// Owns the weight vector so that future partial updates can safely mutate it.
    /// The C++ side stores only a raw pointer; it is valid for the lifetime of `self`.
    pub fn new(cch: &'a CCH, weights: Vec<u32>) -> Self {
        let boxed: Box<[u32]> = weights.into_boxed_slice();
        // Temporarily borrow as slice for FFI creation
        let metric = unsafe {
            let mut metric = cch_metric_new(&cch.inner, &boxed);
            cch_metric_customize(metric.as_mut().unwrap());
            metric
        };
        CCHMetric {
            inner: metric,
            weights: boxed,
            cch,
        }
    }

    /// Parallel customization variant.
    pub fn parallel_new(cch: &'a CCH, weights: Vec<u32>, thread_count: u32) -> Self {
        let boxed: Box<[u32]> = weights.into_boxed_slice();
        let metric = unsafe {
            let mut metric = cch_metric_new(&cch.inner, &boxed);
            cch_metric_parallel_customize(metric.as_mut().unwrap(), thread_count);
            metric
        };
        CCHMetric {
            inner: metric,
            weights: boxed,
            cch,
        }
    }

    /// weights slice
    pub fn weights(&self) -> &[u32] {
        &self.weights
    }
}

/// Reusable partial customization helper. Construct once if you perform many small incremental
/// weight updates; this avoids reallocating O(m) internal buffers each call.
pub struct CCHMetricPartialUpdater<'a> {
    partial: UniquePtr<ffi::CCHPartial>,
    cch: &'a CCH,
    _marker: std::marker::PhantomData<std::cell::Cell<()>>, // Not Sync
}

impl<'a> CCHMetricPartialUpdater<'a> {
    /// Create a reusable partial updater bound to a given CCH. You can then apply it to any
    /// metric built from the same CCH (even if you rebuild metrics with different weight sets).
    pub fn new(cch: &'a CCH) -> Self {
        let partial = unsafe { cch_partial_new(cch.inner.as_ref().unwrap()) };
        CCHMetricPartialUpdater {
            partial,
            cch,
            _marker: std::marker::PhantomData,
        }
    }

    /// Apply a batch of (arc, new_weight) updates to the given metric and run partial customize.
    pub fn apply<T>(&mut self, metric: &mut CCHMetric<'a>, updates: &T)
    where
        T: for<'b> std::ops::Index<&'b u32, Output = u32>,
        for<'b> &'b T: IntoIterator<Item = (&'b u32, &'b u32)>,
    {
        assert!(
            std::ptr::eq(metric.cch, self.cch),
            "CCHMetricPartialUpdater must be used with metrics from the same CCH"
        );
        for (k, v) in updates {
            metric.weights[*k as usize] = *v; // safe: length invariant unchanged (Box<[u32]>)
        }
        unsafe {
            cch_partial_reset(self.partial.as_mut().unwrap());
            for (k, _) in updates {
                cch_partial_update_arc(self.partial.as_mut().unwrap(), *k);
            }
            cch_partial_customize(
                self.partial.as_mut().unwrap(),
                metric.inner.as_mut().unwrap(),
            );
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
