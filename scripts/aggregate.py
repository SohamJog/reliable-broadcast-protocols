import re
from collections import defaultdict

# Paste the copied column below (preserve spacing!)
raw_data = """
256 bytes: 856.438 ms
1024 bytes: 542.889 ms
4096 bytes: 960.763 ms
16384 bytes: 1518.478 ms
65536 bytes: 1827.8 ms
131072 bytes: 2113.108 ms
256 bytes: 853.556 ms
1024 bytes: 533.215 ms
4096 bytes: 849.68 ms
16384 bytes: 1430.112 ms
65536 bytes: 1750.863 ms
131072 bytes: 2175.997 ms
256 bytes: 862.619 ms
1024 bytes: 525.657 ms
4096 bytes: 721.029 ms
16384 bytes: 1085.259 ms
65536 bytes: 1408.188 ms
131072 bytes: 1797.04 ms
256 bytes: 771.087 ms
1024 bytes: 398.585 ms
4096 bytes: 490.996 ms
16384 bytes: 843.276 ms
65536 bytes: 1342.87 ms
131072 bytes: 1731.784 ms
256 bytes: 755.678 ms
1024 bytes: 399.745 ms
4096 bytes: 487.581 ms
16384 bytes: 870.555 ms
65536 bytes: 1386.501 ms
131072 bytes: 1750.618 ms
256 bytes: 778.469 ms
1024 bytes: 412.295 ms
4096 bytes: 510.317 ms
16384 bytes: 900.519 ms
65536 bytes: 1441.571 ms
131072 bytes: 1798.131 ms
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

protocols = ["ADDRBC", "CTRBC", "CCRBC"]

# Split and validate line count
lines = [line.strip() for line in raw_data.strip().splitlines() if line.strip()]
if len(lines) != 54:
    raise ValueError(f"Expected 54 lines (18 per protocol), got {len(lines)}")

# Split lines into 3 chunks for each protocol
raw_chunks = [lines[i * 18 : (i + 1) * 18] for i in range(3)]


# Open file to append
with open("rbc_results.txt", "a") as out:
    for protocol, chunk in zip(protocols, raw_chunks):
        size_to_times = defaultdict(list)
        # for line in chunk.strip().splitlines():
        for line in chunk:
            if line.startswith("256 bytes"):
                continue
            match = re.match(r"(\d+) bytes: ([\d.]+) ms", line)
            if match:
                size = int(match.group(1))
                time = float(match.group(2))
                size_to_times[size].append(time)

        out.write(f"\n{protocol} 40 crash faults\n")
        out.write("Message Size (bytes) | Avg Time (ms) | Min | Max\n")
        out.write("-----------------------------------------------\n")
        for size in sorted(size_to_times):
            times = size_to_times[size]
            avg = sum(times) / len(times)
            min_time = min(times)
            max_time = max(times)
            out.write(f"{size:<21} {avg:<14.3f} {min_time:<5.1f} {max_time:<5.1f}\n")

print("Appended data with average, min, and max per size (except 256 bytes).")
