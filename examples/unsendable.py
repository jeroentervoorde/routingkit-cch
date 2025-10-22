import routingkit_cch as rk
from concurrent.futures import ThreadPoolExecutor


def main():
    tail = [0, 1, 2]
    head = [1, 2, 3]
    weights = [10, 5, 7]
    node_count = 4

    order = rk.compute_order_degree(node_count, tail, head)
    cch = rk.CCH(order, tail, head, False)

    metric = rk.CCHMetric(cch, weights)
    # updater = rk.CCHMetricPartialUpdater(cch)

    max_workers = 4
    executor = ThreadPoolExecutor(max_workers=max_workers)

    def worker(i: int):
        q = rk.CCHQuery(metric)
        res = q.run(0, 3)
        return res

    futures = [executor.submit(worker, i) for i in range(max_workers)]
    results = [future.result() for future in futures]

    print([i for i in results])


if __name__ == "__main__":
    # try:
    main()
    # except Exception as e:
    #     print("An error occurred:", e)

    print("Execution completed.")
