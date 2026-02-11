/// Functional programming primitives exposed to Python via PyO3
/// Rust Result<T, E> and Option<T> monads for Python notebooks
use pyo3::prelude::*;
use pyo3::exceptions::PyValueError;
use pyo3::types::PyModule;
use pyo3::Py;
use std::sync::Arc;

/// Result<T, E> monad - Rust-style error handling for Python
#[pyclass(name = "Result", module = "polars.monads")]
#[derive(Clone)]
pub struct MonadResult {
    value: Arc<ResultValue>,
}

enum ResultValue {
    Ok(Py<PyAny>),
    Err(Py<PyAny>),
}

#[pymethods]
impl MonadResult {
    /// Create Ok variant: Result.ok(value)
    #[staticmethod]
    fn ok(value: Py<PyAny>) -> PyResult<Self> {
        Ok(MonadResult {
            value: Arc::new(ResultValue::Ok(value)),
        })
    }

    /// Create Err variant: Result.err(error)
    #[staticmethod]
    fn err(error: Py<PyAny>) -> PyResult<Self> {
        Ok(MonadResult {
            value: Arc::new(ResultValue::Err(error)),
        })
    }

    /// Check if Ok
    fn is_ok(&self) -> bool {
        matches!(*self.value, ResultValue::Ok(_))
    }

    /// Check if Err  
    fn is_err(&self) -> bool {
        matches!(*self.value, ResultValue::Err(_))
    }

    /// Unwrap value or raise exception
    fn unwrap(&self, py: Python) -> PyResult<Py<PyAny>> {
        match &*self.value {
            ResultValue::Ok(v) => Ok(v.clone_ref(py)),
            ResultValue::Err(_) => Err(PyValueError::new_err("Called unwrap() on an Err value")),
        }
    }

    /// Unwrap or return default
    fn unwrap_or(&self, py: Python, default: Py<PyAny>) -> Py<PyAny> {
        match &*self.value {
            ResultValue::Ok(v) => v.clone_ref(py),
            ResultValue::Err(_) => default,
        }
    }

    /// Get Ok value if present
    fn ok_value(&self, py: Python) -> Option<Py<PyAny>> {
        match &*self.value {
            ResultValue::Ok(v) => Some(v.clone_ref(py)),
            ResultValue::Err(_) => None,
        }
    }

    /// Get Err value if present
    fn err_value(&self, py: Python) -> Option<Py<PyAny>> {
        match &*self.value {
            ResultValue::Ok(_) => None,
            ResultValue::Err(e) => Some(e.clone_ref(py)),
        }
    }

    /// Map function over Ok value: result.map(lambda x: x * 2)
    fn map(&self, py: Python, f: Py<PyAny>) -> PyResult<Self> {
        match &*self.value {
            ResultValue::Ok(v) => {
                let result = f.call1(py, (v.clone_ref(py),))?;
                Ok(MonadResult {
                    value: Arc::new(ResultValue::Ok(result)),
                })
            }
            ResultValue::Err(e) => Ok(MonadResult {
                value: Arc::new(ResultValue::Err(e.clone_ref(py))),
            }),
        }
    }

    /// FlatMap for chaining: result.flat_map(lambda x: Result.ok(x * 2))
    fn flat_map(&self, py: Python, f: Py<PyAny>) -> PyResult<Self> {
        match &*self.value {
            ResultValue::Ok(v) => {
                let result_obj = f.call1(py, (v.clone_ref(py),))?;
                let result: MonadResult = result_obj.extract(py)?;
                Ok(result)
            }
            ResultValue::Err(e) => Ok(MonadResult {
                value: Arc::new(ResultValue::Err(e.clone_ref(py))),
            }),
        }
    }

    /// Alias for flat_map - railway-oriented programming style
    fn and_then(&self, py: Python, f: Py<PyAny>) -> PyResult<Self> {
        self.flat_map(py, f)
    }

    /// Pattern matching: result.match_result(on_ok=lambda x: x, on_err=lambda e: 0)
    fn match_result(&self, py: Python, on_ok: Py<PyAny>, on_err: Py<PyAny>) -> PyResult<Py<PyAny>> {
        match &*self.value {
            ResultValue::Ok(v) => on_ok.call1(py, (v.clone_ref(py),)),
            ResultValue::Err(e) => on_err.call1(py, (e.clone_ref(py),)),
        }
    }

    fn __repr__(&self, _py: Python) -> String {
        match &*self.value {
            ResultValue::Ok(v) => format!("Result.Ok({:?})", v.as_ptr()),
            ResultValue::Err(e) => format!("Result.Err({:?})", e.as_ptr()),
        }
    }
}

/// Option<T> monad - Safe handling of nullable values
#[pyclass(name = "Option", module = "polars.monads")]
#[derive(Clone)]
pub struct MonadOption {
    value: Arc<OptionValue>,
}

enum OptionValue {
    Some(Py<PyAny>),
    Nothing,
}

#[pymethods]
impl MonadOption {
    /// Create Some variant: Option.some(value)
    #[staticmethod]
    fn some(value: Py<PyAny>) -> PyResult<Self> {
        Ok(MonadOption {
            value: Arc::new(OptionValue::Some(value)),
        })
    }

    /// Create Nothing variant: Option.nothing()
    #[staticmethod]
    fn nothing() -> PyResult<Self> {
        Ok(MonadOption {
            value: Arc::new(OptionValue::Nothing),
        })
    }

    /// Check if Some
    fn is_some(&self) -> bool {
        matches!(*self.value, OptionValue::Some(_))
    }

