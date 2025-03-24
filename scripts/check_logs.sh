#!/bin/bash

# Usage: ./check_logs.sh <num_nodes>
if [ "$#" -ne 1 ]; then
    echo "Usage: $0 <num_nodes>"
    exit 1
fi

NUM_NODES=$1
LOG_DIR="logs"
MISSING_IDS=()
NUM_MESSAGES=3

echo "Checking logs for $NUM_NODES nodes..."

# Generate the list of expected instance IDs
EXPECTED_IDS=()
for ((i=0; i<NUM_NODES; i++)); do
    for ((j=1; j<=NUM_MESSAGES; j++)); do
        EXPECTED_IDS+=($((10000 * i + j)))
    done
done

# print expected ids
echo "Expected instance IDs: ${EXPECTED_IDS[*]}"

# Function to extract instance IDs from a log file
extract_instance_ids() {
  grep -Eo 'instance id [0-9]+' "$1" | awk '{print $3}' | sort -n | uniq
}

# Iterate through each log file
for ((node=0; node<NUM_NODES; node++)); do
    LOG_FILE="$LOG_DIR/$node.log"

    if [ ! -f "$LOG_FILE" ]; then
        echo "Log file missing: $LOG_FILE"
        continue
    fi

    # Extract logged instance IDs
    LOGGED_IDS=($(extract_instance_ids "$LOG_FILE"))

    # Compare expected IDs with logged IDs
    MISSING=()
    for ID in "${EXPECTED_IDS[@]}"; do
        if [[ ! " ${LOGGED_IDS[@]} " =~ " $ID " ]]; then
            MISSING+=($ID)
        fi
    done

    # If there are missing IDs, output the result
    if [ ${#MISSING[@]} -gt 0 ]; then
        echo "Missing instance IDs in $LOG_FILE: ${MISSING[*]}"
        MISSING_IDS+=("${MISSING[@]}")
    else
        echo "All expected instance IDs found in $LOG_FILE"
    fi
done

# Final report
if [ ${#MISSING_IDS[@]} -gt 0 ]; then
    echo -e "\n\033[31mSome instance IDs are missing across logs.\033[0m"
else
    echo -e "\n\033[32mAll logs contain the required instance IDs.\033[0m"
fi
