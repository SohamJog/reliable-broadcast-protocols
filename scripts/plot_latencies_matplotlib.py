import pandas as pd
import matplotlib.pyplot as plt
import seaborn as sns
import re
import os

# Load from file
with open("rbc_results.txt", "r") as f:
    raw = f.read()

# Regex to match rows with or without min/max
pattern = r"(?P<protocol>\w+)\s+(?P<nodes>\d+)\s+(?P<fault>no faults|byz|crash)[\s\S]+?Message Size \(bytes\)[\s\S]+?-+\n(?P<rows>(?:\d+\s+[\d.]+\s+(?:[\d.]+\s+){0,2}[\d.]+\n)+)"
matches = re.finditer(pattern, raw, re.IGNORECASE)

# Extract data
rows = []
for match in matches:
    protocol = match.group("protocol").upper()
    if protocol == "RBC":
        continue  # Skip RBC
    nodes = int(match.group("nodes"))
    fault = match.group("fault").strip().lower()
    if fault == "no faults":
        fault_label = "No_Faults"
    elif fault == "crash":
        fault_label = "Crash_Faults"
    else:
        fault_label = "Byzantine_Faults"

    for line in match.group("rows").strip().splitlines():
        parts = line.split()
        msg_size = int(parts[0])
        avg_time = float(parts[1])
        rows.append((protocol, fault_label, nodes, msg_size, avg_time))

# Create DataFrame
df = pd.DataFrame(rows, columns=["Protocol", "Fault", "Nodes", "Message Size", "Avg Time (ms)"])

# Ensure output directory exists
os.makedirs("rbc_subplots_no_rbc", exist_ok=True)

# Set style
sns.set(style="whitegrid")

# Marker styles per protocol
marker_styles = {
    "ADDRBC": "x",
    "CCRBC": "o",
    "CTRBC": "s",
    "default": "*"
}

# Generate and save plots
for (fault, nodes), sub_df in df.groupby(["Fault", "Nodes"]):
    plt.figure(figsize=(5, 4))
    for protocol, group in sub_df.groupby("Protocol"):
        marker = marker_styles.get(protocol, marker_styles["default"])
        plt.plot(
            group["Message Size"],
            group["Avg Time (ms)"],
            marker=marker,
            label=protocol
        )

    plt.xscale("log")  # log x-axis
    plt.xlabel("Message Size (bytes)")
    plt.ylabel("Avg Time (ms)")
    plt.title(f"{fault.replace('_', ' ')} | n = {nodes}")
    plt.legend(fontsize="small")
    plt.tight_layout()

    filename = f"rbc_subplots_no_rbc/{fault}_n{nodes}.png"
    plt.savefig(filename, dpi=300)
    plt.close()
    print(f"Saved {filename}")
