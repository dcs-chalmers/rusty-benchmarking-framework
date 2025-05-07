#!/bin/bash
# Run me from project root pls.
# Show usage if no arguments provided
if [ $# -eq 0 ]; then
    echo "Usage: $0 <feature1,feature2,...> <thread-count-start> <thread-count-end> <thread-count-step> [output-dir] [spread]"
    echo "Example: $0 bounded_ringbuffer,unbounded_ringbuffer 5 100 5"
    echo "Default output directory: ./enqdeq_START_THREADS_END_THREADS_THREAD_STEP"
    echo "Default spread: 0.5"
    exit 1
fi

# Get arguments
FEATURES=$1
START_THREADS=$2
END_THREADS=$3
THREAD_STEP=$4


if [ -z "$6" ]; then
    SPREAD=0.5
    echo "No spread provided. Running with 0.5 as spread."
else
    SPREAD=$6
fi
# Set default values for OUTPUT and SPREAD if not provided
if [ -z "$5" ]; then
    OUTPUT="./enqdeq_${START_THREADS}_${END_THREADS}_${THREAD_STEP}_S${SPREAD}"
    echo "Saving output files to $OUTPUT"
else
    OUTPUT=$5
fi

# Create output directory if it doesn't exist
mkdir -p $OUTPUT

# Validate numeric inputs
if ! [[ "$START_THREADS" =~ ^[0-9]+$ ]] || ! [[ "$END_THREADS" =~ ^[0-9]+$ ]] || ! [[ "$THREAD_STEP" =~ ^[0-9]+$ ]]; then
    echo "Error: Thread counts and step must be numeric values"
    exit 1
fi

# Split features into an array
IFS=',' read -r -a FEATURE_ARRAY <<< "$FEATURES"

echo -e "Benchmark config:\n\tOutput folder: $OUTPUT\n\tThreads: $START_THREADS-$END_THREADS\n\tSteps: $THREAD_STEP\n\tSpread: $SPREAD"

# Loop through each feature
for FEATURE in "${FEATURE_ARRAY[@]}"; do
    echo "Running tests for feature: $FEATURE"
    cargo build --release --features "$FEATURE"    
    
    # Loop through thread counts and run cargo command
    for ((i = START_THREADS; i <= END_THREADS; i += THREAD_STEP)); do
        echo "Running $FEATURE with thread count: $i"
        time ./target/release/lockfree-benchmark -t 1 -i 10 --path $OUTPUT/$FEATURE enq-deq --thread-count $i --spread $SPREAD
    done
done
