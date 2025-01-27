#!/bin/bash

mkdir -p output

if [ $# -eq 0 ]; then 
    cargo run --release
else
    cargo run --release -- $1
fi
