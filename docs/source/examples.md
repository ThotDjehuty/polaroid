# Examples

Real-world examples of using Polarway for common data engineering tasks.

## Basic Data Processing

### Load and Filter

```python
import polarway as pw

# Load CSV with error handling
result = pw.read_csv("sales.csv").and_then(
    lambda df: df.filter(pw.col("revenue") > 1000)
)

match result:
    case pw.Ok(filtered):
        print(f"Found {len(filtered)} high-value sales")
    case pw.Err(e):
        print(f"Error: {e}")
```

### Aggregation

```python
# Group by symbol and calculate statistics
result = (
    pw.read_parquet("market_data.parquet")
    .and_then(lambda df: df.group_by("symbol").agg({
        "price": ["mean", "std", "min", "max"],
        "volume": "sum"
    }))
)
```

## Time-Series Operations

### OHLCV Resampling

```python
# Resample 1-minute data to 5-minute candles
result = (
    pw.read_csv("tick_data.csv")
    .and_then(lambda df: df.resample_ohlcv(
        time_column="timestamp",
        interval="5m",
        ohlcv_columns=["open", "high", "low", "close", "volume"]
    ))
)
```

### Rolling Windows

```python
# Calculate 20-period moving average
result = (
    pw.read_parquet("prices.parquet")
    .and_then(lambda df: df.with_columns([
        pw.col("close").rolling_mean(20).alias("sma_20"),
        pw.col("close").rolling_std(20).alias("volatility")
    ]))
)
```

## Remote Execution

### Connect to gRPC Server

```python
import polarway as pw

# Connect to remote Polarway server
client = pw.connect("polarway.example.com:50051")

# Execute on remote server (heavy computation on powerful machine)
result = (
    client.read_parquet("s3://data-lake/prices.parquet")
    .filter(pw.col("date") >= "2024-01-01")
    .group_by("symbol")
    .agg({"price": "mean"})
    .collect()
)
```

## Streaming Large Files

### Process 100GB+ Datasets

```python
# Handle data larger than RAM
result = (
    pw.scan_csv("massive_file.csv")  # Lazy scan (no memory load)
    .filter(pw.col("year") == 2024)
    .select(["symbol", "price", "volume"])
    .group_by("symbol")
    .agg({"price": "mean", "volume": "sum"})
    .collect(streaming=True)  # Stream chunks, constant memory
)
```

## Error Handling Patterns

### Composable Error Handling

```python
def validate_schema(df: pw.DataFrame) -> pw.Result[pw.DataFrame, str]:
    """Validate required columns exist"""
    required = {"symbol", "price", "date"}
    if not required.issubset(df.columns):
        return pw.Err(f"Missing columns: {required - set(df.columns)}")
    return pw.Ok(df)

def filter_valid_prices(df: pw.DataFrame) -> pw.Result[pw.DataFrame, str]:
    """Remove invalid prices"""
    return pw.Ok(df.filter(pw.col("price") > 0))

# Compose operations
pipeline = (
    pw.read_csv("data.csv")
    .and_then(validate_schema)
    .and_then(filter_valid_prices)
    .map_err(lambda e: f"Pipeline failed: {e}")
)
```

### Recovery from Errors

```python
# Try primary source, fallback to backup
result = (
    pw.read_csv("primary.csv")
    .or_else(lambda _: pw.read_csv("backup.csv"))
    .or_else(lambda _: pw.Ok(pw.DataFrame.empty()))  # Empty as last resort
)
```

## Hybrid Storage Examples

### Automatic Caching

```python
# First read: loads from Parquet (~50ms)
df1 = pw.read_parquet("data.parquet")

# Second read: served from cache (~1ms)
df2 = pw.read_parquet("data.parquet")  # Blazing fast!
```

### DuckDB SQL Analytics

```python
# Complex SQL queries on Parquet files
result = pw.sql("""
    SELECT 
        symbol,
        AVG(price) as avg_price,
        STDDEV(price) as volatility
    FROM 'data/*.parquet'
    WHERE date >= '2024-01-01'
    GROUP BY symbol
    ORDER BY volatility DESC
    LIMIT 10
""")
```

## Advanced Patterns

### Monadic Composition

```python
from functools import reduce

def compose_pipeline(operations):
    """Compose multiple operations into single pipeline"""
    def pipeline(df):
        return reduce(
            lambda acc, op: acc.and_then(op),
            operations,
            pw.Ok(df)
        )
    return pipeline

# Define operations
ops = [
    lambda df: df.filter(pw.col("price") > 0),
    lambda df: df.with_columns([pw.col("price").log().alias("log_price")]),
    lambda df: df.group_by("symbol").agg({"log_price": "mean"})
]

# Execute composed pipeline
result = compose_pipeline(ops)(initial_df)
```

## Next Steps

- [Quickstart](quickstart.md) - Get started quickly
- [Concepts](concepts/index.md) - Understand core concepts
- [API Reference](api/reference.md) - Full API documentation
