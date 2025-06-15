#!/bin/bash

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Parse arguments
TEST_TYPE="debug"
VERBOSE=0
UNIT_ONLY=0
INTEGRATION_ONLY=0

while [[ $# -gt 0 ]]; do
    case $1 in
        release)
            TEST_TYPE="release"
            shift
            ;;
        -v|--verbose)
            VERBOSE=1
            shift
            ;;
        --unit-only)
            UNIT_ONLY=1
            shift
            ;;
        --integration-only)
            INTEGRATION_ONLY=1
            shift
            ;;
        -h|--help)
            echo "Usage: $0 [release] [-v|--verbose] [--unit-only|--integration-only] [-h|--help]"
            echo "  release              Test in release mode (default: debug)"
            echo "  -v,--verbose         Show test output"
            echo "  --unit-only          Run only unit tests"
            echo "  --integration-only   Run only integration tests"
            echo "  -h,--help           Show this help"
            exit 0
            ;;
        *)
            echo "Unknown argument: $1"
            echo "Use -h for help"
            exit 1
            ;;
    esac
done

TEST_FLAG=""
if [ "$TEST_TYPE" = "release" ]; then
    TEST_FLAG="--release"
fi

# Arrays to track test results
declare -a TEST_TARGETS
declare -a TEST_DESCRIPTIONS
declare -a TEST_RESULTS
declare -a TEST_TOOLS

echo -e "${BLUE}=== Cross-Platform Test Script ===${NC}"
echo -e "${BLUE}Test type: $TEST_TYPE${NC}"
if [ $VERBOSE -eq 1 ]; then
    echo -e "${BLUE}Verbose mode: enabled${NC}"
fi
if [ $UNIT_ONLY -eq 1 ]; then
    echo -e "${BLUE}Running: unit tests only${NC}"
elif [ $INTEGRATION_ONLY -eq 1 ]; then
    echo -e "${BLUE}Running: integration tests only${NC}"
else
    echo -e "${BLUE}Running: all tests${NC}"
fi
echo

# Function to check if command exists
check_command() {
    if command -v "$1" >/dev/null 2>&1; then
        echo -e "${GREEN}✓${NC} $1 is installed"
        return 0
    else
        echo -e "${RED}✗${NC} $1 is not installed"
        return 1
    fi
}

# Function to check if Rust target is installed
check_rust_target() {
    if rustup target list --installed | grep -q "$1"; then
        echo -e "${GREEN}✓${NC} Rust target $1 is installed"
        return 0
    else
        echo -e "${RED}✗${NC} Rust target $1 is not installed"
        return 1
    fi
}

# Function to check if Docker is running
check_docker() {
    if docker info >/dev/null 2>&1; then
        echo -e "${GREEN}✓${NC} Docker is running"
        return 0
    else
        echo -e "${RED}✗${NC} Docker is not running"
        return 1
    fi
}

# Function to attempt test and record result
attempt_test() {
    local tool=$1
    local target=$2
    local description=$3
    local test_cmd_args=$4
    
    TEST_TARGETS+=("$target")
    TEST_DESCRIPTIONS+=("$description")
    TEST_TOOLS+=("$tool")
    
    echo -e "${YELLOW}Testing $description...${NC}"
    
    # Determine output redirection based on verbose flag
    local output_redirect=""
    if [ $VERBOSE -eq 0 ]; then
        output_redirect=">/dev/null 2>&1"
    fi
    
    local full_cmd=""
    if [ "$tool" = "cargo" ]; then
        full_cmd="cargo test --target $target $TEST_FLAG $test_cmd_args"
    elif [ "$tool" = "cross" ]; then
        full_cmd="cross test --target $target $TEST_FLAG $test_cmd_args"
    fi
    
    echo -e "${BLUE}Running: $full_cmd${NC}"
    
    if [ $VERBOSE -eq 1 ]; then
        if eval "$full_cmd"; then
            TEST_RESULTS+=("SUCCESS")
            echo -e "${GREEN}✓${NC} Successfully tested $description"
        else
            TEST_RESULTS+=("FAILED")
            echo -e "${RED}✗${NC} Failed to test $description"
        fi
    else
        if eval "$full_cmd $output_redirect"; then
            TEST_RESULTS+=("SUCCESS")
            echo -e "${GREEN}✓${NC} Successfully tested $description"
        else
            TEST_RESULTS+=("FAILED")
            echo -e "${RED}✗${NC} Failed to test $description"
        fi
    fi
    echo
}

# Check prerequisites
echo -e "${BLUE}=== Checking Prerequisites ===${NC}"

