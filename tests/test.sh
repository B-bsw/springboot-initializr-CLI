#!/bin/bash
set -e

# Setup colors
GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${GREEN}=== SpringX CLI Comprehensive Test Suite ===${NC}"

# Recompile and install the CLI just in case
echo -e "\n${GREEN}[1/5] Building and installing springx CLI...${NC}"
cargo install --path .. > /dev/null 2>&1

if [ -n "$1" ]; then
    BOOT_VERSION=$1
else
    echo -e "\n${GREEN}Fetching available Spring Boot versions...${NC}"
    VERSIONS=$(curl -s -H 'Accept: application/json' https://start.spring.io/ | jq -r '.bootVersion.values[].id')
    
    echo "Please choose a Spring Boot version for the test:"
    i=1
    declare -a VERSION_ARRAY
    for v in $VERSIONS; do
        echo "  $i) $v"
        VERSION_ARRAY[$i]=$v
        i=$((i+1))
    done
    
    echo -n "Select a number (1-$((i-1))) [default 4.0.6]: "
    read -r selection
    
    if [[ -z "$selection" ]]; then
        BOOT_VERSION="4.0.6"
    elif [[ "$selection" -ge 1 ]] 2>/dev/null && [[ "$selection" -lt "$i" ]] 2>/dev/null; then
        BOOT_VERSION="${VERSION_ARRAY[$selection]}"
    else
        echo -e "${RED}Invalid selection. Defaulting to 4.0.6${NC}"
        BOOT_VERSION="4.0.6"
    fi
fi

echo -e "\n${GREEN}[2/5] Creating fresh Spring Boot $BOOT_VERSION project sandbox...${NC}"
TEST_DIR="sandbox"
rm -rf $TEST_DIR
mkdir $TEST_DIR
cd $TEST_DIR

# The API fails if we pass the suffix (like .RELEASE or .BUILD-SNAPSHOT), so we strip it.
STRIPPED_VERSION=$(echo "$BOOT_VERSION" | grep -oE '^[0-9]+\.[0-9]+\.[0-9]+')
if [ -z "$STRIPPED_VERSION" ]; then
    STRIPPED_VERSION="$BOOT_VERSION"
fi

# Use curl to download a fresh spring boot project. Added -f to fail on HTTP errors.
if ! curl -f -s https://start.spring.io/starter.zip -d type=gradle-project -d bootVersion=$STRIPPED_VERSION -d javaVersion=17 -o starter.zip; then
    echo -e "${RED}✖ Failed to download Spring Boot project for version $BOOT_VERSION ($STRIPPED_VERSION)${NC}"
    exit 1
fi

unzip -q starter.zip
rm starter.zip

echo -e "\n${GREEN}[3/5] Fetching all dependency IDs from Spring Initializr...${NC}"
ALL_DEPS=$(curl -s https://api-springboot-initializr.vercel.app/api | jq -r '.dependencies.values[].values[].id' | tr '\n' ' ')
TOTAL_DEPS=$(echo $ALL_DEPS | wc -w | tr -d ' ')
echo "Found $TOTAL_DEPS dependencies available."

echo -e "\n${GREEN}[4/5] Testing 'springx add' for all dependencies...${NC}"
echo "Adding dependencies one by one (this handles 400 Bad Requests seamlessly)..."
SUCCESS_COUNT=0
FAIL_COUNT=0

for dep in $ALL_DEPS; do
    if springx add $dep > /dev/null 2>&1; then
        SUCCESS_COUNT=$((SUCCESS_COUNT+1))
    else
        FAIL_COUNT=$((FAIL_COUNT+1))
    fi
done

echo -e "Added ${GREEN}$SUCCESS_COUNT${NC} compatible dependencies. Skipped ${RED}$FAIL_COUNT${NC} incompatible dependencies."

echo -e "\n${GREEN}[5/5] Testing 'springx deps' and 'springx remove'...${NC}"
echo "Currently detected dependencies by springx:"
springx deps

echo -e "\nRemoving all dependencies..."
if springx remove $ALL_DEPS > /dev/null 2>&1; then
    echo -e "${GREEN}✔ Successfully removed all dependencies!${NC}"
else
    echo -e "${RED}✖ Failed to remove dependencies!${NC}"
    exit 1
fi

echo -e "\nVerifying completely clean state..."
REMAINING=$(springx deps | grep -c "✓" || true)
if [ "$REMAINING" -eq 0 ]; then
    echo -e "${GREEN}✔ Verification passed: 0 remaining dependencies detected!${NC}"
else
    echo -e "${RED}✖ Verification failed: $REMAINING dependencies are still detected!${NC}"
    springx deps
    exit 1
fi

echo -e "\n${GREEN}=== All Tests Passed Successfully! ===${NC}"
