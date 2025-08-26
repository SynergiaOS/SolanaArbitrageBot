#!/bin/bash

# Test compilation and check for errors
echo "🔧 Testing Solana Arbitrage Bot Compilation..."
echo "==========================================="

# Clean previous builds
echo "📦 Cleaning previous builds..."
cargo clean

# Check syntax and types without building
echo "🔍 Running cargo check..."
cargo check 2>&1 | tee check_output.log

# Count errors
ERRORS=$(grep -c "error\[" check_output.log || echo "0")
WARNINGS=$(grep -c "warning:" check_output.log || echo "0")

echo ""
echo "📊 Compilation Results:"
echo "----------------------"
echo "❌ Errors: $ERRORS"
echo "⚠️  Warnings: $WARNINGS"

if [ "$ERRORS" -gt 0 ]; then
    echo ""
    echo "🚨 Compilation failed with errors!"
    echo "Please review check_output.log for details."
    exit 1
else
    echo ""
    echo "✅ No compilation errors found!"
    
    if [ "$WARNINGS" -gt 0 ]; then
        echo "⚠️  There are $WARNINGS warnings that should be reviewed."
    fi
    
    # Try to build if no errors
    echo ""
    echo "🔨 Attempting full build..."
    cargo build --release
    
    if [ $? -eq 0 ]; then
        echo "✅ Build successful!"
    else
        echo "❌ Build failed!"
        exit 1
    fi
fi
