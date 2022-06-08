#!/usr/bin/env bash

set -e # Stop on first error

if [ "$RELEASE" == "1" ]; then
    filename="target/release/tokio_demo"
else
    filename="target/x86_64-unknown-linux-gnu/debug/tokio_demo"
fi

until [ -f $filename ]
do
    sleep 1
done
if [ "$RELEASE" == "1" ]; then
    target/release/tokio_demo $TOTAL_NODES $(hostname) &
    server_pid=$!
else
    TSAN_OPTIONS="verbosity=2 detect_deadlocks=1 suppressions=sanitizer-thread-suppressions.txt" RUST_BACKTRACE=1 \
        target/x86_64-unknown-linux-gnu/debug/tokio_demo $TOTAL_NODES $(hostname) &
    server_pid=$!
fi

# Wait for this node to become available...
until grpcurl -plaintext localhost:2323 sample.Sample/CheckHealth &> /dev/null
do
    echo "Waiting for the node to start..."
    sleep 1
done

until [ $(wc -l < all_peers_added.debug.txt) == $TOTAL_NODES ]
do
    sleep 1
done

until grpcurl -plaintext localhost:2323 sample.Sample/InitialDealing > /dev/null
do
    sleep 1
done

wait $server_pid
