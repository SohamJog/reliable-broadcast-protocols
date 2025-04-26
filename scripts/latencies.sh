#!/bin/bash

LOG_FILE="logs/syncer.log"
TESTDATA_FILE="testdata/longer_test_msgs.txt"
if [ ! -f "$LOG_FILE" ]; then
    echo "Log file $LOG_FILE not found!"
    exit 1
fi
if [ ! -f "$TESTDATA_FILE" ]; then
    echo "Test data file $TESTDATA_FILE not found!"
    exit 1
fi

# make an array of the sizes of the messages of each line in the testdata file
declare -a msg_sizes
line_number=1
while IFS= read -r line; do
    line=$(echo "$line" | xargs)
    if [ -z "$line" ]; then
        continue
    fi
    byte_size=$(echo -n "$line" | wc -c)
    msg_sizes+=("$byte_size")
    line_number=$((line_number+1))
done < "$TESTDATA_FILE"



echo "Message ID | Bytes | Avg Latency | Num Latencies"
echo "-----------------------------------------------"

sizes="${msg_sizes[*]}"

awk -v sizes="$sizes" '
BEGIN {
    split(sizes, szarr, " ")
}

/All n nodes completed the protocol for ID:/ {
    if (match($0, /ID: [0-9]+/)) {
        id = substr($0, RSTART + 4, RLENGTH - 4)
    }

    if (match($0, /latency \[[^]]+\]/)) {
        lat_raw = substr($0, RSTART + 9, RLENGTH - 10)
        n = split(lat_raw, lat_arr, /, */)
        for (i = 1; i <= n; i++) {
            sum_lat[id] += lat_arr[i]
            count_lat[id]++
        }
    }

    # Correct way to map ID → message size
    {
        unit_digit = id % 10
        if (unit_digit == 0) unit_digit = 10
        byte_len[id] = szarr[unit_digit]
    }
}

END {
    for (id in sum_lat) {
        avg = sum_lat[id] / count_lat[id]
        printf "ID %-8s | %-5d bytes | %-11.2f | %d latencies\n", id, byte_len[id], avg, count_lat[id]
    }
}
' "$LOG_FILE" | sort -n -k2


# awk '
# /All n nodes completed the protocol for ID:/ {
#     # Extract ID
#     if (match($0, /ID: [0-9]+/)) {
#         id = substr($0, RSTART + 4, RLENGTH - 4)
#     }

#     # Extract latencies
#     if (match($0, /latency \[[^]]+\]/)) {
#         lat_raw = substr($0, RSTART + 9, RLENGTH - 10)
#         n = split(lat_raw, lat_arr, /, */)
#         for (i = 1; i <= n; i++) {
#             sum_lat[id] += lat_arr[i]
#             count_lat[id]++
#         }
#     }

#     # # Extract message value and compute byte size
#     # if (match($0, /value {".*"}/)) {
#     #     msg = substr($0, RSTART + 7, RLENGTH - 8)
#     #     gsub(/\\n/, "", msg)
#     #     byte_len[id] = length(msg)
#     # }
#     # Compute message byte size based on ID
#     {
#         unit_digit = id % 10
#         if (unit_digit == 0) unit_digit = 10
#         byte_len[id] = ENVIRON["msg_size_" unit_digit]
#     }

# }

# END {
#     for (id in sum_lat) {
#         avg = sum_lat[id] / count_lat[id]
#         printf "ID %-8s | %-5d bytes | %-11.2f | %d latencies\n", id, byte_len[id], avg, count_lat[id]
#     }
# }
# ' "$LOG_FILE" | sort -n -k2