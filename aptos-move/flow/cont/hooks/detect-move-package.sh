#!/usr/bin/env bash
# UserPromptSubmit hook: detect current Move package and provide context.
TOOL_STATUS='{{ tool(name="move_package_status") }}'
TOOL_MANIFEST='{{ tool(name="move_package_manifest") }}'

dir=$(pwd)
while [[ "$dir" != "/" ]]; do
    if [[ -f "$dir/Move.toml" ]]; then
        pkg_name=$(python3 -c "
for line in open('$dir/Move.toml'):
    s = line.strip()
    if s.startswith('name'):
        print(s.split('=',1)[1].strip().strip('\"').strip(\"'\"))
        break
" 2>/dev/null)
        python3 -c "
import json
ctx = 'Current Move package: ${pkg_name:-(unknown)} at $dir. '
ctx += 'Use $TOOL_STATUS to check for errors, '
ctx += '$TOOL_MANIFEST to explore the package.'
print(json.dumps({'additionalContext': ctx}))
"
        exit 0
    fi
    dir=$(dirname "$dir")
done
