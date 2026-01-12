# Polaroid Quick Reference

A comprehensive cheat sheet for common Polaroid operations.

## 🚀 Getting Started

```python
import polaroid as pd

# Connect to gRPC server
client = pd.connect("localhost:50051")
```

## 📥 Data Loading

### Reading Files

```python
# Parquet (recommended for performance)
df = pd.read_parquet("data.parquet")

# With column selection
df = pd.read_parquet("data.parquet", columns=["col1", "col2"])

# With server-side filtering
df = pd.read_parquet("data.parquet", predicate="price > 100")

# CSV
df = pd.read_csv("data.csv")
df = pd.read_csv("data.csv", separator=";", has_header=True)

# JSON
df = pd.read_json("data.json")
df = pd.read_json("data.ndjson", format="ndjson")  # Newline-delimited
```

### Reading from Streams

```python
# WebSocket
stream = pd.from_websocket(
    url="wss://stream.example.com/ws",
    schema={"price": pd.Float64, "timestamp": pd.Datetime("ms")},
    format="json"
)

# Process in batches
async for batch in stream.batches(size=1000):
    print(batch)

# REST API with pagination
df = pd.read_rest_api(
    url="https://api.example.com/data",
    pagination="cursor",
    page_size=1000
)
```

## 🔍 Data Inspection

```python
# View first rows
df.head(10)

# View last rows
df.tail(10)

# Schema information
df.schema()

# Row count
df.count()

# Summary statistics
df.describe()

# Column names
df.columns()
```

## ✂️ Selecting Columns

```python
# Select specific columns
df.select(["col1", "col2"])

# Select with expressions
df.select([
    pd.col("price"),
    pd.col("volume").alias("vol"),
    (pd.col("price") * pd.col("volume")).alias("notional")
])

# Drop columns
df.drop(["unwanted_col1", "unwanted_col2"])

# Rename columns
df.rename({"old_name": "new_name"})
```

## 🔎 Filtering Rows

```python
# Simple filter
df.filter(pd.col("price") > 100)

# Multiple conditions (AND)
df.filter(
    (pd.col("price") > 100) & 
    (pd.col("volume") > 1000)
)

# Multiple conditions (OR)
df.filter(
    (pd.col("symbol") == "AAPL") | 
    (pd.col("symbol") == "GOOGL")
)

# String operations
df.filter(pd.col("name").str.contains("Apple"))
df.filter(pd.col("email").str.ends_with("@example.com"))

# Null handling
df.filter(pd.col("value").is_not_null())
df.filter(pd.col("optional").is_null())
```

## ➕ Adding/Modifying Columns

```python
# Add new column
df.with_column(
    (pd.col("price") * 1.1).alias("price_with_tax")
)

# Multiple columns at once
df.with_columns([
    (pd.col("price") * pd.col("quantity")).alias("total"),
    pd.col("price").cast(pd.Int64).alias("price_int")
])

# Conditional column
df.with_column(
    pd.when(pd.col("price") > 100)
      .then(pd.lit("expensive"))
      .otherwise(pd.lit("affordable"))
      .alias("price_category")
)
```

## 📊 Grouping and Aggregation

```python
# Simple group by
df.group_by("symbol").agg({"price": "mean"})

# Multiple aggregations
df.group_by("symbol").agg({
    "price": ["mean", "max", "min", "std"],
    "volume": ["sum", "count"]
})

# Custom aggregations
df.group_by("symbol").agg([
    pd.col("price").mean().alias("avg_price"),
    pd.col("price").max().alias("max_price"),
    pd.col("volume").sum().alias("total_volume")
])

# Multiple group-by columns
df.group_by(["date", "symbol"]).agg({"price": "mean"})
```

## 🔗 Joining DataFrames

```python
# Inner join
df1.join(df2, on="id", how="inner")

# Left join
df1.join(df2, on="id", how="left")

# Right join
df1.join(df2, on="id", how="right")

# Outer join
df1.join(df2, on="id", how="outer")

# Join on multiple columns
df1.join(df2, on=["col1", "col2"], how="inner")

# Join with different column names
df1.join(df2, left_on="id", right_on="user_id", how="left")
```

## 📈 Sorting

