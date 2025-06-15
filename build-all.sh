#!/bin/bash

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Parse arguments
BUILD_TYPE="debug"
VERBOSE=0

while [[ $# -gt 0 ]]; do
    case $1 in
        release)
            BUILD_TYPE="release"
            shift
            ;;
        -v|--verbose)
            VERBOSE=1
            shift
            ;;
        -h|--help)
            echo "Usage: $0 [release] [-v|--verbose] [-h|--help]"
            echo "  release     Build in release mode (default: debug)"
            echo "  -v,--verbose Show build output"
            echo "  -h,--help   Show this help"
            exit 0
            ;;
        *)
            echo "Unknown argument: $1"
            echo "Use -h for help"
            exit 1
            ;;
    esac
done

BUILD_FLAG=""
if [ "$BUILD_TYPE" = "release" ]; then
    BUILD_FLAG="--release"
fi

# Arrays to track build results
declare -a BUILD_TARGETS
declare -a BUILD_DESCRIPTIONS
declare -a BUILD_RESULTS
declare -a BUILD_TOOLS

echo -e "${BLUE}=== Cross-Platform Build Script ===${NC}"
echo -e "${BLUE}Build type: $BUILD_TYPE${NC}"
if [ $VERBOSE -eq 1 ]; then
    echo -e "${BLUE}Verbose mode: enabled${NC}"
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

