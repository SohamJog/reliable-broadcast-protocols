#!/bin/bash

# Usage: ./multiple_runs.sh <num_iterations> [<num_nodes> <protocol> <byzantine>]
# Defaults:
#   <num_nodes> = 4
#   <protocol> = rbc
#   <byzantine> = false

if [ "$#" -lt 1 ]; then
    echo "Usage: $0 <num_iterations> [<num_nodes> <protocol> <byzantine>]"
    exit 1
fi

NUM_ITERATIONS=$1
NUM_NODES=${2:-4}
PROTOCOL=${3:-rbc}
BYZANTINE=${4:-false}
TESTDATA_FILE="testdata/longer_test_msgs.txt"
NUM_MESSAGES=$(wc -l < "$TESTDATA_FILE")
NUM_INSTANCES=$(( NUM_NODES * NUM_MESSAGES ))

for ((i=0; i<NUM_ITERATIONS; i++))
do
  echo "=== Run $((i+1)) ==="
  pkill -f "./target/release/node"

  ./scripts/test.sh testdata/hyb_"$NUM_NODES"/syncer Hi "$BYZANTINE" "$TESTDATA_FILE" "$PROTOCOL" "$NUM_NODES"
  
  # --- Wait for correct number of outputs ---
  EXPECTED_LINES=$(( NUM_INSTANCES + 2 ))
  if [ "$BYZANTINE" == "true" ]; then
    EXPECTED_LINES=$(( NUM_INSTANCES - NUM_MESSAGES * ( (NUM_NODES - 1) / 3 ) + 2 ))
  fi

  while true; do
    sleep 2
    ACTUAL_LINES=$(./scripts/latencies.sh | wc -l)
    if [ "$ACTUAL_LINES" -eq "$EXPECTED_LINES" ]; then
      ./scripts/latencies.sh
      break 
    # else 
    #   echo "Waiting for $EXPECTED_LINES lines, but got $ACTUAL_LINES. Retrying..."
    fi
  done

done

# TODO: test on multiple message sizes: 256 B → 4 KB → 64 KB → 1 MB
