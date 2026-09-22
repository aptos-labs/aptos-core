#!/usr/bin/env bash
# Audit which simp lemmas actually fire across the test corpus.
#
# Lean's `diagnostics` option reports, per `simp` call, every theorem it *tried*
# and how often it *succeeded*; a lemma tried but never applied is flagged ❌️.
# A lemma with many tries and no successes is pure cost: its discrimination-tree
# key matches terms it cannot rewrite, or a higher-priority lemma always wins.
#
# Usage:  scripts/simp-audit.sh [file ...]     (default: Move/Tests/**/*.lean)
set -uo pipefail
cd "$(dirname "$0")/../move"
FILES=("$@")
if [ ${#FILES[@]} -eq 0 ]; then
  mapfile -t FILES < <(find Move/Tests -type f -name '*.lean' -print | sort)
fi
RAW="$(mktemp)"; trap 'rm -f "$RAW"' EXIT

for f in "${FILES[@]}"; do
  echo "  $f" >&2
  lake env lean -Ddiagnostics=true -Ddiagnostics.threshold=0 "$f" 2>&1 \
    | grep -aE '^\s+\[simp\] .* ↦ ' >> "$RAW"
done

# `name ↦ N ❌️`  (tried, never applied)  |  `name ↦ N, succeeded: M`
sed -E 's/^[[:space:]]*\[simp\][[:space:]]*//' "$RAW" \
  | awk '
      # Only the "tried theorems" block is counted.  Lean also prints a
      # "used theorems" summary whose entries carry neither ❌️ nor a
      # "succeeded:" count; those are a subset of the tried ones and counting
      # them again would inflate every try total.
      /❌️/       { sub(/ ❌️.*/,"",$0); split($0,a," ↦ "); tried[a[1]]+=a[2]; seen[a[1]]=1; next }
      /succeeded/ { split($0,a," ↦ "); split(a[2],b,", succeeded: ");
                    tried[a[1]]+=b[1]; ok[a[1]]+=b[2]; seen[a[1]]=1; next }
      END {
        for (k in seen) printf "%d\t%d\t%s\n", tried[k], (k in ok?ok[k]:0), k
      }' | sort -rn > "$RAW.agg"

awk -F'\t' '{tt+=$1; ts+=$2; n++; if ($2==0) {dt+=$1; d++}}
  END {printf "\n=== simp audit: %d distinct theorems ===\n", n
       printf "tried %d times, succeeded %d  (%.1f%% of tries are wasted work)\n", tt, ts, 100.0*(tt-ts)/tt
       printf "never applied at all: %d theorems, %d wasted tries\n\n", d, dt}' "$RAW.agg"

echo "top 25 never-applied (tries · theorem):"
awk -F'\t' '$2==0 {printf "  %8d  %s\n", $1, $3}' "$RAW.agg" | head -25
echo ""
echo "top 15 worst hit-rate among those that do fire (tries/hits · theorem):"
awk -F'\t' '$2>0 && $1>=50 {printf "  %8d /%6d  %5.1f%%  %s\n", $1, $2, 100.0*$2/$1, $3}' "$RAW.agg" \
  | sort -t% -k1 -n | head -15
rm -f "$RAW.agg"
