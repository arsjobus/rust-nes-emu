#!/usr/bin/env bash
set -uo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
manifest="$root/tests/rom-tests.tsv"
cd "$root"

if [[ ! -d "$root/test-roms/.git" ]]; then
    echo "Missing test ROM collection. See tests/README.md" >&2
    exit 2
fi

filter="${1:-}"
passed=0
failed=0
missing=0
log="$(mktemp)"
trap 'rm -f "$log"' EXIT
while IFS=$'\t' read -r name rom frames; do
    [[ -z "${name:-}" || "$name" == \#* ]] && continue
    [[ -n "$filter" && "$name" != *"$filter"* ]] && continue
    if [[ ! -f "$rom" ]]; then
        echo "MISSING $name ($rom)"
        missing=$((missing + 1))
        continue
    fi
    echo "=== $name ($rom, up to $frames frames) ==="
    if ROM="$rom" FRAMES="$frames" cargo test --quiet rom_harness -- --ignored --nocapture >"$log" 2>&1; then
        grep 'RESULT status=' "$log" | tail -n 1 || true
        echo "PASS $name"
        passed=$((passed + 1))
    else
        sed -n '/RESULT status=/,$p' "$log" | head -n 10
        echo "FAIL $name"
        failed=$((failed + 1))
    fi
done < "$manifest"

echo "ROM tests: $passed passed, $failed failed, $missing missing"
[[ $failed -eq 0 && $missing -eq 0 ]]
