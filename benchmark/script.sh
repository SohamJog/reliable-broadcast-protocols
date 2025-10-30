#!/bin/bash
n=${1}
msg=${2}
stat=${3}
mkdir logs/
mkdir logs/$n-$msg-$stat/

for i in {1..3}
do
	echo "Running iteration $i"
	fab rerun
	sleep 3m
	fab logs
	mv syncer.log logs/$n-$msg-$stat/syncer-$i.log
	fab kill
done
