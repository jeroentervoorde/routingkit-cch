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
    pub fn new(metric: &'a CCHMetric<'a>) -> Self {
        let query = unsafe { cch_query_new(&metric.inner) };
        CCHQuery {
            inner: query,
            metric: metric,
            runned: false,
            _marker: std::marker::PhantomData,
        }
    }

    pub fn reset(&mut self) {
        unsafe {
            cch_query_reset(
                self.inner.as_mut().unwrap(),
                self.metric.inner.as_ref().unwrap(),
            );
        }
        self.runned = false;
    }

    pub fn add_source(&mut self, s: u32, dist: u32) {
        unsafe {
            cch_query_add_source(self.inner.as_mut().unwrap(), s, dist);
        }
    }

    pub fn add_target(&mut self, t: u32, dist: u32) {
        unsafe {
            cch_query_add_target(self.inner.as_mut().unwrap(), t, dist);
        }
    }

    pub fn run(&mut self) {
        unsafe {
            cch_query_run(self.inner.as_mut().unwrap());
        }
        self.runned = true;
    }

    pub fn distance(&self) -> u32 {
        if !self.runned {
            panic!("Query distance requested before run()");
        }
        unsafe { cch_query_distance(self.inner.as_ref().unwrap()) }
    }

    pub fn node_path(&self) -> Vec<u32> {
        if !self.runned {
            panic!("Query node_path requested before run()");
        }
        unsafe { cch_query_node_path(self.inner.as_ref().unwrap()) }
    }

    pub fn arc_path(&self) -> Vec<u32> {
        if !self.runned {
            panic!("Query arc_path requested before run()");
        }
        unsafe { cch_query_arc_path(self.inner.as_ref().unwrap()) }
    }
}
