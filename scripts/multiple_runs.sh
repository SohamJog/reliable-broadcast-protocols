# Write the following bash script: For <input> times, 
# run: pkill -f "./target/release/node"  
# run: ./scripts/test.sh testdata/hyb_4/syncer Hi false testdata/test_msgs.txt addrbc
# wait 1 second
# run: ./scripts/check_logs.sh 4
# Usage: ./multiple_runs.sh <num_nodes>
if [ "$#" -ne 1 ]; then
    echo "Usage: $0 <num_iterations>"
    exit 1
fi

for ((i=0; i<$1; i++))
do
  pkill -f "./target/release/node"
  ./scripts/test.sh testdata/hyb_4/syncer Hi false testdata/test_msgs.txt addrbc
  sleep 1
  ./scripts/check_logs.sh 4
done


