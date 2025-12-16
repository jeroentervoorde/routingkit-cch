#pragma once
#include <memory>
#include <vector>
#include <cstdint>
#include <algorithm>
#include "rust/cxx.h"

// RoutingKit headers
#include <routingkit/customizable_contraction_hierarchy.h>
#include <routingkit/contraction_hierarchy.h>

struct CCH
{
    RoutingKit::CustomizableContractionHierarchy inner;
    explicit CCH(RoutingKit::CustomizableContractionHierarchy &&x) : inner(std::move(x)) {}
};

struct CH
{
    RoutingKit::ContractionHierarchy inner;
    explicit CH(RoutingKit::ContractionHierarchy &&x) : inner(std::move(x)) {}
};

struct CHQuery
{
    RoutingKit::ContractionHierarchyQuery inner;
    explicit CHQuery(RoutingKit::ContractionHierarchyQuery &&x) : inner(std::move(x)) {}
};

struct CCHMetric
{
    RoutingKit::CustomizableContractionHierarchyMetric inner;
    explicit CCHMetric(RoutingKit::CustomizableContractionHierarchyMetric &&x) : inner(std::move(x)) {}
};

struct CCHQuery
{
    RoutingKit::CustomizableContractionHierarchyQuery inner;
    explicit CCHQuery(RoutingKit::CustomizableContractionHierarchyQuery &&x) : inner(std::move(x)) {}
};

struct CCHPartial
{
    RoutingKit::CustomizableContractionHierarchyPartialCustomization inner;
    explicit CCHPartial(const RoutingKit::CustomizableContractionHierarchy &cch) : inner(cch) {}
};

std::unique_ptr<CCH> cch_new(rust::Slice<const uint32_t> order,
                             rust::Slice<const uint32_t> tail,
                             rust::Slice<const uint32_t> head,
                             rust::Fn<void(rust::Str)> log_message,
                             bool filter_always_inf_arcs);
std::unique_ptr<CCHMetric> cch_metric_new(const CCH &cch, rust::Slice<const uint32_t> weight);
void cch_metric_customize(CCHMetric &metric);
void cch_metric_parallel_customize(CCHMetric &metric, uint32_t thread_count);
std::unique_ptr<CH> cch_metric_build_perfect_ch(CCHMetric &metric);
std::unique_ptr<CCHQuery> cch_query_new(const CCHMetric &metric);
void cch_query_reset(CCHQuery &query, const CCHMetric &metric);
void cch_query_add_source(CCHQuery &query, uint32_t s, uint32_t dist);
void cch_query_add_target(CCHQuery &query, uint32_t t, uint32_t dist);
void cch_query_run(CCHQuery &query);
void cch_query_run_to_pinned_targets(CCHQuery &query);
void cch_query_pin_targets(CCHQuery &query, rust::Slice<const uint32_t> targets);
rust::Vec<uint32_t> cch_query_get_distances_to_targets(const CCHQuery &query);
void cch_query_get_distances_to_targets_no_alloc(const CCHQuery &query, rust::Slice<uint32_t> dists);

void cch_query_run_to_pinned_sources(CCHQuery &query);
void cch_query_pin_sources(CCHQuery &query, rust::Slice<const uint32_t> sources);
rust::Vec<uint32_t> cch_query_get_distances_to_sources(const CCHQuery &query);
void cch_query_get_distances_to_sources_no_alloc(const CCHQuery &query, rust::Slice<uint32_t> dists);

void cch_query_reset_source(CCHQuery &query);
void cch_query_reset_target(CCHQuery &query);

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

// Partial customization API

std::unique_ptr<CCHPartial> cch_partial_new(const CCH &cch);
void cch_partial_reset(CCHPartial &partial);
void cch_partial_update_arc(CCHPartial &partial, uint32_t arc);
void cch_partial_customize(CCHPartial &partial, CCHMetric &metric);

std::unique_ptr<CH> ch_build(
    uint32_t node_count,
    rust::Slice<const uint32_t> tail,
    rust::Slice<const uint32_t> head,
    rust::Slice<const uint32_t> weight,
    rust::Fn<void(rust::Str)> log_message,
    uint32_t max_pop_count);

std::unique_ptr<CH> ch_load_file(rust::Str file_name);
void ch_save_file(const CH &ch, rust::Str file_name);

std::unique_ptr<CHQuery> ch_query_new(const CH &ch);
void ch_query_reset_ch(CHQuery &query, const CH &ch);
void ch_query_reset(CHQuery &query);
void ch_query_add_source(CHQuery &query, uint32_t s, uint32_t dist);
void ch_query_add_target(CHQuery &query, uint32_t t, uint32_t dist);
void ch_query_run(CHQuery &query);
void ch_query_pin_targets(CHQuery &query, rust::Slice<const uint32_t> targets);
void ch_query_run_to_pinned_targets(CHQuery &query);
rust::Vec<uint32_t> ch_query_get_distances_to_targets(const CHQuery &query);
void ch_query_get_distances_to_targets_no_alloc(const CHQuery &query, rust::Slice<uint32_t> dists);

void ch_query_pin_sources(CHQuery &query, rust::Slice<const uint32_t> sources);
void ch_query_run_to_pinned_sources(CHQuery &query);
rust::Vec<uint32_t> ch_query_get_distances_to_sources(const CHQuery &query);
void ch_query_get_distances_to_sources_no_alloc(const CHQuery &query, rust::Slice<uint32_t> dists);

uint32_t ch_query_distance(const CHQuery &query);
rust::Vec<uint32_t> ch_query_node_path(const CHQuery &query);
rust::Vec<uint32_t> ch_query_arc_path(const CHQuery &query);
void ch_query_reset_source(CHQuery &query);
void ch_query_reset_target(CHQuery &query);
