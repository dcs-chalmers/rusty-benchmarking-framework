#!/bin/bash

mkdir -p output

if [ $# -eq 0 ]; then 
    time cargo run --release
else
    time cargo run --release -- $1
fi
