#!/bin/bash

./clean.sh
RUST_LOG=debug cargo run --release --features memory_tracking,basic_queue -- -t 1 ping-pong
RUST_LOG=debug cargo run --release --features memory_tracking,basic_queue -- -t 1 basic 
RUST_LOG=debug cargo run --release --features memory_tracking,array_queue -- -t 1 ping-pong
RUST_LOG=debug cargo run --release --features memory_tracking,array_queue -- -t 1 basic 
RUST_LOG=debug cargo run --release --features memory_tracking,lockfree_queue -- -t 1 ping-pong
RUST_LOG=debug cargo run --release --features memory_tracking,lockfree_queue -- -t 1 basic 
RUST_LOG=debug cargo run --release --features memory_tracking,concurrent_queue -- -t 1 ping-pong
RUST_LOG=debug cargo run --release --features memory_tracking,concurrent_queue -- -t 1 basic 
RUST_LOG=debug cargo run --release --features memory_tracking,bounded_ringbuffer -- -t 1 ping-pong
RUST_LOG=debug cargo run --release --features memory_tracking,bounded_ringbuffer -- -t 1 basic 
RUST_LOG=debug cargo run --release --features memory_tracking,atomic_queue -- -t 1 ping-pong
RUST_LOG=debug cargo run --release --features memory_tracking,atomic_queue -- -t 1 basic 
RUST_LOG=debug cargo run --release --features memory_tracking,wfqueue -- -t 1 ping-pong
RUST_LOG=debug cargo run --release --features memory_tracking,wfqueue -- -t 1 basic 
RUST_LOG=debug cargo run --release --features memory_tracking,scc_queue -- -t 1 ping-pong
RUST_LOG=debug cargo run --release --features memory_tracking,scc_queue -- -t 1 basic 
RUST_LOG=debug cargo run --release --features memory_tracking,scc2_queue -- -t 1 ping-pong
RUST_LOG=debug cargo run --release --features memory_tracking,scc2_queue -- -t 1 basic 
bat output/*
