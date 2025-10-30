if [ "$#" -ne 1 ]; then
  echo "Usage: $0 <num_nodes>"
  exit 1
fi

num_nodes=$1

mkdir -p testdata/hyb_${num_nodes}
./target/release/genconfig \
  --NumNodes $num_nodes \
  --delay 10 \
  --blocksize 100 \
  --client_base_port 19000 \
  --target testdata/hyb_${num_nodes}/ \
  --payload 100 \
  --out_type json \
  --base_port 15000 \
  --client_run_port 19500 \
  --local true
