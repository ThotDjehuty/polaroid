//! Predicate pushdown optimization for efficient filtering

use crate::error::Result;
use polars::prelude::*;
use std::ops::BitAnd;

/// Predicate that can be pushed down to file reading
pub trait PredicatePushdown: Send + Sync {
    /// Apply predicate to a DataFrame
    fn apply(&self, df: &DataFrame) -> Result<BooleanChunked>;
}

/// Filter by column value
#[derive(Clone)]
pub struct ColumnFilterPredicate {
    column: String,
    op: FilterOp,
    value: AnyValue<'static>,
}

#[derive(Clone)]
enum FilterOp {
    Eq,
    Neq,
    Lt,
    Le,
    Gt,
    Ge,
}

impl ColumnFilterPredicate {
    pub fn new(column: impl Into<String>, op: &str, value: AnyValue<'static>) -> Self {
        let filter_op = match op {
            "==" | "eq" => FilterOp::Eq,
            "!=" | "neq" => FilterOp::Neq,
            "<" | "lt" => FilterOp::Lt,
            "<=" | "le" => FilterOp::Le,
            ">" | "gt" => FilterOp::Gt,
            ">=" | "ge" => FilterOp::Ge,
            _ => FilterOp::Eq,
        };

        Self {
            column: column.into(),
            op: filter_op,
            value,
        }
    }
}

impl PredicatePushdown for ColumnFilterPredicate {
    fn apply(&self, df: &DataFrame) -> Result<BooleanChunked> {
        let column = df.column(&self.column)?;
        let series = column.as_materialized_series();

        let mask = match &self.op {
            FilterOp::Eq => series.equal(&Series::new("_tmp".into(), vec![self.value.clone()]))?,
            FilterOp::Neq => series.not_equal(&Series::new("_tmp".into(), vec![self.value.clone()]))?,
            FilterOp::Lt => series.lt(&Series::new("_tmp".into(), vec![self.value.clone()]))?,
            FilterOp::Le => series.lt_eq(&Series::new("_tmp".into(), vec![self.value.clone()]))?,
            FilterOp::Gt => series.gt(&Series::new("_tmp".into(), vec![self.value.clone()]))?,
            FilterOp::Ge => series.gt_eq(&Series::new("_tmp".into(), vec![self.value.clone()]))?,
        };

        Ok(mask)
    }
}

/// Combine multiple predicates with AND
pub struct AndPredicate {
    predicates: Vec<Box<dyn PredicatePushdown>>,
}

impl AndPredicate {
    pub fn new(predicates: Vec<Box<dyn PredicatePushdown>>) -> Self {
        Self { predicates }
    }
}

impl PredicatePushdown for AndPredicate {
    fn apply(&self, df: &DataFrame) -> Result<BooleanChunked> {
        let mut result: Option<BooleanChunked> = None;

        for predicate in &self.predicates {
            let mask = predicate.apply(df)?;
            result = match result {
                None => Some(mask),
                Some(prev) => Some((&prev).bitand(&mask)),
            };
        }

        result.ok_or_else(|| {
            crate::error::StreamingError::InvalidConfig("No predicates provided".to_string())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_column_filter() {
        let df = DataFrame::new(vec![
            Series::new("a".into(), vec![1, 2, 3, 4, 5]).into(),
            Series::new("b".into(), vec!["x", "y", "z", "w", "v"]).into(),
        ])
        .unwrap();

        let predicate = ColumnFilterPredicate::new("a", ">", AnyValue::Int32(2));
        let mask = predicate.apply(&df).unwrap();

        assert_eq!(mask.sum().unwrap(), 3); // 3, 4, 5 are > 2
    }

    #[test]
    fn test_and_predicate() {
        let df = DataFrame::new(vec![
            Series::new("a".into(), vec![1, 2, 3, 4, 5]).into(),
            Series::new("b".into(), vec![10, 20, 30, 40, 50]).into(),
        ])
        .unwrap();

        let pred1: Box<dyn PredicatePushdown> =
            Box::new(ColumnFilterPredicate::new("a", ">", AnyValue::Int32(2)));
        let pred2: Box<dyn PredicatePushdown> =
            Box::new(ColumnFilterPredicate::new("b", "<", AnyValue::Int32(45)));

        let and_pred = AndPredicate::new(vec![pred1, pred2]);
        let mask = and_pred.apply(&df).unwrap();

        assert_eq!(mask.sum().unwrap(), 2); // 3,4 satisfy both conditions
    }
}
