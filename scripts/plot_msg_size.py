import pandas as pd
import seaborn as sns
import matplotlib.pyplot as plt
import re

# Load from file
with open("rbc_results.txt", "r") as f:
    raw = f.read()

# Regex to extract blocks
pattern = r"(?P<protocol>\w+)\s+(?P<nodes>\d+)\s+(?P<fault>no faults|byz|crash)[\s\S]+?Message Size \(bytes\)[\s\S]+?-+\n(?P<rows>(?:\d+\s+[\d.]+\s+\d+\n)+)"
matches = re.finditer(pattern, raw, re.IGNORECASE)

# Extract data
rows = []
for match in matches:
    protocol = match.group("protocol").upper()
    nodes = int(match.group("nodes"))
    fault = match.group("fault").strip().lower()
    if fault == "no faults":
        fault_label = "No Faults"
    elif fault == "crash":
        fault_label = "Crash Faults"
    else:
        fault_label = "Byzantine Faults"
    
    for line in match.group("rows").strip().splitlines():
        parts = line.split()
        msg_size = int(parts[0])
        avg_time = float(parts[1])
        rows.append((protocol, fault_label, nodes, msg_size, avg_time))

# Create DataFrame
df = pd.DataFrame(rows, columns=["Protocol", "Fault", "Nodes", "Message Size", "Avg Time (ms)"])

# Plot and save to file
sns.set(style="whitegrid")
g = sns.relplot(
    data=df,
    x="Message Size",
    y="Avg Time (ms)",
    hue="Protocol",
    kind="line",
    col="Nodes",
    row="Fault",
    marker="o",
    facet_kws={'sharey': False, 'sharex': True}
)
g.set(xscale="log")
plt.tight_layout()
g.savefig("rbc_scaling_results.png", dpi=300)
print("Saved plot as rbc_scaling_results.png")
