# Polarway Docker Deployment

## Quick Start

### 1. Build and Run Polarway gRPC Server
```bash
# Build the Docker image
docker-compose -f docker-compose.polarway.yml build

# Start the server
docker-compose -f docker-compose.polarway.yml up -d

# Check logs
docker-compose -f docker-compose.polarway.yml logs -f polarway-grpc

# Check health
docker ps | grep polarway
```

### 2. Install Polarway Python Client
```bash
# In your notebook environment
pip install -e polarway-python/
```

### 3. Run Tests
Open `notebooks/phase2_operations_test.ipynb` and run all cells.

The notebook will connect to the Dockerized server at `localhost:50051`.

## Architecture

```
┌──────────────────────┐
│  Jupyter Notebook    │
│  (Your Machine)      │
│                      │
│  import polarway     │
│  pld.connect()       │
└──────────┬───────────┘
           │ gRPC :50051
           │ Arrow IPC
           ▼
┌──────────────────────┐
│  Docker Container    │
│  polarway-grpc       │
│                      │
│  Rust + Polars       │
│  gRPC Server         │
└──────────────────────┘
```

## Environment Variables

```bash
export POLARWAY_GRPC_HOST=localhost  # or Docker service name
export POLARWAY_GRPC_PORT=50051
export TEST_DATA_DIR=/tmp
```

## Docker Network (Multi-Container)

If running Jupyter in Docker:

```yaml
services:
  jupyter:
    image: jupyter/scipy-notebook
    networks:
      - polarway-net
    environment:
      - POLARWAY_GRPC_HOST=polarway-grpc  # Use service name
  
  polarway-grpc:
    # ... same as docker-compose.polarway.yml
    networks:
      - polarway-net

networks:
  polarway-net:
    driver: bridge
```

## Production Deployment

```bash
# Build optimized image
docker build -f Dockerfile.polarway -t polarway-grpc:latest .

# Run with resource limits
docker run -d \
  --name polarway-grpc \
  -p 50051:50051 \
  --memory=4g \
  --cpus=2 \
  -e RUST_LOG=info \
  polarway-grpc:latest
```

## Troubleshooting

### Server not starting
```bash
# Check logs
docker logs polarway-grpc

# Restart
docker-compose -f docker-compose.polarway.yml restart
```

### Connection refused
```bash
# Check server is listening
docker exec polarway-grpc netcat -zv localhost 50051

# Check port mapping
docker port polarway-grpc

# Check from host
nc -zv localhost 50051
```

### Import error
```bash
# Ensure polarway client is installed
pip install -e polarway-python/

# Verify
python -c "import polarway; print(polarway.__version__)"
```

## Stop and Clean Up

```bash
# Stop server
docker-compose -f docker-compose.polarway.yml down

# Remove volumes
docker-compose -f docker-compose.polarway.yml down -v
```
