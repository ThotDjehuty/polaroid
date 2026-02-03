#!/bin/bash
cd /Users/melvinalvarez/Documents/Workspace/polaroid

echo "🚀 Starting Polaroid HTTP server..."
./serverless/target/release/polaroid-http > /tmp/polaroid-test.log 2>&1 &
POLAROID_PID=$!

sleep 3

echo "✅ Testing /health endpoint..."
curl -s http://localhost:8080/health | python3 -m json.tool

echo ""
echo "✅ Testing /api/discover-pairs endpoint..."
curl -s -X POST http://localhost:8080/api/discover-pairs \
  -H "Content-Type: application/json" \
  -d '{"symbols":["BTC-USD","ETH-USD","AAPL","MSFT"]}' | python3 -m json.tool

echo ""
echo "✅ Testing /api/stream-data endpoint..."
curl -s -X POST http://localhost:8080/api/stream-data \
  -H "Content-Type: application/json" \
  -d '{"source":"parquet","path":"/tmp/test.parquet","limit":10}' | python3 -m json.tool || echo '(Expected error - no test data file)'

echo ""
echo "✅ Testing /metrics endpoint..."
curl -s http://localhost:8080/metrics | head -20

echo ""
echo "✅ Testing authentication (should work as guest without token)..."
curl -s -X POST http://localhost:8080/api/discover-pairs \
  -H "Content-Type: application/json" \
  -d '{"symbols":["BTC-USD","ETH-USD"]}' | python3 -m json.tool | head -10

echo ""
echo "🛑 Stopping server..."
kill $POLAROID_PID 2>/dev/null
wait $POLAROID_PID 2>/dev/null

echo "✅ All tests complete!"