# Function to attempt build and record result
attempt_build() {
    local tool=$1
    local target=$2
    local description=$3
    
    BUILD_TARGETS+=("$target")
    BUILD_DESCRIPTIONS+=("$description")
    BUILD_TOOLS+=("$tool")
    
    echo -e "${YELLOW}Building $description...${NC}"
    
    # Determine output redirection based on verbose flag
    local output_redirect=""
    if [ $VERBOSE -eq 0 ]; then
        output_redirect=">/dev/null 2>&1"
    fi
    
    if [ "$tool" = "cargo" ]; then
        echo -e "${BLUE}Running: cargo build --target $target $BUILD_FLAG${NC}"
        if [ $VERBOSE -eq 1 ]; then
            if cargo build --target "$target" $BUILD_FLAG; then
                BUILD_RESULTS+=("SUCCESS")
                echo -e "${GREEN}✓${NC} Successfully built $description"
            else
                BUILD_RESULTS+=("FAILED")
                echo -e "${RED}✗${NC} Failed to build $description"
            fi
        else
            if cargo build --target "$target" $BUILD_FLAG >/dev/null 2>&1; then
                BUILD_RESULTS+=("SUCCESS")
                echo -e "${GREEN}✓${NC} Successfully built $description"
            else
                BUILD_RESULTS+=("FAILED")
                echo -e "${RED}✗${NC} Failed to build $description"
            fi
        fi
    elif [ "$tool" = "cross" ]; then
        echo -e "${BLUE}Running: cross build --target $target $BUILD_FLAG${NC}"
        if [ $VERBOSE -eq 1 ]; then
            if cross build --target "$target" $BUILD_FLAG; then
                BUILD_RESULTS+=("SUCCESS")
                echo -e "${GREEN}✓${NC} Successfully built $description"
            else
                BUILD_RESULTS+=("FAILED")
                echo -e "${RED}✗${NC} Failed to build $description"
            fi
        else
            if cross build --target "$target" $BUILD_FLAG >/dev/null 2>&1; then
                BUILD_RESULTS+=("SUCCESS")
                echo -e "${GREEN}✓${NC} Successfully built $description"
            else
                BUILD_RESULTS+=("FAILED")
                echo -e "${RED}✗${NC} Failed to build $description"
            fi
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

# Check optional tools (don't fail if missing, just skip those builds)
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

echo -e "${GREEN}Starting builds with available tools...${NC}"
echo

# Start building
echo -e "${BLUE}=== Cross-Platform Builds ===${NC}"

# Native build (always available)
if [ $TARGET_X86_LINUX -eq 1 ]; then
    attempt_build "cargo" "x86_64-unknown-linux-gnu" "Linux x86_64 (native)"
else
    BUILD_TARGETS+=("x86_64-unknown-linux-gnu")
    BUILD_DESCRIPTIONS+=("Linux x86_64 (native)")
    BUILD_TOOLS+=("cargo")
    BUILD_RESULTS+=("SKIPPED - target not installed")
fi

# Linux ARM64 build
if [ $TARGET_ARM_LINUX -eq 1 ] && [ $LINUX_ARM_AVAILABLE -eq 1 ]; then
    attempt_build "cargo" "aarch64-unknown-linux-gnu" "Linux ARM64 (cross-compile)"
elif [ $TARGET_ARM_LINUX -eq 0 ]; then
    BUILD_TARGETS+=("aarch64-unknown-linux-gnu")
    BUILD_DESCRIPTIONS+=("Linux ARM64 (cross-compile)")
    BUILD_TOOLS+=("cargo")
    BUILD_RESULTS+=("SKIPPED - target not installed")
elif [ $LINUX_ARM_AVAILABLE -eq 0 ]; then
    BUILD_TARGETS+=("aarch64-unknown-linux-gnu")
    BUILD_DESCRIPTIONS+=("Linux ARM64 (cross-compile)")
    BUILD_TOOLS+=("cargo")
    BUILD_RESULTS+=("SKIPPED - aarch64-linux-gnu-gcc not available")
fi

# Windows GNU build
if [ $TARGET_WIN_GNU -eq 1 ] && [ $MINGW_AVAILABLE -eq 1 ]; then
    attempt_build "cargo" "x86_64-pc-windows-gnu" "Windows x86_64 GNU (mingw-w64)"
elif [ $TARGET_WIN_GNU -eq 0 ]; then
    BUILD_TARGETS+=("x86_64-pc-windows-gnu")
    BUILD_DESCRIPTIONS+=("Windows x86_64 GNU (mingw-w64)")
    BUILD_TOOLS+=("cargo")
    BUILD_RESULTS+=("SKIPPED - target not installed")
elif [ $MINGW_AVAILABLE -eq 0 ]; then
    BUILD_TARGETS+=("x86_64-pc-windows-gnu")
    BUILD_DESCRIPTIONS+=("Windows x86_64 GNU (mingw-w64)")
    BUILD_TOOLS+=("cargo")
    BUILD_RESULTS+=("SKIPPED - mingw-w64-gcc not available")
fi

# Cross-based builds (need cross + docker)
CROSS_READY=0
if [ $CROSS_AVAILABLE -eq 1 ] && [ $DOCKER_AVAILABLE -eq 1 ]; then
    CROSS_READY=1
fi


# macOS builds
if [ $TARGET_MAC_X86 -eq 1 ] && [ $CROSS_READY -eq 1 ]; then
    attempt_build "cross" "x86_64-apple-darwin" "macOS Intel"
elif [ $TARGET_MAC_X86 -eq 0 ]; then
    BUILD_TARGETS+=("x86_64-apple-darwin")
    BUILD_DESCRIPTIONS+=("macOS Intel")
    BUILD_TOOLS+=("cross")
    BUILD_RESULTS+=("SKIPPED - target not installed")
elif [ $CROSS_READY -eq 0 ]; then
    BUILD_TARGETS+=("x86_64-apple-darwin")
    BUILD_DESCRIPTIONS+=("macOS Intel")
    BUILD_TOOLS+=("cross")
    BUILD_RESULTS+=("SKIPPED - cross/docker not available")
fi

if [ $TARGET_MAC_ARM -eq 1 ] && [ $CROSS_READY -eq 1 ]; then
    attempt_build "cross" "aarch64-apple-darwin" "macOS Apple Silicon"
elif [ $TARGET_MAC_ARM -eq 0 ]; then
    BUILD_TARGETS+=("aarch64-apple-darwin")
    BUILD_DESCRIPTIONS+=("macOS Apple Silicon")
    BUILD_TOOLS+=("cross")
    BUILD_RESULTS+=("SKIPPED - target not installed")
elif [ $CROSS_READY -eq 0 ]; then
    BUILD_TARGETS+=("aarch64-apple-darwin")
    BUILD_DESCRIPTIONS+=("macOS Apple Silicon")
    BUILD_TOOLS+=("cross")
    BUILD_RESULTS+=("SKIPPED - cross/docker not available")
fi

# Display results table
echo
echo -e "${BLUE}=== Build Results Summary ===${NC}"
echo
printf "%-30s %-25s %-8s %-s\n" "Platform" "Target" "Tool" "Result"
printf "%-30s %-25s %-8s %-s\n" "--------" "------" "----" "------"

SUCCESS_COUNT=0
FAILED_COUNT=0
SKIPPED_COUNT=0

for i in "${!BUILD_TARGETS[@]}"; do
    target="${BUILD_TARGETS[i]}"
    description="${BUILD_DESCRIPTIONS[i]}"
    result="${BUILD_RESULTS[i]}"
    tool="${BUILD_TOOLS[i]}"
    
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

# Show successful binaries
if [ $SUCCESS_COUNT -gt 0 ]; then
    echo
    echo -e "${BLUE}=== Successfully Built Binaries ===${NC}"
    find target -name "ci" -o -name "ci.exe" 2>/dev/null | sort | while read binary; do
        if [ -f "$binary" ]; then
            size=$(du -h "$binary" 2>/dev/null | cut -f1)
            echo -e "${GREEN}✓${NC} $binary ($size)"
        fi
    done
fi

echo
echo -e "${BLUE}Build script completed!${NC}"