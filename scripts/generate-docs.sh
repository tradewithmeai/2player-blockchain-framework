#!/bin/bash

# Generate comprehensive documentation for Skill Gaming programs
# Usage: ./scripts/generate-docs.sh [--open]

set -e

OPEN_DOCS=false

# Parse arguments
if [ "$1" = "--open" ]; then
    OPEN_DOCS=true
fi

echo "📚 Generating Documentation for Skill Gaming Programs"
echo "====================================================="
echo ""

# Check if cargo is available
if ! command -v cargo &> /dev/null; then
    echo "❌ Error: cargo not found"
    echo "   Please install Rust: https://rustup.rs/"
    exit 1
fi

echo "🔧 Building programs..."
echo ""

# Build programs first to ensure all dependencies are available
cargo build --release

echo ""
echo "📖 Generating rustdoc..."
echo ""

# Generate documentation with all features enabled
# --no-deps: Don't document dependencies
# --document-private-items: Include private items (useful for internal docs)
# --open: Open docs in browser (optional)

RUSTDOCFLAGS="--cfg docsrs" cargo doc \
    --workspace \
    --no-deps \
    --all-features \
    --document-private-items

echo ""
echo "✅ Documentation generated successfully!"
echo ""

# Find the doc directory
DOC_DIR="target/doc"

if [ ! -d "$DOC_DIR" ]; then
    echo "❌ Error: Documentation directory not found at $DOC_DIR"
    exit 1
fi

echo "📁 Documentation location: $DOC_DIR"
echo ""

# List generated docs
echo "📚 Generated documentation for:"
echo "  - skill_treasury:  $DOC_DIR/skill_treasury/index.html"
echo "  - skill_escrow:    $DOC_DIR/skill_escrow/index.html"
echo "  - ttt_onchain:     $DOC_DIR/ttt_onchain/index.html"
echo ""

# Create an index.html that redirects to skill_treasury
cat > "$DOC_DIR/index.html" << 'EOF'
<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>Skill Gaming Documentation</title>
    <style>
        body {
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
            max-width: 800px;
            margin: 50px auto;
            padding: 20px;
            line-height: 1.6;
        }
        h1 {
            border-bottom: 2px solid #333;
            padding-bottom: 10px;
        }
        .program {
            background: #f5f5f5;
            padding: 15px;
            margin: 10px 0;
            border-radius: 5px;
        }
        .program h3 {
            margin-top: 0;
        }
        a {
            color: #0066cc;
            text-decoration: none;
        }
        a:hover {
            text-decoration: underline;
        }
        .description {
            color: #666;
        }
    </style>
</head>
<body>
    <h1>🎮 Skill Gaming Platform - Documentation</h1>

    <p>Welcome to the Skill Gaming platform documentation. This is a 2-player blockchain gaming framework built on Solana.</p>

    <h2>Programs</h2>

    <div class="program">
        <h3><a href="skill_treasury/index.html">skill_treasury</a></h3>
        <p class="description">
            SOL to SKILL token mint/redeem treasury. Provides fixed-rate conversion
            between SOL and SKILL tokens with full backing.
        </p>
    </div>

    <div class="program">
        <h3><a href="skill_escrow/index.html">skill_escrow</a></h3>
        <p class="description">
            Match escrow and settlement for skill-based games. Manages match creation,
            funding, and settlement with support for both on-chain and off-chain game logic.
        </p>
    </div>

    <div class="program">
        <h3><a href="ttt_onchain/index.html">ttt_onchain</a></h3>
        <p class="description">
            On-chain Tic-Tac-Toe game logic with trustless settlement. Example game
            implementation showing turn-based gameplay with timeout protection.
        </p>
    </div>

    <h2>Quick Links</h2>
    <ul>
        <li><a href="../../../README.md">Project README</a></li>
        <li><a href="../../../docs/ERROR_CODES.md">Error Code Registry</a></li>
        <li><a href="../../../docs/BENCHMARKING.md">Benchmarking Guide</a></li>
    </ul>

    <h2>Resources</h2>
    <ul>
        <li><a href="https://docs.solana.com">Solana Documentation</a></li>
        <li><a href="https://www.anchor-lang.com">Anchor Framework</a></li>
        <li><a href="https://github.com/yourusername/2player-blockchain-framework">GitHub Repository</a></li>
    </ul>
</body>
</html>
EOF

echo "📄 Created index page: $DOC_DIR/index.html"
echo ""

# Generate error codes documentation
echo "🔍 Generating error code documentation..."
if [ -f "scripts/extract-errors.sh" ]; then
    ./scripts/extract-errors.sh
    echo "✅ Error codes extracted"
else
    echo "⚠️  extract-errors.sh not found, skipping"
fi

echo ""
echo "═══════════════════════════════════════════════════"
echo "✅ Documentation Generation Complete!"
echo "═══════════════════════════════════════════════════"
echo ""

if [ "$OPEN_DOCS" = true ]; then
    echo "🌐 Opening documentation in browser..."

    # Try to open in browser (works on macOS and Linux)
    if command -v open &> /dev/null; then
        open "$DOC_DIR/index.html"
    elif command -v xdg-open &> /dev/null; then
        xdg-open "$DOC_DIR/index.html"
    else
        echo "⚠️  Could not open browser automatically"
    fi
fi

echo "📖 To view documentation:"
echo ""
echo "   Option 1: Open in browser"
echo "   file://$(pwd)/$DOC_DIR/index.html"
echo ""
echo "   Option 2: Serve with local HTTP server"
echo "   cd $DOC_DIR && python3 -m http.server 8000"
echo "   Then visit: http://localhost:8000"
echo ""
echo "   Option 3: Generate and open in one command"
echo "   ./scripts/generate-docs.sh --open"
echo ""

# Documentation stats
echo "📊 Documentation Statistics:"
echo "───────────────────────────"

total_html_files=$(find "$DOC_DIR" -name "*.html" | wc -l)
echo "   Total HTML files: $total_html_files"

for program in skill_treasury skill_escrow ttt_onchain; do
    if [ -d "$DOC_DIR/$program" ]; then
        program_files=$(find "$DOC_DIR/$program" -name "*.html" | wc -l)
        echo "   $program: $program_files files"
    fi
done

echo ""
echo "Next steps:"
echo "1. Review generated documentation"
echo "2. Consider hosting on GitHub Pages or docs.rs"
echo "3. Add documentation links to README.md"
echo "4. Set up CI/CD to regenerate docs on updates"
