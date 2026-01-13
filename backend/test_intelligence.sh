#!/bin/bash

# Test script for Intelligence API (Brain with tool selection)
# This script tests if the Brain correctly uses available tools

set -e

# Configuration
BASE_URL="${BASE_URL:-http://localhost:3030}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "Testing Intelligence API at $BASE_URL"
echo "======================================"
echo ""

# Check if server is reachable
echo -e "${YELLOW}Checking server connectivity...${NC}"
if ! curl -s -f "$BASE_URL/healthz" > /dev/null 2>&1; then
    echo -e "${RED}Error: Cannot reach server at $BASE_URL${NC}"
    echo "Please make sure the server is running."
    exit 1
fi
echo -e "${GREEN}Server is reachable${NC}"
echo ""

# Test function with error handling
test_api() {
    local test_name="$1"
    local payload="$2"
    
    echo -e "${YELLOW}$test_name${NC}"
    echo "--------------------------------------"
    
    # Make request and capture both response and status code
    HTTP_CODE=$(curl -s -w "%{http_code}" -o /tmp/response.json \
        -X POST "$BASE_URL/agent/pulse" \
        -H "Content-Type: application/json" \
        -d "$payload")
    
    # Check HTTP status code
    if [ "$HTTP_CODE" -ge 200 ] && [ "$HTTP_CODE" -lt 300 ]; then
        echo -e "${GREEN}HTTP Status: $HTTP_CODE${NC}"
        echo "Response:"
        if [ -s /tmp/response.json ]; then
            cat /tmp/response.json | jq '.' 2>/dev/null || cat /tmp/response.json
        else
            echo -e "${RED}Empty response body${NC}"
        fi
    else
        echo -e "${RED}HTTP Status: $HTTP_CODE${NC}"
        echo "Error Response:"
        cat /tmp/response.json 2>/dev/null || echo "No response body"
    fi
    echo ""
}

# Test 1: Request with Researcher agent - should trigger research tool
test_api "Test 1: Request with Researcher agent" '{
    "text": "Who is the president of the United States?",
    "agents": ["researcher"]
}'

# Test 2: Request with Refiner agent - should trigger refine tool
test_api "Test 2: Request with Refiner agent" '{
    "text": "This is a test text that needs improvement.",
    "agents": ["refiner"]
}'

# Test 3: Request with both agents - Brain should decide which tools to use
test_api "Test 3: Request with both agents (Brain decides)" '{
    "text": "The quick brown fox jumps over the lazy dog. This sentence needs fact-checking and improvement.",
    "agents": ["researcher", "refiner"]
}'

# Test 4: Request with text that needs research
test_api "Test 4: Text that should trigger research" '{
    "text": "What is the capital of France?",
    "agents": ["researcher", "refiner"]
}'

# Test 5: Request with text that needs refinement
test_api "Test 5: Text that should trigger refinement" '{
    "text": "this text has many errors and need improvment",
    "agents": ["researcher", "refiner"]
}'

echo "======================================"
echo -e "${GREEN}All tests completed!${NC}"
echo ""
echo "Expected behavior:"
echo "- Brain should analyze the text and available tools"
echo "- Brain should call OpenAI to decide which tools to use"
echo "- Selected tools should be executed"
echo "- Response should contain suggestions from executed agents"

# Cleanup
rm -f /tmp/response.json
