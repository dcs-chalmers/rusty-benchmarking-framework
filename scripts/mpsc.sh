#!/bin/bash
# Run me from project root pls.
# Show usage if no arguments provided
if [ $# -eq 0 ]; then
    echo "Usage: $0 <feature1,feature2,...> <producers-start> <producers-end> <step> <path>"
    exit 1
fi

# Get arguments
FEATURES=$1
PRODUCERS_START=$2
PRODUCERS_END=$3
STEP=$4
OUTPUT=$5

# Validate numeric inputs
if ! [[ "$PRODUCERS_START" =~ ^[0-9]+$ ]] || ! [[ "$PRODUCERS_END" =~ ^[0-9]+$ ]] || ! [[ "$STEP" =~ ^[0-9]+$ ]]; then
    echo "Error: Thread counts and step must be numeric values"
    exit 1
fi

# Split features into an array
IFS=',' read -r -a FEATURE_ARRAY <<< "$FEATURES"

# Loop through each feature
for FEATURE in "${FEATURE_ARRAY[@]}"; do
    echo "Running tests for feature: $FEATURE"
    
    # Loop through thread counts and run cargo command
    for ((i = PRODUCERS_START; i <= PRODUCERS_END; i += STEP)); do
        echo "Running with producer count: $i"
        time cargo run --release --features "$FEATURE" -- -t 1 -i 10 --path $OUTPUT basic -p $i -c 1
    done
done

