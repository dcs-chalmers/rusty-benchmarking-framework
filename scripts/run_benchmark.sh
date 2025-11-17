#!/bin/bash

./clean.sh
RUST_LOG=debug cargo run --release --features memory_tracking -p basic_queue -- -t 1 ping-pong
RUST_LOG=debug cargo run --release --features memory_tracking -p basic_queue -- -t 1 basic
RUST_LOG=debug cargo run --release --features memory_tracking -p array_queue -- -t 1 ping-pong
RUST_LOG=debug cargo run --release --features memory_tracking -p array_queue -- -t 1 basic
RUST_LOG=debug cargo run --release --features memory_tracking -p lockfree_queue -- -t 1 ping-pong
RUST_LOG=debug cargo run --release --features memory_tracking -p lockfree_queue -- -t 1 basic
RUST_LOG=debug cargo run --release --features memory_tracking -p unbounded_concurrent_queue -- -t 1 ping-pong
RUST_LOG=debug cargo run --release --features memory_tracking -p unbounded_concurrent_queue -- -t 1 basic
RUST_LOG=debug cargo run --release --features memory_tracking -p bounded_ringbuffer -- -t 1 ping-pong
RUST_LOG=debug cargo run --release --features memory_tracking -p bounded_ringbuffer -- -t 1 basic
RUST_LOG=debug cargo run --release --features memory_tracking -p atomic_queue -- -t 1 ping-pong
RUST_LOG=debug cargo run --release --features memory_tracking -p atomic_queue -- -t 1 basic
RUST_LOG=debug cargo run --release --features memory_tracking -p wf_queue -- -t 1 ping-pong
RUST_LOG=debug cargo run --release --features memory_tracking -p wf_queue -- -t 1 basic
RUST_LOG=debug cargo run --release --features memory_tracking -p scc_queue -- -t 1 ping-pong
RUST_LOG=debug cargo run --release --features memory_tracking -p scc_queue -- -t 1 basic
RUST_LOG=debug cargo run --release --features memory_tracking -p scc2_queue -- -t 1 ping-pong
RUST_LOG=debug cargo run --release --features memory_tracking -p scc2_queue -- -t 1 basic
bat output/*
