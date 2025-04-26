import os
import re
import numpy as np
import matplotlib.pyplot as plt

# Directory containing benchmark logs
LOG_DIR = "bench_msg_sizes"

# Message sizes corresponding to last digit of message ID
MESSAGE_SIZES = {
    1: 256,
    2: 1024,
    3: 4096,
    4: 16384,
    5: 65536,
    6: 131072,
}

# Data structure: {node_count: {protocol: {byz_true/false: {message_size: [latencies]}}}}
aggregated_data = {}

# Parse each log file
for filename in os.listdir(LOG_DIR):
    if not filename.endswith(".log"):
        continue

    match = re.match(r"(\w+)_(\d+)_(true|false)_.*\.log", filename)
    if not match:
        print(f"Skipping file {filename}: does not match expected pattern.")
        continue

    protocol, node_count, byz = match.groups()
    node_count = int(node_count)
    config_key = (protocol, byz)

    if node_count not in aggregated_data:
        aggregated_data[node_count] = {}
    if config_key not in aggregated_data[node_count]:
        aggregated_data[node_count][config_key] = {}

    with open(os.path.join(LOG_DIR, filename), "r") as f:
        for line in f:
            m = re.match(r"ID\s+(\d+)\s+\|\s+\d+\s+bytes\s+\|\s+([\d.]+)", line)
            if m:
                msg_id = int(m.group(1))
                latency = float(m.group(2))
                unit_digit = msg_id % 10
                if unit_digit == 0:
                    unit_digit = 10

                msg_size = MESSAGE_SIZES.get(unit_digit)
                if msg_size is None:
                    continue

                if msg_size not in aggregated_data[node_count][config_key]:
                    aggregated_data[node_count][config_key][msg_size] = []

                aggregated_data[node_count][config_key][msg_size].append(latency)

# Only for node counts 16 and 40
for node_count in [16, 40]:
    if node_count not in aggregated_data:
        continue

    protocols = set(p for (p, b) in aggregated_data[node_count].keys())

    for protocol in protocols:
        plt.figure(figsize=(10, 6))

        for byz in ["false"]:
            key = (protocol, byz)
            if key not in aggregated_data[node_count]:
                continue

            msg_sizes = sorted(aggregated_data[node_count][key].keys())
            means = [np.mean(aggregated_data[node_count][key][size]) for size in msg_sizes]
            stds = [np.std(aggregated_data[node_count][key][size]) for size in msg_sizes]

            plt.errorbar(
                msg_sizes,
                means,
                yerr=stds,
                label=f"Byzantine = {byz}",
                capsize=5,
                marker="o",
                linestyle="--" if byz == "true" else "-",
            )

        plt.xscale("log")
        plt.title(f"Latency vs Message Size ({protocol.upper()} @ {node_count} Nodes)")
        plt.xlabel("Message Size (bytes)")
        plt.ylabel("Average Latency (ms)")
        plt.grid(True)
        plt.legend()
        plt.tight_layout()
        plt.savefig(f"latency_vs_msgsize_{protocol}_{node_count}nodes.png")
        plt.close()