    /// Check if Nothing
    fn is_none(&self) -> bool {
        matches!(*self.value, OptionValue::Nothing)
    }

    /// Unwrap value or raise exception
    fn unwrap(&self, py: Python) -> PyResult<Py<PyAny>> {
        match &*self.value {
            OptionValue::Some(v) => Ok(v.clone_ref(py)),
            OptionValue::Nothing => Err(PyValueError::new_err("Called unwrap() on Nothing")),
        }
    }

    /// Unwrap or return default
    fn unwrap_or(&self, py: Python, default: Py<PyAny>) -> Py<PyAny> {
        match &*self.value {
            OptionValue::Some(v) => v.clone_ref(py),
            OptionValue::Nothing => default,
        }
    }

    /// Get inner value if present
    fn get(&self, py: Python) -> Option<Py<PyAny>> {
        match &*self.value {
            OptionValue::Some(v) => Some(v.clone_ref(py)),
            OptionValue::Nothing => None,
        }
    }

    /// Map function over Some value
    fn map(&self, py: Python, f: Py<PyAny>) -> PyResult<Self> {
        match &*self.value {
            OptionValue::Some(v) => {
                let result = f.call1(py, (v.clone_ref(py),))?;
                Ok(MonadOption {
                    value: Arc::new(OptionValue::Some(result)),
                })
            }
            OptionValue::Nothing => Ok(MonadOption {
                value: Arc::new(OptionValue::Nothing),
            }),
        }
    }

    /// FlatMap for chaining
    fn flat_map(&self, py: Python, f: Py<PyAny>) -> PyResult<Self> {
        match &*self.value {
            OptionValue::Some(v) => {
                let result_obj = f.call1(py, (v.clone_ref(py),))?;
                let opt: MonadOption = result_obj.extract(py)?;
                Ok(opt)
            }
            OptionValue::Nothing => Ok(MonadOption {
                value: Arc::new(OptionValue::Nothing),
            }),
        }
    }

    /// Filter by predicate
    fn filter(&self, py: Python, predicate: Py<PyAny>) -> PyResult<Self> {
        match &*self.value {
            OptionValue::Some(v) => {
                let result: bool = predicate.call1(py, (v.clone_ref(py),))?.extract(py)?;
                if result {
                    Ok(MonadOption {
                        value: Arc::new(OptionValue::Some(v.clone_ref(py))),
                    })
                } else {
                    Ok(MonadOption {
                        value: Arc::new(OptionValue::Nothing),
                    })
                }
            }
            OptionValue::Nothing => Ok(MonadOption {
                value: Arc::new(OptionValue::Nothing),
            }),
        }
    }

    /// Pattern matching
    fn match_option(&self, py: Python, on_some: Py<PyAny>, on_nothing: Py<PyAny>) -> PyResult<Py<PyAny>> {
        match &*self.value {
            OptionValue::Some(v) => on_some.call1(py, (v.clone_ref(py),)),
            OptionValue::Nothing => on_nothing.call0(py),
        }
    }

    fn __repr__(&self, _py: Python) -> String {
        match &*self.value {
            OptionValue::Some(v) => format!("Option.Some({:?})", v.as_ptr()),
            OptionValue::Nothing => "Option.Nothing".to_string(),
        }
    }
}

/// Thunk<T> - Lazy evaluation with memoization
#[pyclass(name = "Thunk", module = "polars.monads")]
pub struct MonadThunk {
    computation: Py<PyAny>,
    cached: std::sync::Mutex<Option<Py<PyAny>>>,
}

#[pymethods]
impl MonadThunk {
    /// Create new thunk: Thunk(lambda: expensive_computation())
    #[new]
    fn new(computation: Py<PyAny>) -> Self {
        MonadThunk {
            computation,
            cached: std::sync::Mutex::new(None),
        }
    }

    /// Force evaluation (memoized)
    fn force(&self, py: Python) -> PyResult<Py<PyAny>> {
        let mut cache = self.cached.lock().unwrap();
        if let Some(ref cached_value) = *cache {
            return Ok(cached_value.clone_ref(py));
        }

        let result = self.computation.call0(py)?;
        *cache = Some(result.clone_ref(py));
        Ok(result)
    }

    /// Check if already evaluated
    fn is_evaluated(&self) -> bool {
        self.cached.lock().unwrap().is_some()
    }

    /// Map over thunk result (lazy) - creates new lazy computation
    fn map(&self, py: Python, f: Py<PyAny>) -> PyResult<Self> {
        let computation = self.computation.clone_ref(py);
        let f_copy = f.clone_ref(py);
        
        // Create a new lazy computation by wrapping both in a lambda
        // We'll just compose them manually when force() is called
        let new_comp = PyModule::from_code(
            py,
            c"def compose(c, f): return lambda: f(c())\nresult = compose",
            c"<string>",
            c"<string>",
        )?;
        
        let compose_fn = new_comp.getattr("result")?;
        let new_computation = compose_fn.call1((computation, f_copy))?;

        Ok(MonadThunk {
            computation: new_computation.into(),
            cached: std::sync::Mutex::new(None),
        })
    }

    fn __repr__(&self) -> String {
        if self.is_evaluated() {
            "Thunk(evaluated)".to_string()
        } else {
            "Thunk(pending)".to_string()
        }
    }
}

/// Register monads submodule with polars
#[pymodule]
pub fn monads(m: &Bound<PyModule>) -> PyResult<()> {
    m.add_class::<MonadResult>()?;
    m.add_class::<MonadOption>()?;
    m.add_class::<MonadThunk>()?;
    Ok(())
}
