#include "routingkit_cch_wrapper.h"
#include "rust/cxx.h" // rust::Slice definition

#include <routingkit/customizable_contraction_hierarchy.h>
#include <routingkit/nested_dissection.h>
#include <routingkit/constants.h>
#include <stdexcept>
#include <functional>

using namespace RoutingKit;
using namespace rk_wrap;

std::unique_ptr<CCH> rk_wrap::cch_new(rust::Slice<const uint32_t> order,
                                      rust::Slice<const uint32_t> tail,
                                      rust::Slice<const uint32_t> head,
                                      bool filter_always_inf_arcs)
{
    if (tail.size() != head.size())
    {
        throw std::invalid_argument("tail/head size mismatch");
    }
    // copy from rust::Slice to std::vector
    auto to_vec = [](rust::Slice<const uint32_t> s)
    {
        std::vector<unsigned> v;
        v.reserve(s.size());
        for (size_t i = 0; i < s.size(); ++i)
            v.push_back(s[i]);
        return v;
    };
    CustomizableContractionHierarchy cch(
        to_vec(order),
        to_vec(tail),
        to_vec(head),
        [](const std::string &) {},
        filter_always_inf_arcs);
    return std::unique_ptr<CCH>(new CCH(std::move(cch)));
}

std::unique_ptr<CCHMetric> rk_wrap::cch_metric_new(const CCH &cch, rust::Slice<const uint32_t> weight)
{
    if (weight.size() != cch.inner.input_arc_count())
    {
        throw std::invalid_argument("weight size mismatch with input_arc_count");
    }
    // Zero-copy: directly use pointer into Rust slice.
    CustomizableContractionHierarchyMetric metric(cch.inner, reinterpret_cast<const unsigned *>(weight.data()));
    return std::unique_ptr<CCHMetric>(new CCHMetric(std::move(metric)));
}

void rk_wrap::cch_metric_customize(CCHMetric &metric)
{
    metric.inner.customize();
}

void rk_wrap::cch_metric_parallel_customize(CCHMetric &metric, uint32_t thread_count)
{
    RoutingKit::CustomizableContractionHierarchyParallelization par(*metric.inner.cch);
    if (thread_count == 0)
    {
        par.customize(metric.inner); // internal picks #procs (or 1 without OpenMP)
    }
    else
    {
        par.customize(metric.inner, thread_count);
    }
}

std::unique_ptr<CCHQuery> rk_wrap::cch_query_new(const CCHMetric &metric)
{
    CustomizableContractionHierarchyQuery q(metric.inner);
    return std::unique_ptr<CCHQuery>(new CCHQuery(std::move(q)));
}

void rk_wrap::cch_query_reset(CCHQuery &query, const CCHMetric &metric)
{
    query.inner.reset(metric.inner);
}

void rk_wrap::cch_query_add_source(CCHQuery &query, uint32_t s, uint32_t dist)
{
    query.inner.add_source(s, dist);
}

void rk_wrap::cch_query_add_target(CCHQuery &query, uint32_t t, uint32_t dist)
{
    query.inner.add_target(t, dist);
}

void rk_wrap::cch_query_run(CCHQuery &query)
{
    query.inner.run();
}

uint32_t rk_wrap::cch_query_distance(const CCHQuery &query)
{
    auto &mut_query = const_cast<RoutingKit::CustomizableContractionHierarchyQuery &>(query.inner);
    return mut_query.get_distance();
}

rust::Vec<uint32_t> rk_wrap::cch_query_node_path(const CCHQuery &query)
{
    auto &mut_query = const_cast<RoutingKit::CustomizableContractionHierarchyQuery &>(query.inner);
    auto path = mut_query.get_node_path();
    rust::Vec<uint32_t> out;
    out.reserve(path.size());
    for (auto x : path)
        out.push_back(static_cast<uint32_t>(x));
    return out;
}

rust::Vec<uint32_t> rk_wrap::cch_query_arc_path(const CCHQuery &query)
{
    auto &mut_query = const_cast<RoutingKit::CustomizableContractionHierarchyQuery &>(query.inner);
    auto path = mut_query.get_arc_path();
    rust::Vec<uint32_t> out;
    out.reserve(path.size());
    for (auto x : path)
        out.push_back(static_cast<uint32_t>(x));
    return out;
}