```python
# Sort ascending
df.sort("price")

# Sort descending
df.sort("price", descending=True)

# Multiple columns
df.sort(["symbol", "timestamp"])
df.sort([("symbol", False), ("price", True)])  # symbol asc, price desc
```

## 🧮 Expressions

### Column Operations

```python
# Arithmetic
pd.col("price") + 10
pd.col("price") * pd.col("quantity")
pd.col("value") / 100

# Comparisons
pd.col("price") > 100
pd.col("status") == "active"
pd.col("amount").between(10, 100)

# Logical operations
(pd.col("a") > 5) & (pd.col("b") < 10)
(pd.col("status") == "A") | (pd.col("status") == "B")
~pd.col("flag")  # NOT
```

### String Operations

```python
# Basic string methods
pd.col("name").str.to_uppercase()
pd.col("name").str.to_lowercase()
pd.col("text").str.strip()

# Pattern matching
pd.col("email").str.contains("@gmail.com")
pd.col("text").str.starts_with("Hello")
pd.col("file").str.ends_with(".csv")

# String manipulation
pd.col("text").str.replace("old", "new")
pd.col("full_name").str.split(" ")
pd.col("values").str.slice(0, 10)
```

### Datetime Operations

```python
# Extract components
pd.col("timestamp").dt.year()
pd.col("timestamp").dt.month()
pd.col("timestamp").dt.day()
pd.col("timestamp").dt.hour()
pd.col("timestamp").dt.minute()

# Date arithmetic
pd.col("date") + pd.duration(days=7)
pd.col("end_date") - pd.col("start_date")

# Formatting
pd.col("timestamp").dt.strftime("%Y-%m-%d")

# Timezone conversion
pd.col("timestamp").dt.convert_timezone("UTC")
```

### Null Handling

```python
# Check for nulls
pd.col("value").is_null()
pd.col("value").is_not_null()

# Fill nulls
pd.col("value").fill_null(0)
pd.col("value").fill_null_with_strategy("forward")
pd.col("value").fill_null_with_strategy("backward")

# Drop nulls
df.drop_nulls()
df.drop_nulls(subset=["important_col"])
```

### Conditional Logic

```python
# Simple when-then-otherwise
pd.when(pd.col("price") > 100)
  .then(pd.lit("high"))
  .otherwise(pd.lit("low"))

# Multiple conditions
pd.when(pd.col("price") > 100)
  .then(pd.lit("high"))
  .when(pd.col("price") > 50)
  .then(pd.lit("medium"))
  .otherwise(pd.lit("low"))
```

## ⚡ Performance Operations

### Lazy Evaluation

```python
# Build lazy query
lazy_df = df.lazy()

# Chain operations
result = (
    lazy_df
    .filter(pd.col("price") > 100)
    .select(["symbol", "price"])
    .group_by("symbol")
    .agg({"price": "mean"})
    .collect()  # Execute query
)
```

### Streaming (for large datasets)

```python
# Stream large file
for batch in pd.scan_parquet("huge_file.parquet").iter_batches():
    # Process each batch
    processed = batch.filter(pd.col("value") > 0)
    processed.write_parquet("output.parquet", mode="append")
```

### Parallel Operations

```python
import asyncio

async def process_files_parallel():
    async with pd.AsyncClient("localhost:50051") as client:
        # Read 100 files in parallel
        handles = await asyncio.gather(*[
            client.read_parquet(f"file_{i}.parquet")
            for i in range(100)
        ])
        
        # Process all in parallel
        results = await asyncio.gather(*[
            h.filter(pd.col("value") > 0).collect()
            for h in handles
        ])
    
    return results

results = await process_files_parallel()
```

## 📤 Data Output

### Writing Files

```python
# Parquet (recommended)
df.write_parquet("output.parquet")
df.write_parquet("output.parquet", compression="snappy")

# CSV
df.write_csv("output.csv")
df.write_csv("output.csv", separator=";", include_header=True)

# JSON
df.write_json("output.json")
df.write_json("output.ndjson", format="ndjson")

# Append mode
df.write_parquet("output.parquet", mode="append")
```

### Export to Other Formats

