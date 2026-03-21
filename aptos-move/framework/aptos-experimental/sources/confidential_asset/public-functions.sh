#!/bin/bash

grep_for() {
    echo "Grepping for '$1' ..."
    grep -E --include="*.move" "$1" -Irn . | awk -F "$1" '{print $2}' | cut -f1 -d'('
    echo
}

echo

grep_for "public fun"
grep_for "public entry fun"
grep_for "[^c] entry fun" # poor man's attempt at excluding the 'public prefix'
