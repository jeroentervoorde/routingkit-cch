from dataclasses import dataclass
import routingkit_cch as rk
import geopandas as gpd
from tqdm import tqdm
import more_itertools
from concurrent.futures import ThreadPoolExecutor
import os
import pickle


@dataclass
class Data:
    x: list[float]
    y: list[float]
    tail: list[int]
    head: list[int]
    weights: list[int]
    trips: list[list[int]]


def read_data(path: str):
    edge_df = gpd.read_file(os.path.join(path, "map/edges.shp"), ignore_geometry=True)
    node_df = gpd.read_file(os.path.join(path, "map/nodes.shp"), ignore_geometry=True)
    map_node_osmid_to_id = {
        j: i for i, j in enumerate(node_df[["osmid"]].to_numpy().flatten())
    }
    edge_df["u"] = edge_df["u"].map(map_node_osmid_to_id)
    edge_df["v"] = edge_df["v"].map(map_node_osmid_to_id)

    with open(os.path.join(path, "preprocessed_train_trips_all.pkl"), "rb") as f:
        data_train: list[tuple[tuple[str, int], list[int], tuple[int, int]]] = (
            pickle.load(f)
        )

    x: list[float] = node_df["x"].tolist()
    y: list[float] = node_df["y"].tolist()
    tail: list[int] = edge_df["u"].tolist()
    head: list[int] = edge_df["v"].tolist()
    weights = (edge_df["length"] * 1e3).astype(int).tolist()
    trips: list[list[int]] = [trip for _, trip, _ in data_train]
    return Data(x, y, tail, head, weights, trips)


def sequential(path: str = "data/beijing_data"):
    data = read_data(path)
    order = rk.compute_order_inertial(
        len(data.x),
        data.tail,
        data.head,
        data.x,
        data.y,
    )

    cch = rk.CCH(order, data.tail, data.head, False)
    metric = rk.CCHMetric(cch, data.weights)
    q = rk.CCHQuery(metric)

    pbar = tqdm(total=len(data.trips))
    for trip in data.trips:
        pbar.update(1)
        u = data.tail[trip[0]]
        v = data.head[trip[-1]]

        res = q.run(u, v)
        node_path = res.node_path
        assert node_path[0] == u and node_path[-1] == v
        assert sum(data.weights[i] for i in res.arc_path) == res.distance
        assert sum(data.weights[i] for i in trip) >= res.distance
        del res


def parallel(path: str = "data/beijing_data", max_workers: int = 2):
    data = read_data(path)
    order = rk.compute_order_inertial(
        len(data.x),
        data.tail,
        data.head,
        data.x,
        data.y,
    )

    cch = rk.CCH(order, data.tail, data.head, False)
    metric = rk.CCHMetric(cch, data.weights)

    executor = ThreadPoolExecutor(max_workers=max_workers)

    def worker(trips: list[list[int]]):
        q = rk.CCHQuery(metric)
        for trip in trips:
            u = data.tail[trip[0]]
            v = data.head[trip[-1]]

            res = q.run(u, v)
            node_path = res.node_path
            assert node_path[0] == u and node_path[-1] == v
            assert sum(data.weights[i] for i in res.arc_path) == res.distance
            assert sum(data.weights[i] for i in trip) >= res.distance
            del res

    futures = [
        executor.submit(worker, list(chunk))
        for chunk in more_itertools.chunked(
            data.trips, len(data.trips) // max_workers + 1
        )
    ]

    for future in futures:
        future.result()


if __name__ == "__main__":
    import time

    start = time.time()
    sequential()
    end = time.time()
    print(f"Sequential took {end - start} seconds")

    start = time.time()
    parallel()
    end = time.time()
    print(f"Parallel took {end - start} seconds")
