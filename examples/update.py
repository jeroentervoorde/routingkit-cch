import routingkit_cch as rk
from concurrent.futures import ThreadPoolExecutor
import time
import random


def main():
    tail = [0, 1, 2]
    head = [1, 2, 3]
    weights = [10, 5, 7]
    node_count = 4

    order = rk.compute_order_degree(node_count, tail, head)
    cch = rk.CCH(order, tail, head, False)

    metric = rk.CCHMetric(cch, weights)

    executor = ThreadPoolExecutor()

    # meaningless but concurrent updates
    def worker(i: int):
        updater = rk.CCHMetricPartialUpdater(cch)
        for j in range(1000):
            updater.apply(
                metric, {j % len(weights): i + metric.weights[j % len(weights)]}
            )
            time.sleep((0.5 + random.random()) * 0.01)

    futures = [executor.submit(worker, i) for i in range(16)]
    start = time.time()
    _ = [future.result() for future in futures]
    assert time.time() - start < 11.0
    assert metric.weights == [40090, 39965, 39967]


if __name__ == "__main__":
    main()
    print("OK")
