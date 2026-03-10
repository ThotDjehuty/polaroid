#!/bin/bash
# Start polarway-grpc server with Python dylib path
export DYLD_LIBRARY_PATH=$(python3 -c "import sys; print(sys.prefix + '/lib')"):$DYLD_LIBRARY_PATH
echo "Starting polarway-grpc server on port 50051..."
exec /Users/melvinalvarez/Documents/Workspace/polarway/target/debug/polarway-grpc
