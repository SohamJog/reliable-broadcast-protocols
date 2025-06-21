import re
from collections import defaultdict

# Paste the copied column below (preserve spacing!)
raw_data = """
256 bytes: 892.909 ms
1024 bytes: 1444.369 ms
4096 bytes: 3241.387 ms
16384 bytes: 4528.005 ms
65536 bytes: 5229.658 ms
131072 bytes: 5634.484 ms
256 bytes: 890.835 ms
1024 bytes: 1627.212 ms
4096 bytes: 3615.706 ms
16384 bytes: 4783.145 ms
65536 bytes: 5454.407 ms
131072 bytes: 5936.837 ms
256 bytes: 899.609 ms
1024 bytes: 1501.741 ms
4096 bytes: 3312.014 ms
16384 bytes: 4637.007 ms
65536 bytes: 5345.117 ms
131072 bytes: 5761.823 ms
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
