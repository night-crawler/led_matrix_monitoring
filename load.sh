#!/usr/bin/env bash

stress --cpu 8 --vm 1 --vm-bytes 30G --timeout 100 &
STRESS_PID=$!
sudo cpulimit --pid $STRESS_PID --limit 70
wait $STRESS_PID
