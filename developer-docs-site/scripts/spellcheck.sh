#!/bin/sh

# Go to the parent directory.
cd "$(dirname "$0")"
cd ..

build_dict_path="/tmp/additional_dict.rws"
rewritten_file_path="/tmp/rewritten_file.md"

# Build additional word dictionary.
echo "Building additional word dictionary..."
aspell --lang=en create master $build_dict_path < scripts/additional_dict.txt
echo "Built additional word dictionary"
echo

everything_spelled_correctly=1

# Check the spelling of all md files, printing mispelled words if found.
for file in `find docs -type f -name "*.md"`
do
    # Rewrite the file to remove inline and multiline code blocks.
    # We also remove HTML tags.
    cat $file | sed '/```/,//d' | sed '/`/,//d' | sed 's/<[^>]*>/\n/g' > $rewritten_file_path
    mispelled_words=`aspell --lang=en --encoding=utf-8 list --add-extra-dicts=$build_dict_path < $rewritten_file_path`
    if [ ! -z "$mispelled_words" ]
    then
        echo "Mispelled words in $file:"
        echo "$mispelled_words"
        echo
        everything_spelled_correctly=0
    fi
done

# If any word was mispelled, exit with an error.
if [ $everything_spelled_correctly -eq 0 ]
then
    echo "Mispelled words were found 😭"
    echo "If the typo is not actually a typo, add the word to developer-docs-site/scripts/additional_dict.txt"
    echo
    exit 1
else
    echo "No mispelled words were found 🥳"
    echo
    exit 0
fi
