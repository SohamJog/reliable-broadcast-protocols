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

for ((i=0; i<NUM_ITERATIONS; i++))
do
  echo "=== Run $((i+1)) ==="
  pkill -f "./target/release/node"

  ./scripts/test.sh testdata/hyb_"$NUM_NODES"/syncer Hi "$BYZANTINE" testdata/test_msgs.txt "$PROTOCOL" "$NUM_NODES"
  
  sleep $((NUM_NODES / 2))

  # ./scripts/check_logs.sh "$NUM_NODES"
  ./scripts/latencies.sh
done
