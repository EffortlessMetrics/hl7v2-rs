#!/usr/bin/env bash
#
# BDD Tests for EFF-1265: Node.js .gitignore patterns
#
# These tests express the intended behavior BEFORE implementation.
# They will FAIL until the fix is implemented.
#
# Usage: ./scripts/tests/test_gitignore_nodejs.sh
#

set -uo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Test counters
TESTS_PASSED=0
TESTS_FAILED=0

# Repository root
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
GITIGNORE_FILE="$REPO_ROOT/.gitignore"

# Helper functions
pass() {
    echo -e "${GREEN}✓ PASS${NC}: $1"
    ((TESTS_PASSED++))
}

fail() {
    echo -e "${RED}✗ FAIL${NC}: $1"
    ((TESTS_FAILED++))
}

# Test: node_modules pattern exists in .gitignore
test_node_modules_pattern_exists() {
    echo ""
    echo "=== Test: node_modules pattern exists in .gitignore ==="
    
    local result=0
    grep -q "^node_modules/" "$GITIGNORE_FILE" 2>/dev/null || result=$?
    
    if [[ $result -eq 0 ]]; then
        pass "node_modules/ pattern exists in .gitignore"
    else
        fail "node_modules/ pattern MISSING from .gitignore - Node.js dependencies could be accidentally committed"
    fi
}

# Test: package-lock.json pattern exists in .gitignore
test_package_lock_pattern_exists() {
    echo ""
    echo "=== Test: package-lock.json pattern exists in .gitignore ==="
    
    local result=0
    grep -q "^package-lock.json" "$GITIGNORE_FILE" 2>/dev/null || result=$?
    
    if [[ $result -eq 0 ]]; then
        pass "package-lock.json pattern exists in .gitignore"
    else
        fail "package-lock.json pattern MISSING from .gitignore - lock file could be accidentally committed"
    fi
}

# Test: node_modules directory is actually ignored by git
test_node_modules_is_ignored() {
    echo ""
    echo "=== Test: node_modules directory is ignored by git ==="
    
    # First check if pattern exists
    local pattern_exists=0
    grep -q "^node_modules/" "$GITIGNORE_FILE" 2>/dev/null || pattern_exists=$?
    
    if [[ $pattern_exists -ne 0 ]]; then
        fail "Cannot test - node_modules/ pattern not yet in .gitignore"
        return
    fi
    
    # Create a test node_modules directory
    local test_dir="$REPO_ROOT/node_modules"
    local cleanup_needed=false
    
    if [[ ! -d "$test_dir" ]]; then
        mkdir -p "$test_dir/test-package"
        echo '{}' > "$test_dir/test-package/package.json"
        cleanup_needed=true
    fi
    
    # Check if git check-ignore recognizes it
    if git -C "$REPO_ROOT" check-ignore "$test_dir" 2>/dev/null; then
        pass "node_modules directory is correctly ignored by git"
    else
        fail "node_modules directory NOT ignored by git - would appear in 'git status'"
    fi
    
    # Cleanup if we created it
    if [[ "$cleanup_needed" == true ]]; then
        rm -rf "$test_dir"
    fi
}

# Test: package-lock.json file is actually ignored by git
test_package_lock_is_ignored() {
    echo ""
    echo "=== Test: package-lock.json file is ignored by git ==="
    
    # First check if pattern exists
    local pattern_exists=0
    grep -q "^package-lock.json" "$GITIGNORE_FILE" 2>/dev/null || pattern_exists=$?
    
    if [[ $pattern_exists -ne 0 ]]; then
        fail "Cannot test - package-lock.json pattern not yet in .gitignore"
        return
    fi
    
    # Create a test package-lock.json file
    local test_file="$REPO_ROOT/package-lock.json"
    local cleanup_needed=false
    
    if [[ ! -f "$test_file" ]]; then
        echo '{}' > "$test_file"
        cleanup_needed=true
    fi
    
    # Check if git check-ignore recognizes it
    if git -C "$REPO_ROOT" check-ignore "$test_file" 2>/dev/null; then
        pass "package-lock.json file is correctly ignored by git"
    else
        fail "package-lock.json file NOT ignored by git - would appear in 'git status'"
    fi
    
    # Cleanup if we created it
    if [[ "$cleanup_needed" == true ]]; then
        rm -f "$test_file"
    fi
}

# Test: Pattern has correct format with trailing slash
test_node_modules_pattern_format() {
    echo ""
    echo "=== Test: node_modules pattern has correct format ==="
    
    local pattern
    pattern=$(grep "^node_modules" "$GITIGNORE_FILE" 2>/dev/null || true)
    
    if [[ -z "$pattern" ]]; then
        fail "node_modules pattern not found - cannot verify format"
        return
    fi
    
    if [[ "$pattern" == *"/" ]]; then
        pass "node_modules pattern ends with '/' (directory format)"
    else
        fail "node_modules pattern missing trailing '/' - should be 'node_modules/' not 'node_modules'"
    fi
    
    if [[ "$pattern" == *\\\* ]]; then
        fail "node_modules pattern contains backslash - should use forward slash"
    else
        pass "node_modules pattern uses forward slash format"
    fi
}

