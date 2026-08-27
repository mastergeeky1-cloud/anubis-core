#!/usr/bin/env bash
set -e
cd /home/ubuntu/anubis-core
git add -A
echo "=== staged ==="
git status --short
echo "target/ still tracked count: $(git ls-files | grep -c '^target/' || true)"
