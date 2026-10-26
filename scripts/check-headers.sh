# SPDX-License-Identifier: MIT
# Copyright (c) 2026 Michele Broggi
# Fail when a source file does not carry the MIT licence header.
# Usage: scripts/check-headers.sh
set -eu
missing=0
for file in $(git ls-files 2>/dev/null || find . -path ./target -prune -o -type f -print); do
  case "$file" in
    ./target/*|target/*|*.png|*.ico|*.iso|*.dat|LICENSE|Cargo.lock) continue ;;
    *.rs|*.toml|*.sh|*.yml|*.yaml|*.md) ;;
    *) continue ;;
  esac
  if ! head -n 3 "$file" | grep -q "SPDX-License-Identifier: MIT"; then
    echo "missing MIT header: $file"
    missing=1
  fi
done
exit $missing
