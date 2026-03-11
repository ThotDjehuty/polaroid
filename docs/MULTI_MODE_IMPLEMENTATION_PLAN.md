# Polarway Multi-Mode Architecture Plan
**Date**: 2026-01-18

---

## 🎯 Goal

Reorganize Polarway to support 3 deployment modes:
1. **Standalone**: PyO3 Python wheel (local, native)
2. **Distributed**: gRPC polarway-connect (multi-node)  
3. **Portable**: WASM/serverless (client-side)

---

## 🏗️ Current Polarway Architecture

- **src/**
  - **lib.rs** — Main library
  - **dataframe/** — DataFrame engine
  - **grpc_service/** — gRPC server (distributed mode)
  - **lazy/** — Lazy evaluation
- **Cargo.toml** — Rust dependencies
- **docker-compose.yml** — gRPC deployment

**Current Mode**: Distributed only (gRPC)

---

## 🔄 New Architecture

- **crates/**
  - **polarway-core/** — Core DataFrame engine (shared)
    - **src/** — dataframe.rs, lazy.rs, arrow_ops.rs, lib.rs
    - Cargo.toml
  - **polarway-py/** — 🆕 Standalone: PyO3 bindings
    - **src/** — lib.rs (PyO3 wrapper), dataframe.rs, conversions.rs
    - Cargo.toml (dependencies: pyo3, polarway-core)
    - pyproject.toml (Python wheel config)
  - **polarway-connect/** — Distributed: gRPC server
    - **src/** — server.rs, service.rs, main.rs
    - Cargo.toml (dependencies: tonic, polarway-core)
  - **polarway-wasm/** — 🆕 Portable: WASM module
    - **src/** — lib.rs (wasm-bindgen exports), ops.rs
    - Cargo.toml (dependencies: wasm-bindgen, polarway-core)
- **python/** — Python client library
  - **polarway/**
    - **\_\_init\_\_.py** — Unified API
    - **standalone.py** — PyO3 import
    - **distributed.py** — gRPC client
    - **portable.py** — WASM/PyArrow fallback
  - setup.py
- **Cargo.toml** — Workspace config
- **pyproject.toml** — Python package config
- **docker-compose.yml** — Distributed deployment

---

## 📦 Crate Structure

### 1. polarway-core (Shared Library)

**Purpose**: Core DataFrame engine used by all modes

**Dependencies**:
```toml
[dependencies]
polars = "0.36"
arrow = "49.0"
serde = { version = "1.0", features = ["derive"] }
```

**Exports**:
```rust
pub struct DataFrame { ... }
pub struct LazyFrame { ... }
pub trait DataFrameOps { ... }
```

**Features**: Pure Rust, no Python/gRPC/WASM dependencies

---

### 2. polarway-py (Standalone Mode)

**Purpose**: PyO3 bindings for pip install polarway

**Dependencies**:
```toml
[dependencies]
polarway-core = { path = "../polarway-core" }
pyo3 = { version = "0.20", features = ["extension-module"] }
numpy = "0.20"

[lib]
crate-type = ["cdylib"]  # Python extension
```

**Python API**:
```python
import polarway as pl

# Polars-compatible API
df = pl.read_parquet("data.parquet")
df = df.filter(pl.col("price") > 100)
df = df.select(["symbol", "price"])
result = df.collect()  # Execute lazy query
```

**Install**:
```bash
pip install polarway
```

---

### 3. polarway-connect (Distributed Mode)

**Purpose**: gRPC server for multi-node deployment

**Dependencies**:
```toml
[dependencies]
polarway-core = { path = "../polarway-core" }
tonic = "0.10"
prost = "0.12"
tokio = { version = "1", features = ["full"] }

[[bin]]
name = "polarway-connect"
```

**Start Server**:
```bash
polarway-connect --host 0.0.0.0 --port 50052
```

**Docker**:
```bash
docker-compose up polarway-connect
```

**Python Client**:
```python
import polarway as pl

# Connect to remote server
pl.connect("grpc://localhost:50052")

df = pl.read_parquet("s3://bucket/data.parquet")
result = df.filter(pl.col("date") > "2024-01-01").collect()
```

---

### 4. polarway-wasm (Portable Mode)

**Purpose**: WASM module for serverless/browser

**Dependencies**:
```toml
[dependencies]
polarway-core = { path = "../polarway-core" }
wasm-bindgen = "0.2"
serde-wasm-bindgen = "0.6"
js-sys = "0.3"

[lib]
crate-type = ["cdylib"]  # WASM output
```

**Build**:
```bash
wasm-pack build --target web --release
```

**Usage (Browser)**:
```javascript
import init, { read_parquet, filter } from './polarway_wasm.js';

await init();
let df = read_parquet(arrayBuffer);
let filtered = filter(df, "price > 100");
```

**Usage (Python)**:
```python
import polarway as pl

# Automatically uses WASM backend for small data
df = pl.read_parquet("small_data.parquet")  
```

---

## 🔀 Unified Python API

### python/polarway/__init__.py

```python
"""
Polarway - High-performance DataFrame library

Modes:
- Standalone: Native PyO3 (default if installed)
- Distributed: gRPC multi-node
- Portable: PyArrow fallback (always available)
"""

from .router import get_backend, set_backend_mode, BackendMode
from .api import (
    read_parquet,
    read_csv,
    DataFrame,
    LazyFrame,
    col,
)

__all__ = [
    'read_parquet',
    'read_csv',
    'DataFrame',
    'LazyFrame',
    'col',
    'get_backend',
    'set_backend_mode',
    'BackendMode',
]
```

### Backend Selection Logic

```python
# python/polarway/router.py

def get_backend():
    """Auto-detect best backend."""
    
    # 1. Try PyO3 (standalone)
    try:
        import polarway._native  # PyO3 module
        return 'standalone'
    except ImportError:
        pass
    
    # 2. Try gRPC (distributed)
    if _check_grpc_available():
        return 'distributed'
    
    # 3. Fallback to portable (PyArrow)
    return 'portable'
```

---

## 🛠️ Implementation Steps

### Phase 1: Core Refactoring (2-3 days)

1. ✅ Create workspace structure
   ```bash
   cargo new --lib crates/polarway-core
   cargo new --lib crates/polarway-py
   cargo new --bin crates/polarway-connect
   cargo new --lib crates/polarway-wasm
   ```

2. ✅ Extract core engine to polarway-core
   - Move DataFrame/LazyFrame to core
   - Remove gRPC-specific code
   - Keep only Arrow/Polars dependencies

3. ✅ Update Cargo.toml workspace
   ```toml
   [workspace]
   members = [
       "crates/polarway-core",
       "crates/polarway-py",
       "crates/polarway-connect",
       "crates/polarway-wasm",
   ]
   ```

### Phase 2: PyO3 Bindings (2-3 days)

1. Create PyO3 wrapper (polarway-py)
   ```rust
   use pyo3::prelude::*;
   use polarway_core::DataFrame as CoreDataFrame;
   
   #[pyclass]
   struct DataFrame {
       inner: CoreDataFrame,
   }
   
   #[pymethods]
   impl DataFrame {
       fn filter(&self, expr: &str) -> PyResult<Self> {
           let filtered = self.inner.filter(parse_expr(expr))?;
           Ok(DataFrame { inner: filtered })
       }
   }
   ```

2. Build Python wheel
   ```bash
   cd crates/polarway-py
   maturin develop  # For development
   maturin build --release  # For distribution
   ```

3. Test Polars compatibility
   ```python
   import polarway as pl
   
   # Should work like Polars
   df = pl.read_parquet("data.parquet")
   result = df.filter(pl.col("price") > 100).collect()
   ```

### Phase 3: gRPC Refactoring (1-2 days)

1. Move gRPC server to polarway-connect
   - Use polarway-core for engine
   - Keep only gRPC service code

2. Update proto definitions
   ```protobuf
   service PolarwayConnect {
       rpc ReadParquet(ReadRequest) returns (DataFrame);
       rpc Filter(FilterRequest) returns (DataFrame);
       rpc Collect(CollectRequest) returns (ArrowBatch);
   }
   ```

3. Test distributed mode
   ```bash
   # Start server
   cargo run --bin polarway-connect
   
   # Test from Python
   python -c "import polarway; pl.connect('grpc://localhost:50052')"
   ```

### Phase 4: WASM Module (2-3 days)

1. Create WASM bindings (polarway-wasm)
   ```rust
   use wasm_bindgen::prelude::*;
   use polarway_core::DataFrame;
   
   #[wasm_bindgen]
   pub fn read_parquet(bytes: &[u8]) -> JsValue {
       let df = DataFrame::from_parquet_bytes(bytes).unwrap();
       serde_wasm_bindgen::to_value(&df).unwrap()
   }
   ```

2. Build WASM module
   ```bash
   cd crates/polarway-wasm
   wasm-pack build --target web --release
   ```

3. Test in browser
   ```javascript
   const df = await polarway.read_parquet(buffer);
   console.log(df.shape());
   ```

### Phase 5: Python API Unification (2 days)

1. Create unified API (python/polarway/)
   - Auto-detect backend
   - Consistent API across all modes
   - Smart fallback logic

2. Test all modes
   ```python
   # Test standalone
   os.environ['POLARWAY_MODE'] = 'standalone'
   df = pl.read_parquet("data.parquet")
   
   # Test distributed
   os.environ['POLARWAY_MODE'] = 'distributed'
   df = pl.read_parquet("data.parquet")
   
   # Test portable
   os.environ['POLARWAY_MODE'] = 'portable'
   df = pl.read_parquet("data.parquet")
   ```

---

## 📊 Deployment Guide

### Standalone Mode (Development)

```bash
# Install wheel
pip install polarway

# Use locally
python -c "import polarway as pl; df = pl.read_parquet('data.parquet')"
```

### Distributed Mode (Production)

```bash
# Start server
docker-compose up polarway-connect

# Connect from client
export POLARWAY_MODE=distributed
python app.py
```

### Portable Mode (Serverless)

```bash
# No server needed
export POLARWAY_MODE=portable
python app.py  # Uses PyArrow
```

---

## 🎯 Success Criteria

- ✅ Single codebase, 3 deployment modes
- ✅ Polars API compatibility (standalone mode)
- ✅ Existing gRPC functionality preserved
- ✅ WASM module <200KB
- ✅ Zero breaking changes to current users
- ✅ Automatic backend selection
- ✅ pip install polarway (standalone)
- ✅ docker-compose up (distributed)
- ✅ Browser/serverless support (portable)

---

**Timeline**: 2-3 weeks  
**Effort**: High (major refactoring)  
**Impact**: Massive (enables all 3 use cases)  
**Risk**: Medium (careful testing needed)

---

**Next Action**: Start Phase 1 (core refactoring) ✅
