import pandas as pd
import matplotlib.pyplot as plt
import seaborn as sns
import re
import os

# Load from file
with open("rbc_results.txt", "r") as f:
    raw = f.read()

# Updated regex to optionally match min and max
pattern = r"(?P<protocol>\w+)\s+(?P<nodes>\d+)\s+(?P<fault>no faults|byz|crash)[\s\S]+?Message Size \(bytes\)[\s\S]+?-+\n(?P<rows>(?:\d+\s+[\d.]+\s+(?:[\d.]+\s+){0,2}[\d.]+\n)+)"
matches = re.finditer(pattern, raw, re.IGNORECASE)

# Extract data
rows = []
for match in matches:
    protocol = match.group("protocol").upper()
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
        # Optional min and max
        if len(parts) >= 4:
            min_time = float(parts[2])
            max_time = float(parts[3])
        else:
            min_time = avg_time
            max_time = avg_time
        rows.append((protocol, fault_label, nodes, msg_size, avg_time, min_time, max_time))

# Create DataFrame
df = pd.DataFrame(rows, columns=["Protocol", "Fault", "Nodes", "Message Size", "Avg Time (ms)", "Min Time", "Max Time"])

# Ensure output directory exists
os.makedirs("rbc_subplots_new", exist_ok=True)

# Set style
sns.set(style="whitegrid")

# Marker styles per protocol
marker_styles = {
    "ADDRBC": "x",
    "CCRBC": "o",
    "RBC": "^",
    "CTRBC": "s",
    "default": "*"
}

# Generate and save plots
for (fault, nodes), sub_df in df.groupby(["Fault", "Nodes"]):
    plt.figure(figsize=(5, 4))
    for protocol, group in sub_df.groupby("Protocol"):
        marker = marker_styles.get(protocol, marker_styles["default"])
        x = group["Message Size"]
        y = group["Avg Time (ms)"]
        yerr = [
            y - group["Min Time"],
            group["Max Time"] - y
        ]
        plt.errorbar(x, y, yerr=yerr, marker=marker, label=protocol, capsize=4)

    plt.xscale("log")  # <-- log x-axis
    plt.xlabel("Message Size (bytes)")
    plt.ylabel("Avg Time (ms)")
    plt.title(f"{fault.replace('_', ' ')} | n = {nodes}")
    plt.legend(fontsize="small")
    plt.tight_layout()

    filename = f"rbc_subplots_new/{fault}_n{nodes}.png"
    plt.savefig(filename, dpi=300)
    plt.close()
    print(f"Saved {filename}")
