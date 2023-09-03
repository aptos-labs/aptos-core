#!/bin/bash

set -e

tmp=`mktemp`

cat additional_dict.txt | sort | uniq >$tmp

diff additional_dict.txt $tmp -uN || :

cp $tmp additional_dict.txt
