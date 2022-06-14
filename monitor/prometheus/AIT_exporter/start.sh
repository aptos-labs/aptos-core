#!/usr/bin/env bash
nohup ./ait_exporter.py > stdout.log 2>&1 &
echo "Started"