```python
# To PyArrow Table
table = df.collect()  # Returns pyarrow.Table

# To Pandas DataFrame
pandas_df = df.collect().to_pandas()

# To Python dictionaries
records = df.collect().to_pylist()

# To NumPy arrays
arrays = df.collect().to_pydict()
```

## 🎯 Common Patterns

### Method Chaining

```python
result = (
    df
    .filter(pd.col("date") >= "2024-01-01")
    .select(["symbol", "price", "volume"])
    .with_column((pd.col("price") * pd.col("volume")).alias("notional"))
    .group_by("symbol")
    .agg({
        "price": ["mean", "max", "min"],
        "volume": "sum",
        "notional": "sum"
    })
    .sort("notional", descending=True)
    .head(10)
    .collect()
)
```

### Window Functions

```python
# Rolling window
df.with_column(
    pd.col("price")
      .rolling(window_size=20)
      .mean()
      .alias("sma_20")
)

# Partition by group
df.with_column(
    pd.col("price")
      .rank()
      .over(partition_by="symbol", order_by="timestamp")
      .alias("price_rank")
)
```

### Pivot Operations

```python
# Pivot table
df.pivot(
    values="price",
    index="date",
    columns="symbol",
    aggregate_fn="mean"
)

# Unpivot (melt)
df.melt(
    id_vars=["date", "symbol"],
    value_vars=["open", "high", "low", "close"],
    variable_name="price_type",
    value_name="price"
)
```

## ⏱️ Time-Series Operations

### OHLCV Resampling

```python
# Load tick data
ticks = pd.read_parquet("ticks.parquet")

# Convert to time-series
ts = ticks.as_timeseries("timestamp")

# Resample to OHLCV bars
ohlcv_1m = ts.resample_ohlcv(
    "1m",
    price_col="price",
    volume_col="volume"
)

ohlcv_5m = ts.resample_ohlcv("5m", price_col="price", volume_col="volume")
ohlcv_1h = ts.resample_ohlcv("1h", price_col="price", volume_col="volume")
```

### Rolling Window Operations

```python
# Simple moving average
df.with_column(
    pd.col("close").rolling("20m").mean().alias("sma_20")
)

# Multiple aggregations
df.with_columns([
    pd.col("close").rolling("20m").mean().alias("sma_20"),
    pd.col("close").rolling("50m").mean().alias("sma_50"),
    pd.col("volume").rolling("20m").sum().alias("vol_20")
])
```

### Time-Based Operations

```python
# Lag/Lead
df.with_column(pd.col("price").lag(1).alias("prev_price"))
df.with_column(pd.col("price").lead(1).alias("next_price"))

# Difference
df.with_column(pd.col("price").diff().alias("price_change"))

# Percent change
df.with_column(pd.col("price").pct_change().alias("return"))
```

## 🛡️ Error Handling

```python
# Result type pattern
result = df.collect()

if result.is_ok():
    table = result.unwrap()
    print(table)
else:
    error = result.unwrap_err()
    print(f"Error: {error}")

# Monadic operations
result.map(lambda t: print(t)).map_err(lambda e: log_error(e))

# Try-except pattern
try:
    df = pd.read_parquet("file.parquet")
    result = df.collect()
except PolaroidError as e:
    print(f"Operation failed: {e}")
```

## 📊 Type System

```python
# Basic types
pd.Int8, pd.Int16, pd.Int32, pd.Int64
pd.UInt8, pd.UInt16, pd.UInt32, pd.UInt64
pd.Float32, pd.Float64
pd.Boolean
pd.Utf8  # String

# Temporal types
pd.Date
pd.Datetime("ms")  # millisecond precision
pd.Datetime("us")  # microsecond precision
pd.Datetime("ns")  # nanosecond precision
pd.Duration
pd.Time

# Complex types
pd.List(pd.Int64)
pd.Struct({"name": pd.Utf8, "age": pd.Int64})
pd.Categorical

# Type casting
df.with_column(pd.col("value").cast(pd.Float64))
```

---

**See Also**:
- [API Documentation](API_DOCUMENTATION.md) - Full API reference
- [Migration Guide](MIGRATION_GUIDE.md) - Moving from Polars
- [Architecture Guide](ARCHITECTURE.md) - Design deep dive
