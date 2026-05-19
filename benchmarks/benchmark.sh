#!/bin/bash

# Kria Language Benchmark Suite
# Measures execution time of various benchmark tests

# Get the directory where this script is located
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# Change to project root
cd "$PROJECT_ROOT"

KRIA_BINARY="./target/release/kria"
BENCH_DIR="./benchmarks"
RESULTS_FILE="benchmark_results.txt"

# Color codes
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Check if binary exists
if [ ! -f "$KRIA_BINARY" ]; then
    echo -e "${RED}Error: $KRIA_BINARY not found${NC}"
    echo "Building release binary..."
    cargo build --release
    if [ ! -f "$KRIA_BINARY" ]; then
        echo -e "${RED}Failed to build Kria. Exiting.${NC}"
        exit 1
    fi
fi

# Check if benchmarks directory exists
if [ ! -d "$BENCH_DIR" ]; then
    echo -e "${RED}Error: $BENCH_DIR directory not found${NC}"
    exit 1
fi

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}    Kria Language Benchmark Suite${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""

# Initialize results
> "$RESULTS_FILE"
declare -A times
total_time=0

# Run benchmarks
bench_count=0
for bench_file in "$BENCH_DIR"/*.krx; do
    if [ -f "$bench_file" ]; then
        bench_count=$((bench_count + 1))
        bench_name=$(basename "$bench_file" .krx)
        
        echo -n "Running ${bench_name}... "
        
        # Measure execution time using /usr/bin/time
        start_time=$(date +%s%N)
        output=$("$KRIA_BINARY" "$bench_file" 2>&1)
        exit_code=$?
        end_time=$(date +%s%N)
        
        # Calculate elapsed time in milliseconds
        elapsed_ns=$((end_time - start_time))
        elapsed_ms=$(echo "scale=2; $elapsed_ns / 1000000" | bc)
        times[$bench_name]=$elapsed_ms
        total_time=$(echo "$total_time + $elapsed_ms" | bc)
        
        if [ $exit_code -eq 0 ]; then
            echo -e "${GREEN}✓${NC} ${elapsed_ms}ms (output: $output)"
            echo "$bench_name: ${elapsed_ms}ms - Output: $output" >> "$RESULTS_FILE"
        else
            echo -e "${RED}✗${NC} ${elapsed_ms}ms (error)"
            echo "$bench_name: ${elapsed_ms}ms - ERROR" >> "$RESULTS_FILE"
        fi
    fi
done

echo ""
echo -e "${BLUE}════════════════════════════════════════${NC}"
echo -e "${BLUE}         Kria Benchmark Results${NC}"
echo -e "${BLUE}════════════════════════════════════════${NC}"
echo ""

# Print results table
printf "%-35s %12s\n" "Test Name" "Time (ms)"
printf "%-35s %12s\n" "─────────────────────────────────" "──────────"

for bench_name in "${!times[@]}"; do
    printf "%-35s %12s\n" "$bench_name" "${times[$bench_name]} ms"
done | sort

echo ""
printf "%-35s %12s\n" "─────────────────────────────────" "──────────"
printf "%-35s %12s\n" "Total Execution Time" "${total_time} ms"
printf "%-35s %12s\n" "Number of Tests" "$bench_count"

if [ $bench_count -gt 0 ]; then
    average=$(echo "scale=2; $total_time / $bench_count" | bc)
    printf "%-35s %12s\n" "Average Time per Test" "${average} ms"
fi

echo ""
echo -e "${BLUE}════════════════════════════════════════${NC}"
echo ""
echo -e "${GREEN}✓ Results saved to: $RESULTS_FILE${NC}"
