import re
from collections import defaultdict

# Paste the copied column below (preserve spacing!)
raw_data = """
256 bytes: 2483.1 ms
1024 bytes: 8736.308 ms
4096 bytes: 9880.11 ms
16384 bytes: 12524.826 ms
65536 bytes: 13580.727 ms
131072 bytes: 15046.066 ms
256 bytes: 2379.273 ms
1024 bytes: 7185.849 ms
4096 bytes: 8340.573 ms
16384 bytes: 10241.308 ms
65536 bytes: 13271.481 ms
131072 bytes: 14666.153 ms
256 bytes: 2437.711 ms
1024 bytes: 8628.428 ms
4096 bytes: 10349.713 ms
16384 bytes: 12398.972 ms
65536 bytes: 15826.14 ms
131072 bytes: 17898.85 ms
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
