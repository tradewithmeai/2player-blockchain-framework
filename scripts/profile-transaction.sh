#!/bin/bash

# Profile a specific transaction to extract compute unit usage
# Usage: ./scripts/profile-transaction.sh <signature> [network]

set -e

SIGNATURE=$1
NETWORK=${2:-devnet}

if [ -z "$SIGNATURE" ]; then
    echo "❌ Error: Transaction signature required"
    echo ""
    echo "Usage: $0 <signature> [network]"
    echo ""
    echo "Example:"
    echo "  $0 5KZr...abc123 devnet"
    echo "  $0 3Hx...xyz789 mainnet-beta"
    echo ""
    exit 1
fi

echo "🔍 Profiling Transaction: $SIGNATURE"
echo "🌐 Network: $NETWORK"
echo ""

# Set Solana config to specified network
solana config set --url "https://api.$NETWORK.solana.com" > /dev/null 2>&1

echo "📡 Fetching transaction details..."
echo ""

# Fetch transaction with full details
TX_DATA=$(solana confirm "$SIGNATURE" -v 2>&1 || true)

if echo "$TX_DATA" | grep -q "Not found"; then
    echo "❌ Error: Transaction not found on $NETWORK"
    echo ""
    echo "Possible reasons:"
    echo "  - Signature is incorrect"
    echo "  - Transaction is on a different network"
    echo "  - Transaction is too old (pruned from RPC)"
    echo ""
    exit 1
fi

echo "✅ Transaction found!"
echo ""

# Extract compute units from logs
if echo "$TX_DATA" | grep -q "Consumed.*compute units"; then
    echo "📊 Compute Unit Usage:"
    echo "──────────────────────"

    CONSUMED=$(echo "$TX_DATA" | grep "Consumed.*compute units" | sed -n 's/.*Consumed \([0-9]*\) of \([0-9]*\).*/\1/p' | head -n 1)
    LIMIT=$(echo "$TX_DATA" | grep "Consumed.*compute units" | sed -n 's/.*Consumed \([0-9]*\) of \([0-9]*\).*/\2/p' | head -n 1)

    if [ -n "$CONSUMED" ] && [ -n "$LIMIT" ]; then
        PERCENTAGE=$(awk "BEGIN {printf \"%.2f\", ($CONSUMED/$LIMIT)*100}")

        # Status indicator
        if [ "$CONSUMED" -lt 50000 ]; then
            STATUS="✅ Excellent"
        elif [ "$CONSUMED" -lt 100000 ]; then
            STATUS="⚠️  Moderate"
        else
            STATUS="❌ High"
        fi

        echo "  Consumed: $CONSUMED CU"
        echo "  Limit:    $LIMIT CU"
        echo "  Usage:    $PERCENTAGE%"
        echo "  Status:   $STATUS"
    else
        echo "  Could not parse compute unit data"
    fi
else
    echo "⚠️  No compute unit data in transaction logs"
fi

echo ""

# Extract program logs
if echo "$TX_DATA" | grep -q "Program log:"; then
    echo "📝 Program Logs:"
    echo "────────────────"
    echo "$TX_DATA" | grep "Program log:" | sed 's/.*Program log: /  /' | head -n 20
    echo ""
fi

# Extract any errors
if echo "$TX_DATA" | grep -q "Error:"; then
    echo "❌ Errors:"
    echo "──────────"
    echo "$TX_DATA" | grep "Error:" | sed 's/.*Error: /  /' | head -n 10
    echo ""
fi

# Transaction summary
echo "📋 Transaction Summary:"
echo "───────────────────────"

if echo "$TX_DATA" | grep -q "Status:"; then
    STATUS=$(echo "$TX_DATA" | grep "Status:" | sed 's/.*Status: //')
    echo "  Status: $STATUS"
fi

if echo "$TX_DATA" | grep -q "Slot:"; then
    SLOT=$(echo "$TX_DATA" | grep "Slot:" | sed 's/.*Slot: //' | awk '{print $1}')
    echo "  Slot:   $SLOT"
fi

echo ""
echo "🔗 View full transaction:"
echo "   https://explorer.solana.com/tx/$SIGNATURE?cluster=$NETWORK"
echo ""
