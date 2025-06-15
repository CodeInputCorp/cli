#!/bin/bash

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Script to update README.md with latest release information
# Similar to TOC generation, this uses markers to define update sections

README_FILE="README.md"
TEMP_FILE="README.tmp"

# Function to check if we're in a git repository
check_git_repo() {
    if ! git rev-parse --git-dir >/dev/null 2>&1; then
        echo -e "${RED}Error: Not in a git repository${NC}"
        exit 1
    fi
}

# Function to get latest release info from GitHub API
get_latest_release() {
    local repo_url=$(git config --get remote.origin.url)
    local repo_path=""

    # Extract owner/repo from different URL formats
    if [[ $repo_url == *"github.com"* ]]; then
        if [[ $repo_url == git@* ]]; then
            # SSH format: git@github.com:user/repo.git
            repo_path=$(echo "$repo_url" | sed 's/git@github.com://' | sed 's/\.git$//')
        else
            # HTTPS format: https://github.com/user/repo.git
            repo_path=$(echo "$repo_url" | sed 's|.*github.com/||' | sed 's/\.git$//')
        fi
    else
        echo -e "${RED}Error: Not a GitHub repository${NC}"
        exit 1
    fi

    echo "$repo_path"
}

# Function to fetch release data from GitHub API
fetch_release_data() {
    local repo_path="$1"
    local api_url="https://api.github.com/repos/${repo_path}/releases/latest"

    echo -e "${BLUE}Fetching latest release from: $api_url${NC}"

    # Use curl to fetch release data
    local response=$(curl -s "$api_url")

    # Check if we got a valid response
    if echo "$response" | grep -q '"message": "Not Found"'; then
        echo -e "${YELLOW}Warning: No releases found or repository not public${NC}"
        return 1
    fi

    echo "$response"
}

# Function to extract download URLs for different platforms
extract_download_urls() {
    local release_data="$1"
    local version=$(echo "$release_data" | grep '"tag_name"' | head -1 | cut -d'"' -f4)
    local release_notes=$(echo "$release_data" | grep '"body"' | head -1 | cut -d'"' -f4 | sed 's/\\n/\n/g' | sed 's/\\r//g')

    # Extract asset download URLs
    local assets=$(echo "$release_data" | grep '"browser_download_url"' | cut -d'"' -f4)

    # Generate markdown for different platforms
    local download_section=""
    download_section+="### Pre-built Binaries\n\n"
    download_section+="**Latest Release: \`$version\`**\n\n"

    # Platform-specific downloads
    if echo "$assets" | grep -q "linux.*x86_64"; then
        local linux_x64_url=$(echo "$assets" | grep "linux.*x86_64" | head -1)
        download_section+="- **Linux x86_64**: [Download]($linux_x64_url)\n"
    fi

    if echo "$assets" | grep -q "linux.*aarch64"; then
        local linux_arm64_url=$(echo "$assets" | grep "linux.*aarch64" | head -1)
        download_section+="- **Linux ARM64**: [Download]($linux_arm64_url)\n"
    fi

    if echo "$assets" | grep -q "windows.*x86_64"; then
        local windows_x64_url=$(echo "$assets" | grep "windows.*x86_64" | head -1)
        download_section+="- **Windows x86_64**: [Download]($windows_x64_url)\n"
    fi

    if echo "$assets" | grep -q "macos.*x86_64"; then
        local macos_x64_url=$(echo "$assets" | grep "macos.*x86_64" | head -1)
        download_section+="- **macOS Intel**: [Download]($macos_x64_url)\n"
    fi

    if echo "$assets" | grep -q "macos.*aarch64"; then
        local macos_arm64_url=$(echo "$assets" | grep "macos.*aarch64" | head -1)
        download_section+="- **macOS Apple Silicon**: [Download]($macos_arm64_url)\n"
    fi

    download_section+="\n#### Installation Instructions\n\n"
    download_section+="1. Download the appropriate binary for your platform\n"
    download_section+="2. Rename the downloaded file to \`ci\` (Linux/macOS) or \`ci.exe\` (Windows)\n"
    download_section+="3. Move the binary to your PATH: \`mv ci /usr/local/bin/\` (Linux/macOS)\n"
    download_section+="4. Make it executable: \`chmod +x /usr/local/bin/ci\` (Linux/macOS)\n\n"

    # Add what's new section if release notes exist
    if [[ -n "$release_notes" && "$release_notes" != "null" ]]; then
        download_section+="#### What's New in $version\n\n"
        download_section+="$release_notes\n\n"
    fi

    echo -e "$download_section"
}

# Function to update README between markers
update_readme() {
    local new_content="$1"
    local start_marker="<!-- RELEASE_INFO_START -->"
    local end_marker="<!-- RELEASE_INFO_END -->"

    if [ ! -f "$README_FILE" ]; then
        echo -e "${RED}Error: $README_FILE not found${NC}"
        exit 1
    fi

    # Check if markers exist
    if ! grep -q "$start_marker" "$README_FILE" || ! grep -q "$end_marker" "$README_FILE"; then
        echo -e "${YELLOW}Warning: Release info markers not found in $README_FILE${NC}"
        echo -e "${YELLOW}Please add the following markers where you want the release info:${NC}"
        echo -e "${BLUE}$start_marker${NC}"
        echo -e "${BLUE}$end_marker${NC}"
        exit 1
    fi

    # Create temp file with updated content
    awk -v start="$start_marker" -v end="$end_marker" -v content="$new_content" '
        BEGIN { in_section = 0 }
        $0 ~ start { 
            print $0
            print content
            in_section = 1
            next
        }
        $0 ~ end { 
            print $0
            in_section = 0
            next
        }
        !in_section { print $0 }
    ' "$README_FILE" >"$TEMP_FILE"

    # Replace original file
    mv "$TEMP_FILE" "$README_FILE"

    echo -e "${GREEN}✓ README.md updated successfully${NC}"
}

# Main execution
main() {
    echo -e "${BLUE}=== README Release Info Updater ===${NC}"

    # Check if we're in a git repo
    check_git_repo

    # Get repository information
    local repo_path=$(get_latest_release)
    echo -e "${BLUE}Repository: $repo_path${NC}"

    # Fetch release data
    local release_data=$(fetch_release_data "$repo_path")
    if [[ $? -ne 0 ]]; then
        exit 1
    fi

    # Extract and format download information
    local download_content=$(extract_download_urls "$release_data")

    # Update README
    update_readme "$download_content"

    echo -e "${GREEN}Release info update completed!${NC}"
}

# Parse command line arguments
case "${1:-}" in
-h | --help)
    echo "Usage: $0 [--help]"
    echo "Updates README.md with latest release information from GitHub"
    echo ""
    echo "The script looks for these markers in README.md:"
    echo "  <!-- RELEASE_INFO_START -->"
    echo "  <!-- RELEASE_INFO_END -->"
    echo ""
    echo "All content between these markers will be replaced with"
    echo "automatically generated release information."
    exit 0
    ;;
*)
    main "$@"
    ;;
esac
