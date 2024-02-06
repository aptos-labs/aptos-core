#!/bin/bash

for ((x=$1; x<($1+$2); x++))
do
    zip_file="gs://txn-data/$3net-$x-$((x+1))m-new.zip"
    gsutil -q stat $zip_file
    status=$?
    if [[ $status == 1 ]]; then
      zip_file="gs://txn-data/$3net-$x-$((x+1))m.zip"
    fi

    gsutil cp $zip_file ./$3net-$x-$((x+1))m-new.zip
    unzip -q $3net-$x-$((x+1))m-new.zip

    folder_1="$3net-$x-$((x+1))m-new"
    folder_2="$3net-$x-$((x+1))m"
    folder_3="$3net-data-$x-$((x+1))m"


    if [ -d "$folder_1" ]; then
        dir="$folder_1"
    elif [ -d "$folder_2" ]; then
        dir="$folder_2"
    elif [ -d "$folder_3"  ]; then
        dir="$folder_3"
    else
        echo "Error: Folder not found for iteration $x."
        continue  # Skip to the next iteration if folder not found
    fi

    target/release/aptos-comparison-testing \
    --begin-version $((x*1000000)) --limit 1000000 execute --execution-mode=v1 $dir
done