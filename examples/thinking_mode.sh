#!/usr/bin/env bash
# Extended Thinking Mode example script
# 
# This script demonstrates how to enable Extended Thinking mode for Claude Code ACP

set -e

# Color output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}Claude Code ACP - Extended Thinking Mode${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""

# Check required environment variables
if [ -z "$ANTHROPIC_API_KEY" ]; then
    echo -e "${YELLOW}Warning: ANTHROPIC_API_KEY is not set${NC}"
    echo "Please set your API key:"
    echo "  export ANTHROPIC_API_KEY='your-api-key'"
    echo ""
fi

# Configure Thinking mode
echo -e "${GREEN}Configuring Extended Thinking mode...${NC}"
export MAX_THINKING_TOKENS=4096
export ANTHROPIC_MODEL="claude-sonnet-4-20250514"

echo "  MAX_THINKING_TOKENS: $MAX_THINKING_TOKENS"
echo "  ANTHROPIC_MODEL: $ANTHROPIC_MODEL"
echo ""

# Optional: configure other parameters
# export ANTHROPIC_BASE_URL="https://api.anthropic.com"
# export ANTHROPIC_SMALL_FAST_MODEL="claude-3-5-haiku-20241022"

# Start agent
echo -e "${GREEN}Starting Claude Code ACP Agent...${NC}"
echo "The agent will use Extended Thinking mode to handle complex tasks"
echo ""

# If you need diagnostic logs, uncomment the line below
# exec ./target/release/claude-code-acp-rs --diagnostic -vv

# Normal launch
exec ./target/release/claude-code-acp-rs
