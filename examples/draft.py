import routingkit_cch as rk


def main():
    tail = [0, 1, 2]
    head = [1, 2, 3]
    weights = [10, 5, 7]
    node_count = 4

    order = rk.compute_order_degree(node_count, tail, head)
    cch = rk.CCH(order, tail, head, False)

    metric = rk.CCHMetric(cch, weights)

    q = rk.CCHQuery(metric)
    res = q.run_multi_st_with_dist([(0, 0)], [(3, 0)])
    return res


if __name__ == "__main__":
    res = main()
    print("Here")
    print(res.distance)
    print(res.node_path)
    print(res.arc_path)
    print("Done")
