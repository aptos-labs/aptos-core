#!/bin/bash
set -e

echo "🚀 Starting full replacement: velor → velor"

# 1. Directory renaming (velor-* → velor-*)
for dir in $(find . -type d -name "velor-*"); do
  new=$(echo $dir | sed 's/velor-/velor-/g')
  echo "Renaming directory: $dir → $new"
  mv "$dir" "$new"
done

# 2. File renaming (velor-* → velor-*)
for file in $(find . -type f -name "*velor-*"); do
  new=$(echo $file | sed 's/velor-/velor-/g')
  echo "Renaming file: $file → $new"
  mv "$file" "$new"
done

# 3. Replace in all text/code files
echo "🔄 Replacing inside files..."
grep -rl "velor" . | while read -r f; do
  echo "Updating $f"
  sed -i 's@velor@velor@g' "$f"
done

echo "✅ Replacement completed!"
