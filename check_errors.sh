#!/bin/bash
cd /home/marcin/windsurf/Projects/SolanaArbitrageBot
cargo check 2>&1 | tee compilation_errors.txt
echo "---"
echo "Error count: $(grep -c "error\[" compilation_errors.txt || echo 0)"
echo "Warning count: $(grep -c "warning:" compilation_errors.txt || echo 0)"