MISSING_DEPS=0

# Check basic tools
check_command "rustup" || MISSING_DEPS=1
check_command "cargo" || MISSING_DEPS=1

# Check optional tools (don't fail if missing, just skip those tests)
CROSS_AVAILABLE=0
DOCKER_AVAILABLE=0
LINUX_ARM_AVAILABLE=0
MINGW_AVAILABLE=0

if check_command "cross"; then
    CROSS_AVAILABLE=1
fi

if check_docker; then
    DOCKER_AVAILABLE=1
fi

if check_command "aarch64-linux-gnu-gcc"; then
    LINUX_ARM_AVAILABLE=1
fi

if check_command "x86_64-w64-mingw32-gcc"; then
    MINGW_AVAILABLE=1
fi

echo

# Check Rust targets
echo -e "${BLUE}=== Checking Rust Targets ===${NC}"
TARGET_X86_LINUX=0
TARGET_ARM_LINUX=0
TARGET_WIN_GNU=0
TARGET_MAC_X86=0
TARGET_MAC_ARM=0

check_rust_target "x86_64-unknown-linux-gnu" && TARGET_X86_LINUX=1
check_rust_target "aarch64-unknown-linux-gnu" && TARGET_ARM_LINUX=1
check_rust_target "x86_64-pc-windows-gnu" && TARGET_WIN_GNU=1
check_rust_target "x86_64-apple-darwin" && TARGET_MAC_X86=1
check_rust_target "aarch64-apple-darwin" && TARGET_MAC_ARM=1

echo

if [ $MISSING_DEPS -eq 1 ]; then
    echo -e "${RED}Critical dependencies missing (rustup/cargo). Cannot continue.${NC}"
    exit 1
fi

# Determine test command arguments based on flags
TEST_CMD_ARGS=""
if [ $UNIT_ONLY -eq 1 ]; then
    TEST_CMD_ARGS="--lib"
elif [ $INTEGRATION_ONLY -eq 1 ]; then
    TEST_CMD_ARGS="--test '*'"
fi

echo -e "${GREEN}Starting tests with available tools...${NC}"
echo

# Start testing
echo -e "${BLUE}=== Cross-Platform Tests ===${NC}"

# Native test (always available)
if [ $TARGET_X86_LINUX -eq 1 ]; then
    attempt_test "cargo" "x86_64-unknown-linux-gnu" "Linux x86_64 (native)" "$TEST_CMD_ARGS"
else
    TEST_TARGETS+=("x86_64-unknown-linux-gnu")
    TEST_DESCRIPTIONS+=("Linux x86_64 (native)")
    TEST_TOOLS+=("cargo")
    TEST_RESULTS+=("SKIPPED - target not installed")
fi

# Linux ARM64 test
if [ $TARGET_ARM_LINUX -eq 1 ] && [ $LINUX_ARM_AVAILABLE -eq 1 ]; then
    attempt_test "cargo" "aarch64-unknown-linux-gnu" "Linux ARM64 (cross-compile)" "$TEST_CMD_ARGS"
elif [ $TARGET_ARM_LINUX -eq 0 ]; then
    TEST_TARGETS+=("aarch64-unknown-linux-gnu")
    TEST_DESCRIPTIONS+=("Linux ARM64 (cross-compile)")
    TEST_TOOLS+=("cargo")
    TEST_RESULTS+=("SKIPPED - target not installed")
elif [ $LINUX_ARM_AVAILABLE -eq 0 ]; then
    TEST_TARGETS+=("aarch64-unknown-linux-gnu")
    TEST_DESCRIPTIONS+=("Linux ARM64 (cross-compile)")
    TEST_TOOLS+=("cargo")
    TEST_RESULTS+=("SKIPPED - aarch64-linux-gnu-gcc not available")
fi

# Windows GNU test
if [ $TARGET_WIN_GNU -eq 1 ] && [ $MINGW_AVAILABLE -eq 1 ]; then
    attempt_test "cargo" "x86_64-pc-windows-gnu" "Windows x86_64 GNU (mingw-w64)" "$TEST_CMD_ARGS"
elif [ $TARGET_WIN_GNU -eq 0 ]; then
    TEST_TARGETS+=("x86_64-pc-windows-gnu")
    TEST_DESCRIPTIONS+=("Windows x86_64 GNU (mingw-w64)")
    TEST_TOOLS+=("cargo")
    TEST_RESULTS+=("SKIPPED - target not installed")
