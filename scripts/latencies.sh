#!/bin/bash

LOG_FILE="logs/syncer.log"

echo "Message ID | Bytes | Avg Latency | Num Latencies"
echo "-----------------------------------------------"

awk '
/All n nodes completed the protocol for ID:/ {
    # Extract ID
    if (match($0, /ID: [0-9]+/)) {
        id = substr($0, RSTART + 4, RLENGTH - 4)
    }

    # Extract latencies
    if (match($0, /latency \[[^]]+\]/)) {
        lat_raw = substr($0, RSTART + 9, RLENGTH - 10)
        n = split(lat_raw, lat_arr, /, */)
        for (i = 1; i <= n; i++) {
            sum_lat[id] += lat_arr[i]
            count_lat[id]++
        }
    }

    # Extract message value and compute byte size
    if (match($0, /value {".*"}/)) {
        msg = substr($0, RSTART + 7, RLENGTH - 8)
        gsub(/\\n/, "", msg)
        byte_len[id] = length(msg)
    }
}

END {
    for (id in sum_lat) {
        avg = sum_lat[id] / count_lat[id]
        printf "ID %-8s | %-5d bytes | %-11.2f | %d latencies\n", id, byte_len[id], avg, count_lat[id]
    }
}
' "$LOG_FILE" | sort -n -k2
