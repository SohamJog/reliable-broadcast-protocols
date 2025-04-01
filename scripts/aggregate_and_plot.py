import seaborn as sns
import numpy as np


import os
import re
import statistics
from collections import defaultdict
import matplotlib.pyplot as plt

# Directory containing benchmark logs
LOG_DIR = "bench_logs"

# Data structure: {node_count: {protocol_byz: {msg_id: [latencies]}}}
aggregated_data = defaultdict(lambda: defaultdict(lambda: defaultdict(list)))

# Parse each log file
for filename in os.listdir(LOG_DIR):
    if not filename.endswith(".log"):
        continue

    match = re.match(r"(\w+)_(\d+)_(true|false)_.*\.log", filename)
    if not match:
        print(f"Skipping file {filename}: does not match expected pattern.")
        continue

    protocol, node_count, byz = match.groups()
    config_key = f"{protocol}_{byz}"
    node_count = int(node_count)

    with open(os.path.join(LOG_DIR, filename), "r") as f:
        for line in f:
            m = re.match(r"ID\s+(\d+)\s+\|\s+\d+\s+bytes\s+\|\s+([\d.]+)", line)

            if m:
                msg_id = int(m.group(1))
                latency = float(m.group(2))
                aggregated_data[node_count][config_key][msg_id].append(latency)

for node_count in sorted(aggregated_data.keys()):
    for config_key, msg_dict in aggregated_data[node_count].items():
        # Flatten all latencies for this config
        all_latencies = []
        for msg_id, lat_list in msg_dict.items():
            all_latencies.extend(lat_list)

        if not all_latencies:
            continue

        data = np.array(all_latencies)
        mean = data.mean()
        median = np.median(data)
        std = data.std()

        # --- Histogram with normal curve ---
        plt.figure(figsize=(12, 6))
        sns.histplot(data, kde=True, bins=20, color="skyblue", stat="density", label="Latency Dist")

        plt.axvline(mean, color='yellow', linestyle='--', label=f"Mean: {mean:.2f}")
        plt.axvline(median, color='green', linestyle='-.', label=f"Median: {median:.2f}")
        plt.axvline(mean - std, color='blue', linestyle=':', label=f"±1 STD: {mean-std:.2f}")
        plt.axvline(mean + std, color='blue', linestyle=':')

        plt.title(f"Latency Distribution\n{config_key} @ {node_count} Nodes")
        plt.xlabel("Latency (ms)")
        plt.ylabel("Density")
        plt.legend()
        plt.grid(True)
        plt.tight_layout()
        plt.savefig(f"latency_hist_{config_key}_{node_count}.png")
        plt.close()

    # --- Box plot comparing configs for this node count ---
    plt.figure(figsize=(12, 6))
    data_for_box = []
    labels = []

    for config_key, msg_dict in aggregated_data[node_count].items():
        all_latencies = []
        for msg_id, lat_list in msg_dict.items():
            all_latencies.extend(lat_list)
        if all_latencies:
            data_for_box.append(all_latencies)
            labels.append(config_key)

    if data_for_box:
        plt.boxplot(data_for_box, labels=labels, patch_artist=True)
        plt.title(f"Latency Box Plot @ {node_count} Nodes")
        plt.ylabel("Latency (ms)")
        plt.grid(True)
        plt.tight_layout()
        plt.savefig(f"latency_boxplot_{node_count}.png")
        plt.close()
