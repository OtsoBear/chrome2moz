#!/bin/bash
set -euo pipefail  # Exit on error, undefined vars, pipe failures

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

# Fetch API stats from cargo command
log_info "Fetching Chrome-only API statistics..."
OUTPUT=$(cargo run --release chrome-only-apis 2>&1) || {
    log_error "Failed to run cargo command"
    exit 1
}

echo "$OUTPUT"

# Extract stats using more robust parsing
TOTAL=$(echo "$OUTPUT" | grep -oP "Total Chrome-only APIs.*?: \K\d+" || echo "")
IMPLEMENTED=$(echo "$OUTPUT" | grep -oP "Implemented.*?: \K\d+" || echo "")
NOT_IMPLEMENTED=$(echo "$OUTPUT" | grep -oP "Not yet implemented.*?: \K\d+" || echo "")
PERCENTAGE=$(echo "$OUTPUT" | grep -oP "Implementation coverage.*?: \K\d+" || echo "")

# Validate extracted values
if [[ -z "$TOTAL" ]] || [[ -z "$IMPLEMENTED" ]] || [[ -z "$NOT_IMPLEMENTED" ]]; then
    log_error "Failed to extract statistics from output"
    log_error "TOTAL=$TOTAL, IMPLEMENTED=$IMPLEMENTED, NOT_IMPLEMENTED=$NOT_IMPLEMENTED"
    exit 1
fi

# Calculate percentage if not found
if [[ -z "$PERCENTAGE" ]] && [[ "$TOTAL" -gt 0 ]]; then
    PERCENTAGE=$(( (IMPLEMENTED * 100) / TOTAL ))
    log_warning "Calculated percentage: ${PERCENTAGE}%"
elif [[ -z "$PERCENTAGE" ]]; then
    PERCENTAGE=0
fi

NOT_IMPL_PERCENTAGE=$(( 100 - PERCENTAGE ))

log_info "Statistics extracted:"
log_info "  Total: $TOTAL"
log_info "  Implemented: $IMPLEMENTED ($PERCENTAGE%)"
log_info "  Not Implemented: $NOT_IMPLEMENTED ($NOT_IMPL_PERCENTAGE%)"

# Update README.md
log_info "Updating README.md..."
if [[ ! -f "README.md" ]]; then
    log_error "README.md not found"
    exit 1
fi

# Use perl for more reliable in-place editing (works on both Linux and macOS)
perl -i -pe "s|https://progress-bar\.xyz/\d+/\?scale=100&title=API%20Coverage|https://progress-bar.xyz/${PERCENTAGE}/?scale=100&title=API%20Coverage|g" README.md
perl -i -pe "s|\*\*\d+ of \d+ Chrome-only APIs\*\*|**${IMPLEMENTED} of ${TOTAL} Chrome-only APIs**|g" README.md
perl -i -pe "s|\*\*Total Tracked\*\* \| \d+|**Total Tracked** | ${TOTAL}|g" README.md
perl -i -pe "s|\*\*Implemented\*\* \| \d+ \(\d+%\)|**Implemented** | ${IMPLEMENTED} (${PERCENTAGE}%)|g" README.md
perl -i -pe "s|\*\*Not Implemented\*\* \| \d+ \(\d+%\)|**Not Implemented** | ${NOT_IMPLEMENTED} (${NOT_IMPL_PERCENTAGE}%)|g" README.md

# Update CHROME_ONLY_API_IMPLEMENTATION_STATUS.md
log_info "Updating CHROME_ONLY_API_IMPLEMENTATION_STATUS.md..."
if [[ ! -f "CHROME_ONLY_API_IMPLEMENTATION_STATUS.md" ]]; then
    log_error "CHROME_ONLY_API_IMPLEMENTATION_STATUS.md not found"
    exit 1
fi

perl -i -pe "s|> \*\*Summary\*\*: \d+ Chrome-only APIs detected \| \d+ Implemented \(\d+%\) \| \d+ Not Implemented \(\d+%\)|> **Summary**: ${TOTAL} Chrome-only APIs detected | ${IMPLEMENTED} Implemented (${PERCENTAGE}%) | ${NOT_IMPLEMENTED} Not Implemented (${NOT_IMPL_PERCENTAGE}%)|g" CHROME_ONLY_API_IMPLEMENTATION_STATUS.md
perl -i -pe "s|\*\*Total Chrome-Only APIs\*\* \| \d+ \| 100%|**Total Chrome-Only APIs** | ${TOTAL} | 100%|g" CHROME_ONLY_API_IMPLEMENTATION_STATUS.md
perl -i -pe "s|\*\*Implemented\*\* \| \d+ \| \d+%|**Implemented** | ${IMPLEMENTED} | ${PERCENTAGE}%|g" CHROME_ONLY_API_IMPLEMENTATION_STATUS.md
perl -i -pe "s|\*\*Not Implemented\*\* \| \d+ \| \d+%|**Not Implemented** | ${NOT_IMPLEMENTED} | ${NOT_IMPL_PERCENTAGE}%|g" CHROME_ONLY_API_IMPLEMENTATION_STATUS.md

log_info "Files updated successfully"

# Export variables for GitHub Actions (if running in CI)
if [[ -n "${GITHUB_OUTPUT:-}" ]]; then
    {
        echo "total=$TOTAL"
        echo "implemented=$IMPLEMENTED"
        echo "not_implemented=$NOT_IMPLEMENTED"
        echo "percentage=$PERCENTAGE"
    } >> "$GITHUB_OUTPUT"
fi

exit 0