use crate::{
    CCH, CCHMetric, CCHMetricPartialUpdater, CCHQuery, CCHQueryResult, compute_order_degree,
    compute_order_inertial,
};
use pyo3::prelude::*;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

#[pyfunction]
#[pyo3(name = "compute_order_degree")]
fn py_compute_order_degree(node_count: u32, tail: Vec<u32>, head: Vec<u32>) -> Vec<u32> {
    compute_order_degree(node_count, &tail, &head)
}

#[pyfunction]
#[pyo3(name = "compute_order_inertial")]
fn py_compute_order_inertial(
    node_count: u32,
    tail: Vec<u32>,
    head: Vec<u32>,
    latitude: Vec<f32>,
    longitude: Vec<f32>,
) -> Vec<u32> {
    compute_order_inertial(node_count, &tail, &head, &latitude, &longitude)
}

#[pyclass(frozen)]
#[pyo3(name = "CCH")]
struct PyCCH(Arc<CCH>);

#[pymethods]
impl PyCCH {
    #[new]
    fn new(order: Vec<u32>, tail: Vec<u32>, head: Vec<u32>, filter_always_inf_arcs: bool) -> Self {
        Self(Arc::new(CCH::new(
            &order,
            &tail,
            &head,
            |_| {},
            filter_always_inf_arcs,
        )))
    }
}

#[pyclass]
#[pyo3(name = "CCHMetric")]
struct PyCCHMetric {
    inner: Arc<CCHMetric<'static>>, // should drop before cch
    _cch: Arc<CCH>,
}

#[pymethods]
impl PyCCHMetric {
    #[new]
    fn new(cch: &PyCCH, weights: Vec<u32>) -> Self {
        let arc_cch = cch.0.clone();
        let cch_static = unsafe { &*Arc::as_ptr(&arc_cch) };
        Self {
            inner: Arc::new(CCHMetric::new(cch_static, weights)),
            _cch: arc_cch,
        }
    }

    #[getter]
    fn weights(&self) -> Vec<u32> {
        self.inner.weights().to_vec()
    }
}

#[pyclass(unsendable)]
#[pyo3(name = "CCHMetricPartialUpdater")]
struct PyCCHMetricPartialUpdater {
    inner: CCHMetricPartialUpdater<'static>,
    _cch: Arc<CCH>,
}

#[pymethods]
impl PyCCHMetricPartialUpdater {
    #[new]
    fn new(cch: &PyCCH) -> Self {
        let arc_cch = cch.0.clone();
        let cch_static = unsafe { &*Arc::as_ptr(&arc_cch) };
        Self {
            inner: CCHMetricPartialUpdater::new(cch_static),
            _cch: arc_cch,
        }
    }

    fn apply(&mut self, metric: &mut PyCCHMetric, updates: HashMap<u32, u32>) {
        self.inner.apply(
            Arc::get_mut(&mut metric.inner)
                .expect("cannot update CCHMetric: multiple references exist"),
            &updates,
        );
    }
}

#[pyclass(unsendable)]
#[pyo3(name = "CCHQuery")]
struct PyCCHQuery {
    inner: Rc<CCHQuery<'static>>,
    _metric: Arc<CCHMetric<'static>>,
}

#[pymethods]
impl PyCCHQuery {
    #[new]
    fn new(metric: &PyCCHMetric) -> Self {
        let metric_static = unsafe { &*Arc::as_ptr(&metric.inner) };
        Self {
            inner: Rc::new(CCHQuery::new(metric_static)),
            _metric: metric.inner.clone(),
        }
    }

    fn run(&mut self, py: Python, source: u32, target: u32) -> PyCCHQueryResult {
        let mut_q_static = unsafe {
            assert!(
                Rc::weak_count(&self.inner) == 0 && Rc::strong_count(&self.inner) == 1,
                "cannot run CCHQuery: multiple references exist"
            );
            &mut *(Rc::as_ptr(&self.inner) as *mut CCHQuery<'static>)
        };
        let result = py.detach(|| {
            mut_q_static.add_source(source, 0);
            mut_q_static.add_target(target, 0);
            mut_q_static.run()
        });
        PyCCHQueryResult {
            inner: result,
            _query: self.inner.clone(),
        }
    }

    fn run_multi_st_with_dist(
        &mut self,
        py: Python,
        sources: Vec<(u32, u32)>,
        target: Vec<(u32, u32)>,
    ) -> PyCCHQueryResult {
        let mut_q_static = unsafe {
            assert!(
                Rc::weak_count(&self.inner) == 0 && Rc::strong_count(&self.inner) == 1,
                "cannot run CCHQuery: multiple references exist"
            );
            &mut *(Rc::as_ptr(&self.inner) as *mut CCHQuery<'static>)
        };
        let result = py.detach(|| {
            for (s, d) in sources {
                mut_q_static.add_source(s, d);
            }
            for (t, d) in target {
                mut_q_static.add_target(t, d);
            }
            mut_q_static.run()
        });
        PyCCHQueryResult {
            inner: result,
            _query: self.inner.clone(),
        }
    }
}

#[pyclass(unsendable)]
#[pyo3(name = "CCHQueryResult")]
struct PyCCHQueryResult {
    inner: CCHQueryResult<'static, 'static>,
    _query: Rc<CCHQuery<'static>>,
}

#[pymethods]
impl PyCCHQueryResult {
    #[getter]
    fn distance(&self) -> Option<u32> {
        self.inner.distance()
    }

    #[getter]
    fn node_path(&self) -> Vec<u32> {
        self.inner.node_path().to_vec()
    }

    #[getter]
    fn arc_path(&self) -> Vec<u32> {
        self.inner.arc_path().to_vec()
    }
}

#[pymodule]
mod routingkit_cch {
    #[pymodule_export]
    use super::PyCCH;
    #[pymodule_export]
    use super::PyCCHMetric;
    #[pymodule_export]
    use super::PyCCHMetricPartialUpdater;
    #[pymodule_export]
    use super::PyCCHQuery;
    #[pymodule_export]
    use super::PyCCHQueryResult;
    #[pymodule_export]
    use super::py_compute_order_degree;
    #[pymodule_export]
    use super::py_compute_order_inertial;
}
