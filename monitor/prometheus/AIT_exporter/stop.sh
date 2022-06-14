#!/usr/bin/env bash
ps -ef | grep ait_exporter.py | grep -v grep | awk '{print $2}' | xargs kill
echo "Stopped"