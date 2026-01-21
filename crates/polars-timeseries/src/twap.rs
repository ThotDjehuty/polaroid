//! TWAP (Time-Weighted Average Price) calculation
//!
//! TWAP is the average price of a security over a specified time period.
//! Unlike VWAP, it doesn't weight by volume.

use polars::prelude::*;
use crate::error::{TimeSeriesError, TimeSeriesResult};

/// Calculate TWAP for a DataFrame
///
/// # Arguments
/// * `df` - Input DataFrame with time-series data
/// * `time_col` - Name of timestamp column
/// * `price_col` - Name of price column
/// * `window` - Time window (e.g., "5m", "1h", "1d")
///
/// # Returns
/// DataFrame with additional "twap" column
///
/// # Example
/// ```rust,no_run
/// use polars::prelude::*;
/// use polars_timeseries::twap;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let df = DataFrame::new(vec![
///     Series::new("timestamp".into(), vec![1i64, 2, 3, 4, 5]),
///     Series::new("close".into(), vec![100.0, 101.0, 102.0, 101.5, 103.0]),
/// ])?;
///
/// let df_with_twap = twap(&df, "close", 3)?;
/// # Ok(())
/// # }
/// ```
pub fn twap(
    df: &DataFrame,
    price_col: &str,
    window_size: usize,
) -> TimeSeriesResult<DataFrame> {
    // Validate columns
    let col_names = df.get_column_names();
    if !col_names.iter().any(|c| c.as_str() == price_col) {
        return Err(TimeSeriesError::MissingColumn(price_col.to_string()));
    }

    if df.height() == 0 {
        return Err(TimeSeriesError::EmptyDataFrame);
    }

    // Convert to LazyFrame for efficient computation
    let lf = df.clone().lazy();
    let result = twap_lazy(lf, price_col, window_size)?;
    
    Ok(result.collect()?)
}

/// Calculate TWAP using lazy evaluation with fixed window
///
/// More efficient for large datasets
pub fn twap_lazy(
    lf: LazyFrame,
    price_col: &str,
    window_size: usize,
) -> TimeSeriesResult<LazyFrame> {
    // Calculate rolling mean
    let result = lf
        .with_columns([
            col(price_col)
                .rolling_mean(RollingOptionsFixedWindow {
                    window_size,
                    min_periods: 1,
                    center: false,
                    ..Default::default()
                })
                .alias("twap"),
        ]);

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_twap() {
        let df = DataFrame::new(vec![
            Series::new("close".into(), vec![100.0, 101.0, 102.0, 101.5, 103.0]).into(),
        ])
        .unwrap();

        let result = twap(&df, "close", 3);
        assert!(result.is_ok());
        
        let result_df = result.unwrap();
        assert!(result_df.column("twap").is_ok());
        assert_eq!(result_df.height(), 5);
    }
}
