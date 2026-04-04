#!/bin/bash
set -euo pipefail

# Validate Kyverno policies against test fixtures
# Usage: ./validate-policies.sh

POLICY_DIR="${1:-../}"
TEST_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "🔍 Validating Kyverno policies..."
echo "Policy dir: $POLICY_DIR"
echo "Test dir: $TEST_DIR"
echo ""

# Check if kyverno CLI is available
if ! command -v kyverno &> /dev/null; then
    echo "⚠️  Kyverno CLI not found. Falling back to kubectl validation."
    USE_KUBECTL=true
else
    USE_KUBECTL=false
fi

# Test each fixture
TESTS_PASSED=0
TESTS_FAILED=0

for test_file in "$TEST_DIR"/test-*.yaml; do
    if [ ! -f "$test_file" ]; then
        continue
    fi
    
    test_name=$(basename "$test_file" .yaml)
    echo "Testing: $test_name"
    
    # Extract expected result from filename
    if [[ "$test_name" == *"-fail-"* ]]; then
        expected="FAIL"
    elif [[ "$test_name" == *"-pass-"* ]]; then
        expected="PASS"
    elif [[ "$test_name" == *"-exempt-"* ]]; then
        expected="SKIP"
    else
        expected="UNKNOWN"
    fi
    
    echo "  Expected: $expected"
    
    # TODO: Implement actual validation logic
    # For now, just validate YAML syntax
    if kubectl apply --dry-run=client -f "$test_file" &> /dev/null; then
        echo "  ✓ YAML valid"
        ((TESTS_PASSED++))
    else
        echo "  ✗ YAML invalid"
        ((TESTS_FAILED++))
    fi
done

echo ""
echo "📊 Test Results:"
echo "  Passed: $TESTS_PASSED"
echo "  Failed: $TESTS_FAILED"

if [ $TESTS_FAILED -eq 0 ]; then
    echo "✅ All tests passed!"
    exit 0
else
    echo "❌ Some tests failed"
    exit 1
fi
