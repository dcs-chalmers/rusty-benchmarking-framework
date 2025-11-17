#!/bin/bash
# Run me from project root pls.
# Show usage if no arguments provided
if [ $# -eq 0 ]; then
    echo "Usage: $0 <package1,package2,...> <consumers-start> <consumers-end> <step>"
    exit 1
fi

# Get arguments
PACKAGES=$1
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
IFS=',' read -r -a PACKAGE_ARRAY <<< "$PACKAGES"

# Loop through each feature
for PACKAGE in "${PACKAGE_ARRAY[@]}"; do
    echo "Running tests for package: $PACKAGE"
    cargo build --release -p "$PACKAGE"
    # Loop through thread counts and run cargo command
    for ((i = CONSUMERS_START; i <= CONSUMERS_END; i += STEP)); do
        echo "Running with producer count: $i on feature $PACKAGE"
        time ./target/release/${PACKAGE} -t 1 -i 10 --path $OUTPUT/$PACKAGE basic -p 1 -c $i
    done
done

