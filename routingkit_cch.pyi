from typing import Self, Optional

class CCH:
    def __init__(
        self,
        order: list[int],
        tail: list[int],
        head: list[int],
        filter_always_inf_arcs: bool,
    ) -> Self: ...

class CCHMetric:
    def __init__(
        self,
        cch: CCH,
        weights: list[int],
    ) -> Self:
        self.weights: list[int]

class CCHMetricPartialUpdater:
    def __init__(self, cch: CCH) -> Self: ...
    def apply(self, metric: CCHMetric, updates: dict[int, int]) -> None: ...

class CCHQueryResult:
    distance: Optional[int]
    node_path: list[int]
    arc_path: list[int]

class CCHQuery:
    def __init__(self, metric: CCHMetric) -> Self: ...
    def run(self, source: int, target: int) -> CCHQueryResult: ...
    def run_multi_st_with_dist(
        self, sources: list[tuple[int, int]], targets: list[tuple[int, int]]
    ) -> CCHQueryResult: ...

def compute_order_degree(
    node_count: int, tail: list[int], head: list[int]
) -> list[int]: ...
def compute_order_inertial(
    node_count: int,
    tail: list[int],
    head: list[int],
    latitude: list[float],
    longitude: list[float],
) -> list[int]: ...