elif [ $MINGW_AVAILABLE -eq 0 ]; then
    TEST_TARGETS+=("x86_64-pc-windows-gnu")
    TEST_DESCRIPTIONS+=("Windows x86_64 GNU (mingw-w64)")
    TEST_TOOLS+=("cargo")
    TEST_RESULTS+=("SKIPPED - mingw-w64-gcc not available")
fi

# Cross-based tests (need cross + docker)
CROSS_READY=0
if [ $CROSS_AVAILABLE -eq 1 ] && [ $DOCKER_AVAILABLE -eq 1 ]; then
    CROSS_READY=1
fi

# macOS tests
if [ $TARGET_MAC_X86 -eq 1 ] && [ $CROSS_READY -eq 1 ]; then
    attempt_test "cross" "x86_64-apple-darwin" "macOS Intel" "$TEST_CMD_ARGS"
elif [ $TARGET_MAC_X86 -eq 0 ]; then
    TEST_TARGETS+=("x86_64-apple-darwin")
    TEST_DESCRIPTIONS+=("macOS Intel")
    TEST_TOOLS+=("cross")
    TEST_RESULTS+=("SKIPPED - target not installed")
elif [ $CROSS_READY -eq 0 ]; then
    TEST_TARGETS+=("x86_64-apple-darwin")
    TEST_DESCRIPTIONS+=("macOS Intel")
    TEST_TOOLS+=("cross")
    TEST_RESULTS+=("SKIPPED - cross/docker not available")
fi

if [ $TARGET_MAC_ARM -eq 1 ] && [ $CROSS_READY -eq 1 ]; then
    attempt_test "cross" "aarch64-apple-darwin" "macOS Apple Silicon" "$TEST_CMD_ARGS"
elif [ $TARGET_MAC_ARM -eq 0 ]; then
    TEST_TARGETS+=("aarch64-apple-darwin")
    TEST_DESCRIPTIONS+=("macOS Apple Silicon")
    TEST_TOOLS+=("cross")
    TEST_RESULTS+=("SKIPPED - target not installed")
elif [ $CROSS_READY -eq 0 ]; then
    TEST_TARGETS+=("aarch64-apple-darwin")
    TEST_DESCRIPTIONS+=("macOS Apple Silicon")
    TEST_TOOLS+=("cross")
    TEST_RESULTS+=("SKIPPED - cross/docker not available")
fi

# Display results table
echo
echo -e "${BLUE}=== Test Results Summary ===${NC}"
echo
printf "%-30s %-25s %-8s %-s\n" "Platform" "Target" "Tool" "Result"
printf "%-30s %-25s %-8s %-s\n" "--------" "------" "----" "------"

SUCCESS_COUNT=0
FAILED_COUNT=0
SKIPPED_COUNT=0

for i in "${!TEST_TARGETS[@]}"; do
    target="${TEST_TARGETS[i]}"
    description="${TEST_DESCRIPTIONS[i]}"
    result="${TEST_RESULTS[i]}"
    tool="${TEST_TOOLS[i]}"
    
    if [[ "$result" == "SUCCESS" ]]; then
        printf "%-30s %-25s %-8s ${GREEN}%-s${NC}\n" "$description" "$target" "$tool" "$result"
        ((SUCCESS_COUNT++))
    elif [[ "$result" == "FAILED" ]]; then
        printf "%-30s %-25s %-8s ${RED}%-s${NC}\n" "$description" "$target" "$tool" "$result"
        ((FAILED_COUNT++))
    else
        printf "%-30s %-25s %-8s ${YELLOW}%-s${NC}\n" "$description" "$target" "$tool" "$result"
        ((SKIPPED_COUNT++))
    fi
done

echo
echo -e "${BLUE}Summary: ${GREEN}$SUCCESS_COUNT successful${NC}, ${RED}$FAILED_COUNT failed${NC}, ${YELLOW}$SKIPPED_COUNT skipped${NC}"

# Show test artifacts if any exist
if [ $SUCCESS_COUNT -gt 0 ]; then
    echo
    echo -e "${BLUE}=== Test Artifacts ===${NC}"
    # Look for test reports or coverage files
    if [ -d "target" ]; then
        find target -name "*.profraw" -o -name "*.gcda" -o -name "test-results.xml" 2>/dev/null | sort | while read artifact; do
            if [ -f "$artifact" ]; then
                echo -e "${GREEN}✓${NC} $artifact"
            fi
        done
    fi
fi

echo
echo -e "${BLUE}Test script completed!${NC}"

# Exit with failure if any tests failed
if [ $FAILED_COUNT -gt 0 ]; then
    exit 1
fi