# Test: Section header comment exists
test_section_header_exists() {
    echo ""
    echo "=== Test: Node.js section header exists ==="
    
    # Check for Node.js patterns in file
    local has_node_modules=0
    local has_pkg_lock=0
    grep -q "^node_modules/" "$GITIGNORE_FILE" 2>/dev/null || has_node_modules=$?
    grep -q "^package-lock.json" "$GITIGNORE_FILE" 2>/dev/null || has_pkg_lock=$?
    
    if [[ $has_node_modules -ne 0 && $has_pkg_lock -ne 0 ]]; then
        fail "Cannot verify - Node.js patterns not yet added to .gitignore"
        return
    fi
    
    # Look for a comment indicating Node.js section near the patterns
    # Get line numbers of patterns
    local node_line pkg_line
    node_line=$(grep -n "^node_modules/" "$GITIGNORE_FILE" 2>/dev/null | head -1 | cut -d: -f1 || echo "0")
    pkg_line=$(grep -n "^package-lock.json" "$GITIGNORE_FILE" 2>/dev/null | head -1 | cut -d: -f1 || echo "0")
    
    # Check up to 5 lines before the first pattern for a comment
    local first_line=$node_line
    if [[ $pkg_line -gt 0 && ($node_line -eq 0 || $pkg_line -lt $node_line) ]]; then
        first_line=$pkg_line
    fi
    
    if [[ $first_line -gt 1 ]]; then
        local start_line=$((first_line - 5))
        if [[ $start_line -lt 1 ]]; then
            start_line=1
        fi
        
        local context
        context=$(sed -n "${start_line},${first_line}p" "$GITIGNORE_FILE" 2>/dev/null || true)
        
        if echo "$context" | grep -qi "node\|npm"; then
            pass "Node.js section header comment found near patterns"
        else
            fail "Node.js patterns lack descriptive section header comment"
        fi
    else
        fail "Node.js patterns at beginning of file - should have section header"
    fi
}

# Test: Existing patterns still work (regression test)
test_existing_patterns_still_work() {
    echo ""
    echo "=== Test: Existing .gitignore patterns still work ==="
    
    local result=0
    
    # Check that target/ is still ignored
    result=0
    grep -q "^target" "$GITIGNORE_FILE" 2>/dev/null || result=$?
    if [[ $result -eq 0 ]]; then
        pass "target/ pattern still exists in .gitignore"
    else
        fail "target/ pattern missing from .gitignore - regression!"
    fi
    
    # Check that .env is still ignored
    result=0
    grep -q "^\\.env" "$GITIGNORE_FILE" 2>/dev/null || result=$?
    if [[ $result -eq 0 ]]; then
        pass ".env pattern still exists in .gitignore"
    else
        fail ".env pattern missing from .gitignore - regression!"
    fi
    
    # Check that *.rs.bk is still ignored
    result=0
    grep -q "\\*\\.rs\\.bk" "$GITIGNORE_FILE" 2>/dev/null || result=$?
    if [[ $result -eq 0 ]]; then
        pass "*.rs.bk pattern still exists in .gitignore"
    else
        fail "*.rs.bk pattern missing from .gitignore - regression!"
    fi
}

# Main test runner
main() {
    echo "========================================"
    echo "BDD Tests: EFF-1265 - Node.js .gitignore Patterns"
    echo "========================================"
    echo ""
    echo "These tests express the INTENDED behavior."
    echo "They will FAIL until the implementation is complete."
    echo ""
    
    # Pre-check: .gitignore exists
    if [[ ! -f "$GITIGNORE_FILE" ]]; then
        echo "ERROR: .gitignore file not found at $GITIGNORE_FILE"
        exit 1
    fi
    
    echo "Testing .gitignore file: $GITIGNORE_FILE"
    echo ""
    
    # Run all tests
    test_node_modules_pattern_exists
    test_package_lock_pattern_exists
    test_node_modules_is_ignored
    test_package_lock_is_ignored
    test_node_modules_pattern_format
    test_section_header_exists
    test_existing_patterns_still_work
    
    # Summary
    echo ""
    echo "========================================"
    echo "Test Summary"
    echo "========================================"
    echo -e "${GREEN}Passed: $TESTS_PASSED${NC}"
    echo -e "${RED}Failed: $TESTS_FAILED${NC}"
    echo ""
    
    if [[ $TESTS_FAILED -eq 0 ]]; then
        echo -e "${GREEN}All tests passed!${NC}"
        exit 0
    else
        echo -e "${RED}Tests FAILED - Implementation needed${NC}"
        echo ""
        echo "Expected fix:"
        echo "  Add to .gitignore:"
        echo "    # Node.js dependencies"
        echo "    node_modules/"
        echo "    package-lock.json"
        exit 1
    fi
}

main "$@"
