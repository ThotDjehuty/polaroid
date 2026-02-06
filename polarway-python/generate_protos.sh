#!/bin/bash
# Generate Python gRPC stubs from proto files

set -e

cd "$(dirname "$0")"

echo "🔨 Generating Python gRPC stubs..."

python -m grpc_tools.protoc \
    -I../proto \
    --python_out=polarway \
    --grpc_python_out=polarway \
    ../proto/polarway.proto

echo "✅ Generated polarway_pb2.py and polarway_pb2_grpc.py"

# Fix imports in generated files (Python 3.9+ compatibility)
if [[ "$OSTYPE" == "darwin"* ]]; then
    # macOS
    sed -i '' 's/import polarway_pb2/from . import polarway_pb2/g' polarway/polarway_pb2_grpc.py
else
    # Linux
    sed -i 's/import polarway_pb2/from . import polarway_pb2/g' polarway/polarway_pb2_grpc.py
fi

echo "✅ Fixed imports"
echo "🎉 Done!"
