#pragma once
#include <memory>
#include <vector>
#include <cstdint>
#include <algorithm>
#include "rust/cxx.h"

// RoutingKit headers
#include <routingkit/customizable_contraction_hierarchy.h>

// cxx requires declarations inside an extern block or namespace
namespace rk_wrap
{
    struct CCH
    {
        RoutingKit::CustomizableContractionHierarchy inner;
        explicit CCH(RoutingKit::CustomizableContractionHierarchy &&x) : inner(std::move(x)) {}
    };

    struct CCHMetric
    {
        // Keep an owned copy of the weight vector so that the raw pointer
        // stored inside RoutingKit::CustomizableContractionHierarchyMetric
        // (metric.input_weight) never dangles. RoutingKit only stores the
        // pointer; without owning storage here we previously passed a
        // temporary std::vector that got destroyed after construction.
        std::shared_ptr<std::vector<unsigned>> owned_weights; // may be null if user supplied external lifetime (future extension)
        RoutingKit::CustomizableContractionHierarchyMetric inner;
        CCHMetric(std::shared_ptr<std::vector<unsigned>> w,
                  RoutingKit::CustomizableContractionHierarchyMetric &&x)
            : owned_weights(std::move(w)), inner(std::move(x)) {}
    };

    struct CCHQuery
    {
        RoutingKit::CustomizableContractionHierarchyQuery inner;
        explicit CCHQuery(RoutingKit::CustomizableContractionHierarchyQuery &&x) : inner(std::move(x)) {}
    };

    std::unique_ptr<CCH> cch_new(rust::Slice<const uint32_t> order,
                                 rust::Slice<const uint32_t> tail,
                                 rust::Slice<const uint32_t> head,
                                 bool filter_always_inf_arcs);

    std::unique_ptr<CCHMetric> cch_metric_new(const CCH &cch, rust::Slice<const uint32_t> weight);
    void cch_metric_customize(CCHMetric &metric);

    std::unique_ptr<CCHQuery> cch_query_new(const CCHMetric &metric);
    void cch_query_reset(CCHQuery &query, const CCHMetric &metric);
    void cch_query_add_source(CCHQuery &query, uint32_t s, uint32_t dist);
    void cch_query_add_target(CCHQuery &query, uint32_t t, uint32_t dist);
    void cch_query_run(CCHQuery &query);
    // Note: RoutingKit's get_distance / get_node_path are not const, so we use const_cast for read-only access
    uint32_t cch_query_distance(const CCHQuery &query);
    rust::Vec<uint32_t> cch_query_node_path(const CCHQuery &query);
    rust::Vec<uint32_t> cch_query_arc_path(const CCHQuery &query);

    // Compute nested dissection order using inertial flow (needs latitude & longitude arrays)
    rust::Vec<uint32_t> cch_compute_order_inertial(
        uint32_t node_count,
        rust::Slice<const uint32_t> tail,
        rust::Slice<const uint32_t> head,
        rust::Slice<const float> latitude,
        rust::Slice<const float> longitude);

    // Compute a simple degree-based heuristic order when coordinates are unavailable.
    // Sort nodes by (degree, node_id) ascending.
    rust::Vec<uint32_t> cch_compute_order_degree(
        uint32_t node_count,
        rust::Slice<const uint32_t> tail,
        rust::Slice<const uint32_t> head);
}

// Expose for cxx bridge
using rk_wrap::CCH;
using rk_wrap::CCHMetric;
using rk_wrap::CCHQuery;

std::unique_ptr<CCH> cch_new(rust::Slice<const uint32_t> order,
                             rust::Slice<const uint32_t> tail,
                             rust::Slice<const uint32_t> head,
                             bool filter_always_inf_arcs);
std::unique_ptr<CCHMetric> cch_metric_new(const CCH &cch, rust::Slice<const uint32_t> weight);
void cch_metric_customize(CCHMetric &metric);
std::unique_ptr<CCHQuery> cch_query_new(const CCHMetric &metric);
void cch_query_reset(CCHQuery &query, const CCHMetric &metric);
void cch_query_add_source(CCHQuery &query, uint32_t s, uint32_t dist);
void cch_query_add_target(CCHQuery &query, uint32_t t, uint32_t dist);
void cch_query_run(CCHQuery &query);
uint32_t cch_query_distance(const CCHQuery &query);
rust::Vec<uint32_t> cch_query_node_path(const CCHQuery &query);
rust::Vec<uint32_t> cch_query_arc_path(const CCHQuery &query);
rust::Vec<uint32_t> cch_compute_order_inertial(
    uint32_t node_count,
    rust::Slice<const uint32_t> tail,
    rust::Slice<const uint32_t> head,
    rust::Slice<const float> latitude,
    rust::Slice<const float> longitude);
rust::Vec<uint32_t> cch_compute_order_degree(
    uint32_t node_count,
    rust::Slice<const uint32_t> tail,
    rust::Slice<const uint32_t> head);
