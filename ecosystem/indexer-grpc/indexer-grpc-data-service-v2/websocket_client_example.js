#!/usr/bin/env node
/**
 * Example WebSocket client for indexer-grpc-data-service-v2
 * 
 * Usage:
 * 1. Install dependencies: npm install ws
 * 2. Start the indexer service with WebSocket enabled
 * 3. Run this script: node websocket_client_example.js
 */

const WebSocket = require('ws');

// WebSocket server configuration
const WS_HOST = 'localhost';
const WS_PORT = 8000;

// Example: Connect to transactions stream
function connectToTransactionsStream() {
    const ws = new WebSocket(`ws://${WS_HOST}:${WS_PORT}/ws/transactions`);

    ws.on('open', function open() {
        console.log('Connected to transactions WebSocket');

        // Send a GetTransactions request
        const request = {
            starting_version: 1,
            transactions_count: 10, // Get 10 transactions
            batch_size: 5,         // 5 transactions per response
            transaction_filter: null // No filtering
        };

        console.log('Sending request:', JSON.stringify(request, null, 2));
        ws.send(JSON.stringify(request));
    });

    ws.on('message', function message(data) {
        try {
            const response = JSON.parse(data.toString());
            console.log('Received message type:', response.type);
            
            if (response.type === 'transactions_response') {
                console.log(`Received ${response.transactions.length} transactions`);
                console.log('Chain ID:', response.chain_id);
                
                // Print basic info about each transaction
                response.transactions.forEach((tx, idx) => {
                    console.log(`  Transaction ${idx + 1}:`);
                    console.log(`    Version: ${tx.version}`);
                    console.log(`    Hash: ${tx.info?.hash ? Buffer.from(tx.info.hash).toString('hex') : 'N/A'}`);
                    console.log(`    Success: ${tx.info?.success ?? 'N/A'}`);
                });
            } else if (response.type === 'error') {
                console.error('Error:', response.message);
            } else if (response.type === 'stream_end') {
                console.log('Stream ended');
                ws.close();
            }
        } catch (e) {
            console.error('Failed to parse message:', e);
        }
    });

    ws.on('error', function error(err) {
        console.error('WebSocket error:', err);
    });

    ws.on('close', function close() {
        console.log('WebSocket connection closed');
    });
}

// Example: Connect to events stream
function connectToEventsStream() {
    const ws = new WebSocket(`ws://${WS_HOST}:${WS_PORT}/ws/events`);

    ws.on('open', function open() {
        console.log('Connected to events WebSocket');

        // Send a GetEvents request
        const request = {
            starting_version: 1,
            transactions_count: 5, // Process 5 transactions
            batch_size: 10,        // Up to 10 events per response
            transaction_filter: null // No filtering
        };

        console.log('Sending request:', JSON.stringify(request, null, 2));
        ws.send(JSON.stringify(request));
    });

    ws.on('message', function message(data) {
        try {
            const response = JSON.parse(data.toString());
            console.log('Received message type:', response.type);

            if (response.type === 'events_response') {
                console.log(`Received ${response.events.length} events`);
                console.log('Chain ID:', response.chain_id);

                // Print basic info about each event
                response.events.forEach((eventWithMetadata, idx) => {
                    console.log(`  Event ${idx + 1}:`);
                    console.log(`    Transaction Version: ${eventWithMetadata.version}`);
                    console.log(`    Transaction Success: ${eventWithMetadata.success}`);
                    console.log(`    Block Height: ${eventWithMetadata.block_height}`);
                    if (eventWithMetadata.event) {
                        console.log(`    Event Type: ${eventWithMetadata.event.type_str ?? 'N/A'}`);
                        console.log(`    Event Sequence: ${eventWithMetadata.event.sequence_number ?? 'N/A'}`);
                    }
                });
            } else if (response.type === 'error') {
                console.error('Error:', response.message);
            } else if (response.type === 'stream_end') {
                console.log('Stream ended');
                ws.close();
            }
        } catch (e) {
            console.error('Failed to parse message:', e);
        }
    });

    ws.on('error', function error(err) {
        console.error('WebSocket error:', err);
    });

    ws.on('close', function close() {
        console.log('WebSocket connection closed');
    });
}

// Main function
function main() {
    const args = process.argv.slice(2);
    const mode = args[0] || 'transactions';

    console.log(`Starting WebSocket client in ${mode} mode...`);
    
    if (mode === 'events') {
        connectToEventsStream();
    } else {
        connectToTransactionsStream();
    }
}

// Run if called directly
if (require.main === module) {
    main();
}

module.exports = {
    connectToTransactionsStream,
    connectToEventsStream
};