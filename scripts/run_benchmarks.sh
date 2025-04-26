#!/bin/bash

# Usage: ./run_benchmarks.sh
# Ensure: ./scripts/multiple_runs.sh <num_iterations> <num_nodes> <protocol> <byzantine>

NUM_ITERATIONS=4
# NODE_COUNTS=(4 )
NODE_COUNTS=(16 40)
# PROTOCOLS=( rbc)
PROTOCOLS=(rbc addrbc ctrbc)
BYZ_OPTIONS=(false true)

LOG_DIR="bench_msg_sizes"
mkdir -p $LOG_DIR

TIMESTAMP=$(date +%Y%m%d_%H%M%S)

echo "Starting benchmark suite..."

for NODES in "${NODE_COUNTS[@]}"; do
  for PROTOCOL in "${PROTOCOLS[@]}"; do
    for BYZ in "${BYZ_OPTIONS[@]}"; do
      echo "Running: Nodes=$NODES Protocol=$PROTOCOL Byzantine=$BYZ Iterations=$NUM_ITERATIONS"
      LOG_FILE="$LOG_DIR/${PROTOCOL}_${NODES}_${BYZ}_$TIMESTAMP.log"
      ./scripts/multiple_runs.sh $NUM_ITERATIONS $NODES $PROTOCOL $BYZ | tee "$LOG_FILE"
    done
  done
done

echo "Benchmark suite complete. Logs saved to $LOG_DIR."

# Optional: Add a post-processing hook
# ./scripts/aggregate_benchmarks.sh $LOG_DIR
