#!/bin/bash

./clean.sh
cargo run --release --features memory_tracking,basic_queue -- -t 1 ping-pong
cargo run --release --features memory_tracking,basic_queue -- -t 1 basic 
cargo run --release --features memory_tracking,array_queue -- -t 1 ping-pong
cargo run --release --features memory_tracking,array_queue -- -t 1 basic 
cargo run --release --features memory_tracking,lockfree_queue -- -t 1 ping-pong
cargo run --release --features memory_tracking,lockfree_queue -- -t 1 basic 
cargo run --release --features memory_tracking,concurrent_queue -- -t 1 ping-pong
cargo run --release --features memory_tracking,concurrent_queue -- -t 1 basic 
cargo run --release --features memory_tracking,bounded_ringbuffer -- -t 1 ping-pong
cargo run --release --features memory_tracking,bounded_ringbuffer -- -t 1 basic 
bat output/*
