#!/bin/bash
set -e

# --- Step 1: Crates / Directories Rename ---
for dir in $(find . -type d -name "velor-*"); do
  new=$(echo $dir | sed 's/velor-/velor-/g')
  mv "$dir" "$new"
done

# --- Step 2: All Cargo.toml replacements ---
grep -rl "velor-" --include="Cargo.toml" . | xargs sed -i 's@velor-@velor-@g' || true
grep -rl "velor_" --include="Cargo.toml" . | xargs sed -i 's@velor_@velor_@g' || true

# --- Step 3: All Rust source replacements ---
grep -rl "velor-" --include="*.rs" . | xargs sed -i 's@velor-@velor-@g' || true
grep -rl "velor_" --include="*.rs" . | xargs sed -i 's@velor_@velor_@g' || true

# --- Step 4: Other text files (md, sh, yml, toml etc) ---
grep -rl "velor-" --include="*.md" --include="*.yml" --include="*.yaml" --include="*.sh" --include="*.toml" . | xargs sed -i 's@velor-@velor-@g' || true
grep -rl "velor_" --include="*.md" --include="*.yml" --include="*.yaml" --include="*.sh" --include="*.toml" . | xargs sed -i 's@velor_@velor_@g' || true

# --- Step 5: Special functions like get_velor_* ---
grep -rl "get_velor_" --include="*.rs" . | xargs sed -i 's@get_velor_@get_velor_@g' || true

echo "✅ All velor → velor replacements done!"
