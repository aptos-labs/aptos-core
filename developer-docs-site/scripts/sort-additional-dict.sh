#!/bin/bash

set -e

tmp=`mktemp`

cat additional_dict.txt | sort | uniq >$tmp

cp $tmp additional_dict.txt
