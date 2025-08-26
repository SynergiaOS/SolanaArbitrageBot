#!/bin/bash

# 🔧 Automatic Fix Script for Solana Arbitrage Bot
# This script applies automatic fixes for known issues

echo "🤖 Solana Arbitrage Bot - Automatic Fix Script"
echo "=============================================="
echo ""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    if [ "$1" = "success" ]; then
        echo -e "${GREEN}✅ $2${NC}"
    elif [ "$1" = "error" ]; then
        echo -e "${RED}❌ $2${NC}"
    elif [ "$1" = "warning" ]; then
        echo -e "${YELLOW}⚠️  $2${NC}"
    else
        echo "$2"
    fi
}

# Backup function
backup_file() {
    if [ -f "$1" ]; then
        cp "$1" "$1.backup.$(date +%Y%m%d_%H%M%S)"
        print_status "success" "Backed up $1"
    fi
}

echo "📋 Starting automatic fixes..."
echo ""

# Fix 1: Ensure rust_decimal has correct features
echo "1️⃣ Checking Cargo.toml for rust_decimal features..."
if grep -q 'rust_decimal.*serde-with-float' Cargo.toml; then
    print_status "success" "rust_decimal features already correct"
else
    print_status "warning" "rust_decimal missing features - fixing..."
    backup_file "Cargo.toml"
    sed -i 's/rust_decimal = .*/rust_decimal = { version = "1.36", features = ["serde-with-float", "serde"] }/' Cargo.toml
    print_status "success" "Fixed rust_decimal features in Cargo.toml"
fi

echo ""

# Fix 2: Create missing directories
echo "2️⃣ Checking required directories..."
REQUIRED_DIRS=("logs" "data" "test_results")
for dir in "${REQUIRED_DIRS[@]}"; do
    if [ ! -d "$dir" ]; then
        mkdir -p "$dir"
        print_status "success" "Created directory: $dir"
    else
        print_status "success" "Directory exists: $dir"
    fi
done

echo ""

# Fix 3: Set correct permissions on sensitive files
echo "3️⃣ Setting secure permissions..."
if [ -f ".env" ]; then
    chmod 600 .env
    print_status "success" "Set secure permissions on .env"
fi

if [ -f "config.yaml" ]; then
    chmod 644 config.yaml
    print_status "success" "Set permissions on config.yaml"
fi

echo ""

# Fix 4: Check for wallet files in repo
echo "4️⃣ Checking for wallet files in repository..."
WALLET_FILES=$(find . -name "*wallet*.json" -o -name "*.key" 2>/dev/null | grep -v node_modules | grep -v target)
if [ -z "$WALLET_FILES" ]; then
    print_status "success" "No wallet files found in repository"
else
    print_status "error" "Found wallet files that should be removed:"
    echo "$WALLET_FILES"
    echo ""
    read -p "Remove these files? (y/n): " -n 1 -r
    echo ""
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        echo "$WALLET_FILES" | while read -r file; do
            rm "$file"
            print_status "success" "Removed: $file"
        done
    fi
fi

echo ""

# Fix 5: Update dependencies
echo "5️⃣ Updating dependencies..."
print_status "warning" "Running cargo update..."
cargo update 2>/dev/null
if [ $? -eq 0 ]; then
    print_status "success" "Dependencies updated"
else
    print_status "error" "Failed to update dependencies"
fi

echo ""

# Fix 6: Run cargo fmt
echo "6️⃣ Formatting code..."
if command -v rustfmt &> /dev/null; then
    cargo fmt 2>/dev/null
    print_status "success" "Code formatted"
else
    print_status "warning" "rustfmt not installed - skipping"
fi

echo ""

# Fix 7: Run cargo clippy for linting
echo "7️⃣ Running linter..."
if command -v cargo-clippy &> /dev/null; then
    cargo clippy --fix --allow-dirty --allow-staged 2>/dev/null
    if [ $? -eq 0 ]; then
        print_status "success" "Linting fixes applied"
    else
        print_status "warning" "Some linting issues couldn't be auto-fixed"
    fi
else
    print_status "warning" "clippy not installed - skipping"
fi

echo ""

# Final compilation test
echo "8️⃣ Testing compilation..."
cargo check 2>&1 | tee /tmp/cargo_check.log > /dev/null
ERRORS=$(grep -c "error\[" /tmp/cargo_check.log || echo "0")

echo ""
echo "========================================"
echo "📊 Fix Script Results:"
echo "========================================"

if [ "$ERRORS" -eq 0 ]; then
    print_status "success" "All automatic fixes applied successfully!"
    print_status "success" "No compilation errors found!"
    echo ""
    echo "🚀 Next steps:"
    echo "   1. Review AUDIT_REPORT.md for remaining issues"
    echo "   2. Run ./test_compilation.sh for full build test"
    echo "   3. Fix configuration mismatches manually"
else
    print_status "error" "Found $ERRORS compilation errors after fixes"
    echo ""
    echo "⚠️  Manual intervention required for:"
    echo "   - Configuration mismatches"
    echo "   - Hard-coded values"
    echo "   - Missing safety features"
    echo ""
    echo "📖 See AUDIT_REPORT.md for details"
fi

echo ""
echo "✨ Fix script completed!"
