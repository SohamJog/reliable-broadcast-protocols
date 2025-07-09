import re
from collections import defaultdict

# Paste the copied column below (preserve spacing!)
raw_data = """
256 bytes: 872.426 ms
1024 bytes: 494.619 ms
4096 bytes: 629.802 ms
16384 bytes: 777.556 ms
65536 bytes: 937.131 ms
131072 bytes: 1086.176 ms
256 bytes: 853.017 ms
1024 bytes: 470.83 ms
4096 bytes: 585.215 ms
16384 bytes: 710.225 ms
65536 bytes: 799.955 ms
131072 bytes: 865.34 ms
256 bytes: 871.738 ms
1024 bytes: 498.29 ms
4096 bytes: 603.819 ms
16384 bytes: 780.067 ms
65536 bytes: 962.205 ms
131072 bytes: 1071.868 ms
"""

size_to_times = defaultdict(list)

for line in raw_data.strip().splitlines():
    if line.startswith("256 bytes"):
        continue
    match = re.match(r"(\d+) bytes: ([\d.]+) ms", line)
    if match:
        size = int(match.group(1))
        time = float(match.group(2))
        size_to_times[size].append(time)

# Print average times per size
print("Message Size (bytes) | Avg Time (ms) | Count")
print("----------------------------------------------")
for size in sorted(size_to_times):
    times = size_to_times[size]
    avg = sum(times) / len(times)
    print(f"{size:<21} {avg:<14.3f} {len(times)}")