rust::Vec<uint32_t> rk_wrap::cch_compute_order_inertial(
    uint32_t node_count,
    rust::Slice<const uint32_t> tail,
    rust::Slice<const uint32_t> head,
    rust::Slice<const float> latitude,
    rust::Slice<const float> longitude)
{
    if (latitude.size() != node_count || longitude.size() != node_count)
    {
        throw std::invalid_argument("latitude/longitude size mismatch with node_count");
    }
    if (tail.size() != head.size())
    {
        throw std::invalid_argument("tail/head size mismatch");
    }
    auto to_uvec = [](rust::Slice<const uint32_t> s)
    {
        std::vector<unsigned> v;
        v.reserve(s.size());
        for (size_t i = 0; i < s.size(); ++i)
            v.push_back(s[i]);
        return v;
    };
    std::vector<float> lat;
    lat.reserve(latitude.size());
    std::vector<float> lon;
    lon.reserve(longitude.size());
    for (size_t i = 0; i < latitude.size(); ++i)
        lat.push_back(latitude[i]);
    for (size_t i = 0; i < longitude.size(); ++i)
        lon.push_back(longitude[i]);
    auto order = RoutingKit::compute_nested_node_dissection_order_using_inertial_flow(
        node_count,
        to_uvec(tail),
        to_uvec(head),
        lat,
        lon,
        [](const std::string &) {});
    rust::Vec<uint32_t> out;
    out.reserve(order.size());
    for (auto x : order)
        out.push_back(static_cast<uint32_t>(x));
    return out;
}

rust::Vec<uint32_t> rk_wrap::cch_compute_order_degree(
    uint32_t node_count,
    rust::Slice<const uint32_t> tail,
    rust::Slice<const uint32_t> head)
{
    if (tail.size() != head.size())
    {
        throw std::invalid_argument("tail/head size mismatch");
    }
    std::vector<uint32_t> deg(node_count, 0);
    for (size_t i = 0; i < tail.size(); ++i)
    {
        auto u = tail[i];
        auto v = head[i];
        if (u < node_count)
            deg[u]++;
        if (v < node_count)
            deg[v]++;
    }
    std::vector<uint32_t> nodes(node_count);
    for (uint32_t i = 0; i < node_count; ++i)
        nodes[i] = i;
    std::sort(nodes.begin(), nodes.end(), [&](uint32_t a, uint32_t b)
              {
        if(deg[a] != deg[b]) return deg[a] < deg[b];
        return a < b; });
    rust::Vec<uint32_t> out;
    out.reserve(nodes.size());
    for (auto x : nodes)
        out.push_back(x);
    return out;
}

// Expose C functions for cxx (forwarders)
std::unique_ptr<CCH> cch_new(rust::Slice<const uint32_t> order,
                             rust::Slice<const uint32_t> tail,
                             rust::Slice<const uint32_t> head,
                             bool filter_always_inf_arcs)
{
    return rk_wrap::cch_new(order, tail, head, filter_always_inf_arcs);
}
std::unique_ptr<CCHMetric> cch_metric_new(const CCH &cch, rust::Slice<const uint32_t> weight)
{
    return rk_wrap::cch_metric_new(cch, weight);
}
void cch_metric_customize(CCHMetric &metric) { rk_wrap::cch_metric_customize(metric); }
void cch_metric_parallel_customize(CCHMetric &metric, uint32_t thread_count) { rk_wrap::cch_metric_parallel_customize(metric, thread_count); }
std::unique_ptr<CCHQuery> cch_query_new(const CCHMetric &metric) { return rk_wrap::cch_query_new(metric); }
void cch_query_reset(CCHQuery &query, const CCHMetric &metric) { rk_wrap::cch_query_reset(query, metric); }
void cch_query_add_source(CCHQuery &query, uint32_t s, uint32_t dist) { rk_wrap::cch_query_add_source(query, s, dist); }
void cch_query_add_target(CCHQuery &query, uint32_t t, uint32_t dist) { rk_wrap::cch_query_add_target(query, t, dist); }
void cch_query_run(CCHQuery &query) { rk_wrap::cch_query_run(query); }
uint32_t cch_query_distance(const CCHQuery &query) { return rk_wrap::cch_query_distance(query); }
rust::Vec<uint32_t> cch_query_node_path(const CCHQuery &query) { return rk_wrap::cch_query_node_path(query); }
rust::Vec<uint32_t> cch_query_arc_path(const CCHQuery &query) { return rk_wrap::cch_query_arc_path(query); }
rust::Vec<uint32_t> cch_compute_order_inertial(
    uint32_t node_count,
    rust::Slice<const uint32_t> tail,
    rust::Slice<const uint32_t> head,
    rust::Slice<const float> latitude,
    rust::Slice<const float> longitude) { return rk_wrap::cch_compute_order_inertial(node_count, tail, head, latitude, longitude); }
rust::Vec<uint32_t> cch_compute_order_degree(
    uint32_t node_count,
    rust::Slice<const uint32_t> tail,
    rust::Slice<const uint32_t> head) { return rk_wrap::cch_compute_order_degree(node_count, tail, head); }
