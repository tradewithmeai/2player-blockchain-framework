#!/bin/bash

# Extract error codes from Rust programs and generate documentation
# Usage: ./scripts/extract-errors.sh

set -e

OUTPUT_FILE="docs/ERROR_CODES_GENERATED.md"

echo "🔍 Extracting error codes from programs..."

# Create output directory
mkdir -p docs

# Start the markdown file
cat > "$OUTPUT_FILE" << EOF
# Error Code Registry (Auto-Generated)

> ⚠️ This file is auto-generated from source code.

Generated on: $(date)

## How to Use

1. Find your error code in the list below
2. Read the message
3. Refer to ERROR_CODES.md for detailed resolution steps

---

EOF

# Function to extract errors from a program
extract_errors() {
    local program_name=$1
    local source_file=$2

    echo "## $program_name" >> "$OUTPUT_FILE"
    echo "" >> "$OUTPUT_FILE"

    if [ -f "$source_file" ]; then
        # Extract error messages using grep
        grep -A 1 "#\[msg(" "$source_file" | grep -E "(#\[msg|^\s+[A-Z])" | \
        sed 'N;s/\n/ /' | \
        sed 's/#\[msg("\(.*\)"\)\]/\1/' | \
        sed 's/^\s*/- /' | \
        sed 's/,$//' >> "$OUTPUT_FILE" || echo "   (No errors defined)" >> "$OUTPUT_FILE"

        echo "" >> "$OUTPUT_FILE"
    else
        echo "⚠️  Source file not found: $source_file"
        echo "   (Source file not found)" >> "$OUTPUT_FILE"
        echo "" >> "$OUTPUT_FILE"
    fi
}

# Extract from each program
extract_errors "skill_treasury" "programs/skill_treasury/src/lib.rs"
extract_errors "skill_escrow" "programs/skill_escrow/src/lib.rs"
extract_errors "ttt_onchain" "programs/ttt_onchain/src/lib.rs"

echo "---" >> "$OUTPUT_FILE"
echo "" >> "$OUTPUT_FILE"
echo "For detailed error resolutions, see [ERROR_CODES.md](./ERROR_CODES.md)" >> "$OUTPUT_FILE"

echo "✅ Error codes extracted to $OUTPUT_FILE"
echo ""
echo "📊 Summary:"
total_errors=$(grep -c "^-" "$OUTPUT_FILE" || echo "0")
echo "   - Total error codes extracted: $total_errors"
echo ""
echo "Next steps:"
echo "1. Review $OUTPUT_FILE"
echo "2. Consult ERROR_CODES.md for detailed resolutions"
echo "3. Run 'cargo doc' to generate full Rust documentation"
