#!/bin/bash

# A script to quickly test node startup and protocol execution

# Kill existing node processes
pkill -f "./target/release/node"                                       
killall node &> /dev/null
rm -rf /tmp/*.db &> /dev/null

# Default values
vals=(27000 27100 27200 27300)
TYPE=${TYPE:="release"}
protocol=${5:-rbc}
NUM_NODES=${6:-4}
TESTDIR=${TESTDIR:="testdata/hyb_$NUM_NODES"}
crash=${7:-true}

# Run the syncer
./target/$TYPE/node \
    --config "$TESTDIR/nodes-0.json" \
    --ip ip_file \
    --protocol sync \
    --input 100 \
    --syncer "$1" \
    --msg_size "$4" \
    --byzantine false \
    --crash false > logs/syncer.log &


# Run all the nodes
for ((i=0; i<NUM_NODES; i++)); do
    ./target/$TYPE/node \
        --config "$TESTDIR/nodes-$i.json" \
        --ip ip_file \
        --protocol "$protocol" \
        --input "$2" \
        --syncer "$1" \
        --msg_size "$4" \
        --byzantine "$3" \
        --crash "$crash" > logs/$i.log &
done

# Example usage:
# ./test.sh syncer_id 10 false bfile_path rbc 4
