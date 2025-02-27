#!/bin/bash
# Run me from project root pls.
# Show usage if no arguments provided
if [ $# -eq 0 ]; then
    echo "Usage: $0 <feature1,feature2,...> <thread-count-start> <thread-count-end> <thread-count-step>"
    echo "Example: $0 bounded_ringbuffer,unbounded_ringbuffer 5 100 5"
    exit 1
fi

# Get arguments
FEATURES=$1
START_THREADS=$2
END_THREADS=$3
THREAD_STEP=$4
OUTPUT=$5

# Validate numeric inputs
if ! [[ "$START_THREADS" =~ ^[0-9]+$ ]] || ! [[ "$END_THREADS" =~ ^[0-9]+$ ]] || ! [[ "$THREAD_STEP" =~ ^[0-9]+$ ]]; then
    echo "Error: Thread counts and step must be numeric values"
    exit 1
fi

# Split features into an array
IFS=',' read -r -a FEATURE_ARRAY <<< "$FEATURES"

# Loop through each feature
for FEATURE in "${FEATURE_ARRAY[@]}"; do
    echo "Running tests for feature: $FEATURE"
    
    # Loop through thread counts and run cargo command
    for ((i = START_THREADS; i <= END_THREADS; i += THREAD_STEP)); do
        echo "Running with thread count: $i"
        time cargo run --release --features "$FEATURE" -- -t 1 -i 10 --path $5 ping-pong --thread-count $i
    done
done
