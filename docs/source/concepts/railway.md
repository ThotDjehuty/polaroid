# Railway-Oriented Programming

**Railway-Oriented Programming** (ROP) is a functional programming pattern that treats data pipelines as train tracks with explicit success and failure paths.

## The Problem

Traditional imperative code hides errors:

```python
# ❌ Traditional: Errors are hidden landmines
def process_data(path):
    try:
        data = load_csv(path)        # Could fail
        filtered = filter_data(data)  # Could fail
        result = aggregate(filtered)  # Could fail
        return result
    except Exception as e:
        log.error(f"Something broke: {e}")
        return None  # Lost context!
```

**Problems:**
- ❌ Error origins unclear
- ❌ Silent failures with `None`
- ❌ Hard to compose operations
- ❌ Implicit failure modes

## The Solution

Railway-Oriented Programming makes errors explicit:

```python
# ✅ Polarway: Explicit success/failure paths
def process_data(path):
    return (
        pw.load_csv(path)           # Result<DataFrame, LoadError>
        .and_then(filter_data)      # Result<DataFrame, FilterError>
        .and_then(aggregate)        # Result<DataFrame, AggError>
        .map_err(log_error)         # Transform errors
    )
    # Returns Result<DataFrame, Error> - always explicit!
```

**Benefits:**
- ✅ Every error type is known
- ✅ No silent failures
- ✅ Composable operations
- ✅ Type-safe transformations

## Result<T, E> Type

The `Result` type represents either success (`Ok`) or failure (`Err`):

```python
from polarway import Result, Ok, Err

# Success case
success: Result[int, str] = Ok(42)
match success:
    case Ok(value):
        print(f"Got value: {value}")  # Prints: Got value: 42
    case Err(error):
        print(f"Got error: {error}")

# Failure case
failure: Result[int, str] = Err("Division by zero")
match failure:
    case Ok(value):
        print(f"Got value: {value}")
    case Err(error):
        print(f"Got error: {error}")  # Prints: Got error: Division by zero
```

## Option<T> Type

The `Option` type represents optional values without `None`:

```python
from polarway import Option, Some, Null

# Has value
some_value: Option[int] = Some(42)
value = some_value.unwrap_or(0)  # Returns 42

# No value
no_value: Option[int] = Null()
value = no_value.unwrap_or(0)  # Returns 0 (default)
```

## Chaining Operations

### and_then

Chain operations that can fail:

```python
result = (
    pw.read_csv("data.csv")                    # Result<DataFrame, IOError>
    .and_then(lambda df: validate(df))         # Result<DataFrame, ValidationError>
    .and_then(lambda df: transform(df))        # Result<DataFrame, TransformError>
)
# Type: Result<DataFrame, IOError | ValidationError | TransformError>
```

### map

Transform success values:

```python
result = (
    pw.read_csv("data.csv")                    # Result<DataFrame, IOError>
    .map(lambda df: df.filter(pw.col("price") > 0))  # Still Result<DataFrame, IOError>
)
```

### map_err

Transform error values:

```python
result = (
    pw.read_csv("data.csv")
    .map_err(lambda e: f"Failed to load: {e}")  # Custom error message
)
```

### or_else

Recover from errors:

```python
result = (
    pw.read_csv("primary.csv")
    .or_else(lambda _: pw.read_csv("backup.csv"))  # Fallback
    .or_else(lambda _: pw.Ok(pw.DataFrame.empty()))  # Last resort
)
```

## Railway Diagram

```text
Success Track (Ok)         Failure Track (Err)
─────────────────          ────────────────────

    read_csv
       │
       ├─────────────────► IOError
       ▼                      │
    validate                  │
       │                      │
       ├─────────────────► ValidationError
       ▼                      │
   transform                  │
       │                      │
       ├─────────────────► TransformError
       ▼                      │
   Result<T>              Result<E>
```

Operations stay on the **success track** until an error occurs, then switch to the **failure track**. Once on the failure track, subsequent operations are skipped.

## Real-World Example

### Data Processing Pipeline

```python
from polarway import Result, Ok, Err
import polarway as pw

def load_data(path: str) -> Result[pw.DataFrame, str]:
    """Load CSV with validation"""
    return pw.read_csv(path).map_err(lambda e: f"Load failed: {e}")

def validate_schema(df: pw.DataFrame) -> Result[pw.DataFrame, str]:
    """Validate required columns"""
    required = {"symbol", "price", "date"}
    if not required.issubset(df.columns):
        return Err(f"Missing columns: {required - set(df.columns)}")
    return Ok(df)

def clean_data(df: pw.DataFrame) -> Result[pw.DataFrame, str]:
    """Remove invalid records"""
    return Ok(df.filter(
        (pw.col("price") > 0) & 
        (pw.col("symbol").str.len() > 0)
    ))

def aggregate(df: pw.DataFrame) -> Result[pw.DataFrame, str]:
    """Calculate statistics"""
    return Ok(df.group_by("symbol").agg({
        "price": ["mean", "std", "min", "max"]
    }))

# Compose pipeline
pipeline = (
    load_data("market_data.csv")
    .and_then(validate_schema)
    .and_then(clean_data)
    .and_then(aggregate)
    .map_err(lambda e: f"Pipeline failed: {e}")
)

# Execute
match pipeline:
    case Ok(result):
        print(f"✅ Success: {len(result)} symbols processed")
        print(result)
    case Err(error):
        print(f"❌ Failure: {error}")
```

## Benefits

### 1. Explicit Error Handling

No more guessing where errors come from:

```python
# ✅ Clear error types at each stage
Result<DataFrame, IOError>
    → Result<DataFrame, ValidationError>
    → Result<DataFrame, TransformError>
```

### 2. Composability

Chain operations naturally:

```python
result = (
    load_data()
    .and_then(validate)
    .and_then(transform)
    .and_then(aggregate)
)
```

### 3. Type Safety

Compile-time error checking (in Rust):

```rust
// Rust example
let result: Result<DataFrame, Error> = 
    read_csv("data.csv")?
    .filter(col("price").gt(0))?
    .group_by(&["symbol"])?;
// ✅ Type-checked at compile time!
```

### 4. No Silent Failures

Every error must be handled:

```python
# ❌ This won't compile (in strongly-typed systems)
result = pw.read_csv("data.csv")  # Result<DataFrame, IOError>
result.filter(...)  # ERROR: can't call methods on Result!

# ✅ Must unwrap explicitly
match result:
    case Ok(df):
        df.filter(...)  # Now it's a DataFrame
    case Err(e):
        handle_error(e)
```

## Comparison

| Traditional | Railway-Oriented |
|-------------|------------------|
| `try/except` | `Result<T, E>` |
| `None` values | `Option<T>` |
| Hidden errors | Explicit errors |
| Silent failures | Type-safe failures |
| Hard to compose | Easy to chain |
| Runtime errors | Compile-time checks |

## Further Reading

- [Scott Wlaschin - Railway-Oriented Programming](https://fsharpforfunandprofit.com/rop/)
- [Rust Book - Error Handling](https://doc.rust-lang.org/book/ch09-00-error-handling.html)
- [Examples](../examples.md) - Real-world usage
- [API Reference](../api/reference.md) - Complete API
