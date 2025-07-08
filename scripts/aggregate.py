import re
from collections import defaultdict

# Paste the copied column below (preserve spacing!)
raw_data = """
256 bytes: 790.52 ms
1024 bytes: 468.928 ms
4096 bytes: 715.727 ms
16384 bytes: 1771.148 ms
65536 bytes: 2724.849 ms
131072 bytes: 3091.129 ms
256 bytes: 794.324 ms
1024 bytes: 476.036 ms
4096 bytes: 719.597 ms
16384 bytes: 1884.464 ms
65536 bytes: 3061.592 ms
131072 bytes: 3513.936 ms
256 bytes: 796.113 ms
1024 bytes: 476.907 ms
4096 bytes: 788.172 ms
16384 bytes: 1766.387 ms
65536 bytes: 3000.713 ms
131072 bytes: 3473.532 ms
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
