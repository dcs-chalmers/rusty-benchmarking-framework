#!/bin/bash
# Run me from project root pls.
# Show usage if no arguments provided
if [ $# -eq 0 ]; then
    echo "Usage: $0 <feature1,feature2,...> <consumers-start> <consumers-end> <step>"
    exit 1
fi

# Get arguments
FEATURES=$1
CONSUMERS_START=$2
CONSUMERS_END=$3
STEP=$4
OUTPUT=$5

mkdir -p $OUTPUT

# Validate numeric inputs
if ! [[ "$CONSUMERS_START" =~ ^[0-9]+$ ]] || ! [[ "$CONSUMERS_END" =~ ^[0-9]+$ ]] || ! [[ "$STEP" =~ ^[0-9]+$ ]]; then
    echo "Error: Thread counts and step must be numeric values"
    exit 1
fi

# Split features into an array
IFS=',' read -r -a FEATURE_ARRAY <<< "$FEATURES"

# Loop through each feature
for FEATURE in "${FEATURE_ARRAY[@]}"; do
    echo "Running tests for feature: $FEATURE"
    
    # Loop through thread counts and run cargo command
    for ((i = CONSUMERS_START; i <= CONSUMERS_END; i += STEP)); do
        echo "Running with producer count: $i on feature $FEATURE"
        time cargo run --release --features "$FEATURE" -- -t 1 -i 10 --path $OUTPUT/$FEATURE basic -p 1 -c $i
    done
done

