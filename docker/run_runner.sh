#!/usr/bin/env bash

# The runner is responsible for building the node binary, orchestrating the nodes, 
# sending the nodes commands, and all tasks that should be conducted only
# once

set -e # Stop on first error

rm -f *.debug.txt
touch all_peers_added.debug.txt
touch servers_running.debug.txt
touch added_peer.debug.txt
touch opening_complete.debug.txt
touch spawned_all_dealing_requests.debug.txt
touch inbound_dealing_received.debug.txt
touch dealing_created.debug.txt
touch dealing_sent.debug.txt
touch client_sent.debug.txt
touch server_sent.debug.txt
touch client_received.debug.txt
touch server_received.debug.txt

# Build the node binary
echo "Debug build..."
cargo build --target x86_64-unknown-linux-gnu

# Wait for all the servers to be listening
until [ $(wc -l < servers_running.debug.txt) == $TOTAL_NODES ]
do
    sleep 1
done
sleep 1 # Make sure the servers are actually listening, it saves to this file before the server listen call
echo "All nodes have started, adding peers..."

# Tell every node what its ID is and to add other nodes as peers
for (( n=1; n<=$TOTAL_NODES; n++ ))
do
    grpcurl -d "{\"node_index\": \"$n\"}" -plaintext "tokio-sample-node-$n:2323" sample.Sample/IteratePeers > /dev/null &
    sleep 0.1
    #echo "Asked node $n to add its peers."
done

# Add peers
#let expected_connections="$TOTAL_NODES * ($TOTAL_NODES - 1) / 2"
let expected_connections="$TOTAL_NODES * $TOTAL_NODES"
until [ $(wc -l < all_peers_added.debug.txt) == $TOTAL_NODES ]
do
    current_peer_connections=$(wc -l < added_peer.debug.txt)
    echo "Have $current_peer_connections / $expected_connections peer connections..."
    sleep 1
done
number_connections=$(wc -l < added_peer.debug.txt)
if [ $number_connections != $expected_connections ]; then
    echo "Got incorrect number of connections $number_connections, expected $expected_connections"
    exit 1
else
    echo "All nodes successfully finished adding peers."
fi

until [ $(wc -l < spawned_all_dealing_requests.debug.txt) == $TOTAL_NODES ]
do
    sleep 1
done
echo "All nodes finished spawning signature requests."

let expected_openings="3 * $TOTAL_NODES"
let expected_sent="3 * $TOTAL_NODES * ($TOTAL_NODES - 1)"
let expected_on_one_side="$expected_sent / 2"
print_dealings_stats () {
    current_openings_complete=$(wc -l < opening_complete.debug.txt)
    current_dealings_created=$(wc -l < dealing_created.debug.txt)
    current_dealings_received=$(wc -l < inbound_dealing_received.debug.txt)
    current_dealings_sent=$(wc -l < dealing_sent.debug.txt)
    current_client_sent=$(wc -l < client_sent.debug.txt)
    current_server_sent=$(wc -l < server_sent.debug.txt)
    current_client_received=$(wc -l < client_received.debug.txt)
    current_server_received=$(wc -l < server_received.debug.txt)
    echo "$current_dealings_created / $expected_openings signatures have been created,
    $current_client_sent / $expected_on_one_side signatures have been sent to client side,
    $current_server_sent / $expected_on_one_side signatures have been sent to server side,
    $current_dealings_sent / $expected_sent total signatures have been sent,
    $current_client_received / $expected_on_one_side signatures have been received on client side,
    $current_server_received / $expected_on_one_side signatures have been received on server side,
    $current_dealings_received / $expected_sent signatures have been received,
    $current_openings_complete / $expected_openings signature rounds have completed"
}

until [ $(wc -l < opening_complete.debug.txt) == $expected_openings ]
do
    print_dealings_stats
    sleep 1
done
print_dealings_stats
current_openings_complete=$(wc -l < opening_complete.debug.txt)
echo "$current_openings_complete / $expected_openings siganture rounds have completed."
echo "All nodes have completed an initial opening."

# This is a check for a race condition where more than expected dealing openings happen
sleep 5
if [ $(wc -l < opening_complete.debug.txt) != $expected_openings ]; then
    echo "More signatures than expected were prodcued!"
    exit 1
fi

exit 0
