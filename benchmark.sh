#!/bin/bash

# Simple benchmark script for RustWeb server

set -e

echo "🦀 RustWeb Performance Benchmark"
echo "================================"

# Configuration
CONFIG_FILE="rustweb.toml"
SERVER_PORT="8080"
TEST_DURATION="10s"
CONCURRENCY="100"

# Check if server binary exists
if [ ! -f "./target/release/rustweb" ]; then
    echo "❌ Server binary not found. Please run: cargo build --release"
    exit 1
fi

# Check if configuration exists
if [ ! -f "$CONFIG_FILE" ]; then
    echo "❌ Configuration file $CONFIG_FILE not found."
    exit 1
fi

echo "📋 Configuration:"
echo "   - Config: $CONFIG_FILE"
echo "   - Port: $SERVER_PORT"
echo "   - Duration: $TEST_DURATION"
echo "   - Concurrency: $CONCURRENCY"
echo

# Start the server
echo "🚀 Starting RustWeb server..."
./target/release/rustweb -c "$CONFIG_FILE" > benchmark.log 2>&1 &
SERVER_PID=$!

# Wait for server to start
sleep 2

# Check if server is running
if ! kill -0 "$SERVER_PID" 2>/dev/null; then
    echo "❌ Failed to start server. Check benchmark.log for details."
    cat benchmark.log
    exit 1
fi

echo "✅ Server started successfully (PID: $SERVER_PID)"
echo

# Test basic connectivity
echo "🔗 Testing connectivity..."
if curl -s -f http://localhost:$SERVER_PORT/ > /dev/null; then
    echo "✅ Server is responding"
else
    echo "❌ Server is not responding"
    kill $SERVER_PID 2>/dev/null
    exit 1
fi
echo

# Run benchmarks
echo "📊 Running performance benchmarks..."
echo

# Test 1: Basic HTTP performance
echo "Test 1: Basic static file serving"
echo "================================="
if command -v wrk >/dev/null 2>&1; then
    wrk -t4 -c$CONCURRENCY -d$TEST_DURATION --latency http://localhost:$SERVER_PORT/ 2>/dev/null || echo "wrk test failed"
elif command -v ab >/dev/null 2>&1; then
    ab -t 10 -c $CONCURRENCY http://localhost:$SERVER_PORT/ 2>/dev/null | grep -E "(Requests per second|Time per request|Transfer rate)" || echo "ab test failed"
else
    echo "⚠️  No benchmark tool found (wrk or ab). Testing with curl..."
    echo "Running 100 sequential requests..."
    start_time=$(date +%s.%N)
    for i in {1..100}; do
        curl -s -o /dev/null http://localhost:$SERVER_PORT/ || break
    done
    end_time=$(date +%s.%N)
    duration=$(echo "$end_time - $start_time" | bc -l 2>/dev/null || echo "calculation failed")
    if [ "$duration" != "calculation failed" ]; then
        rps=$(echo "scale=2; 100 / $duration" | bc -l)
        echo "Completed 100 requests in ${duration}s (${rps} RPS)"
    fi
fi
echo

# Test 2: Different content types
echo "Test 2: Different content types"
echo "=============================="

# Create test files
mkdir -p www/test
echo '{"message": "Hello, World!", "server": "RustWeb", "timestamp": "'$(date -Iseconds)'"}' > www/test/api.json
echo "body { font-family: Arial; } h1 { color: blue; }" > www/test/style.css
echo "console.log('RustWeb test');" > www/test/app.js

echo "Testing JSON response..."
curl -w "Status: %{http_code}, Time: %{time_total}s, Size: %{size_download} bytes\\n" \
     -o /dev/null -s http://localhost:$SERVER_PORT/test/api.json

echo "Testing CSS response..."
curl -w "Status: %{http_code}, Time: %{time_total}s, Size: %{size_download} bytes\\n" \
     -o /dev/null -s http://localhost:$SERVER_PORT/test/style.css

echo "Testing JavaScript response..."
curl -w "Status: %{http_code}, Time: %{time_total}s, Size: %{size_download} bytes\\n" \
     -o /dev/null -s http://localhost:$SERVER_PORT/test/app.js
echo

# Test 3: Compression
echo "Test 3: Compression support"
echo "=========================="
echo "Testing GZIP compression..."
gzip_size=$(curl -H "Accept-Encoding: gzip" -s http://localhost:$SERVER_PORT/ | wc -c)
normal_size=$(curl -s http://localhost:$SERVER_PORT/ | wc -c)
echo "Normal response: $normal_size bytes"
echo "Gzipped response: $gzip_size bytes"
if [ "$normal_size" -gt 0 ] && [ "$gzip_size" -gt 0 ]; then
    compression_ratio=$(echo "scale=1; $normal_size / $gzip_size" | bc -l 2>/dev/null || echo "N/A")
    echo "Compression ratio: ${compression_ratio}x"
fi
echo

# Test 4: Virtual hosts
echo "Test 4: Virtual hosts"
echo "=================="
echo "Testing default host..."
curl -w "Status: %{http_code}\\n" -o /dev/null -s http://localhost:$SERVER_PORT/

echo "Testing example.com virtual host..."
curl -H "Host: example.com" -w "Status: %{http_code}\\n" -o /dev/null -s http://localhost:$SERVER_PORT/
echo

# Test 5: Security features
echo "Test 5: Security features"
echo "======================="
echo "Testing security headers..."
curl -I -s http://localhost:$SERVER_PORT/ | grep -E "(X-Frame-Options|X-Content-Type-Options|Server|X-XSS-Protection)" | sed 's/^/  /'

echo "Testing path traversal protection..."
curl -w "Status: %{http_code}\\n" -o /dev/null -s http://localhost:$SERVER_PORT/../../../etc/passwd
echo

# Test 6: Error handling
echo "Test 6: Error handling"
echo "===================="
echo "Testing 404 response..."
curl -w "Status: %{http_code}\\n" -o /dev/null -s http://localhost:$SERVER_PORT/nonexistent-file
echo

# Clean up
echo "🧹 Cleaning up..."
kill $SERVER_PID 2>/dev/null || true
wait $SERVER_PID 2>/dev/null || true

# Show server stats from log
echo "📈 Server Statistics:"
echo "===================="
if [ -f "benchmark.log" ]; then
    echo "Requests processed:"
    grep "Request completed" benchmark.log | wc -l | sed 's/^/  Total requests: /'
    
    echo "Status code distribution:"
    grep "Request completed" benchmark.log | grep -o "status=[0-9]*" | sort | uniq -c | sed 's/^/  /'
    
    echo "Last few log entries:"
    tail -5 benchmark.log | sed 's/^/  /'
else
    echo "  No log file found"
fi

echo
echo "✅ Benchmark completed!"
echo "📝 Full server log available in: benchmark.log"