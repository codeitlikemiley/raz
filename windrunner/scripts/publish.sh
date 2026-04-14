#!/usr/bin/env bash
set -e

# Color output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

DRY_RUN="--dry-run"

if [ "$1" == "--execute" ]; then
    DRY_RUN=""
    echo -e "${YELLOW}WARNING: Executing real publish to crates.io!${NC}"
else
    echo -e "${GREEN}Running in dry-run mode. Use --execute to actually publish.${NC}"
fi

echo "Publishing cargo-runner-core..."
cd crates/core
cargo publish $DRY_RUN --allow-dirty
cd ../..

echo "Waiting for core crate to propagate on crates.io..."
if [ -z "$DRY_RUN" ]; then
    sleep 30
fi

echo "Publishing cargo-runner cli..."
cd crates/cli
cargo publish $DRY_RUN --allow-dirty
cd ../..

echo -e "${GREEN}Publish sequence complete!${NC}"
