"""
Functional Programming Primitives for Python
Rust-inspired Result, Option, and Thunk monads

This module provides functional programming patterns that
mirror Rust's error handling and lazy evaluation.
"""

from typing import TypeVar, Generic, Callable, Optional, Any
from dataclasses import dataclass

T = TypeVar('T')
E = TypeVar('E')
U = TypeVar('U')


@dataclass
class Result(Generic[T, E]):
    """
    Result<T, E> monad - Rust-style error handling
    
    Usage:
        result = Result.ok(42)
        result.map(lambda x: x * 2).unwrap()  # 84
        
        error = Result.err("failed")
        error.unwrap_or(0)  # 0
    """
    _value: Any
    _is_ok: bool
    
    @classmethod
    def ok(cls, value: T) -> 'Result[T, E]':
        """Create Ok variant"""
        return cls(_value=value, _is_ok=True)
    
    @classmethod
    def err(cls, error: E) -> 'Result[T, E]':
        """Create Err variant"""
        return cls(_value=error, _is_ok=False)
    
    def is_ok(self) -> bool:
        """Check if Ok"""
        return self._is_ok
    
    def is_err(self) -> bool:
        """Check if Err"""
        return not self._is_ok
    
    def unwrap(self) -> T:
        """Unwrap value or raise exception"""
        if self._is_ok:
            return self._value
        raise ValueError(f"Called unwrap() on Err: {self._value}")
    
    def unwrap_or(self, default: T) -> T:
        """Unwrap or return default"""
        return self._value if self._is_ok else default
    
    def ok_value(self) -> Optional[T]:
        """Get Ok value if present"""
        return self._value if self._is_ok else None
    
    def err_value(self) -> Optional[E]:
        """Get Err value if present"""
        return None if self._is_ok else self._value
    
    def map(self, f: Callable[[T], U]) -> 'Result[U, E]':
        """Map function over Ok value"""
        if self._is_ok:
            return Result.ok(f(self._value))
        return Result.err(self._value)
    
    def flat_map(self, f: Callable[[T], 'Result[U, E]']) -> 'Result[U, E]':
        """FlatMap for chaining Results"""
        if self._is_ok:
            return f(self._value)
        return Result.err(self._value)
    
    def and_then(self, f: Callable[[T], 'Result[U, E]']) -> 'Result[U, E]':
        """Alias for flat_map - railway-oriented programming"""
        return self.flat_map(f)
    
    def match(self, on_ok: Callable[[T], U], on_err: Callable[[E], U]) -> U:
        """Pattern matching"""
        return on_ok(self._value) if self._is_ok else on_err(self._value)
    
    def __repr__(self) -> str:
        if self._is_ok:
            return f"Result.Ok({repr(self._value)})"
        return f"Result.Err({repr(self._value)})"


@dataclass
class Option(Generic[T]):
    """
    Option<T> monad - Safe handling of nullable values
    
    Usage:
        opt = Option.some(10)
        opt.map(lambda x: x * 2).unwrap()  # 20
        
        nothing = Option.nothing()
        nothing.unwrap_or(0)  # 0
    """
    _value: Optional[T]
    
    @classmethod
    def some(cls, value: T) -> 'Option[T]':
        """Create Some variant"""
        return cls(_value=value)
    
    @classmethod
    def nothing(cls) -> 'Option[T]':
        """Create Nothing variant"""
        return cls(_value=None)
    
    def is_some(self) -> bool:
        """Check if Some"""
        return self._value is not None
    
    def is_none(self) -> bool:
        """Check if Nothing"""
        return self._value is None
    
    def unwrap(self) -> T:
        """Unwrap value or raise exception"""
        if self._value is not None:
            return self._value
        raise ValueError("Called unwrap() on Nothing")
    
    def unwrap_or(self, default: T) -> T:
        """Unwrap or return default"""
        return self._value if self._value is not None else default
    
    def get(self) -> Optional[T]:
        """Get inner value"""
        return self._value
    
    def map(self, f: Callable[[T], U]) -> 'Option[U]':
        """Map function over Some value"""
        if self._value is not None:
            return Option.some(f(self._value))
        return Option.nothing()
    
    def flat_map(self, f: Callable[[T], 'Option[U]']) -> 'Option[U]':
        """FlatMap for chaining Options"""
        if self._value is not None:
            return f(self._value)
        return Option.nothing()
    
    def filter(self, predicate: Callable[[T], bool]) -> 'Option[T]':
        """Filter by predicate"""
        if self._value is not None and predicate(self._value):
            return Option.some(self._value)
        return Option.nothing()
    
    def match(self, on_some: Callable[[T], U], on_nothing: Callable[[], U]) -> U:
        """Pattern matching"""
        return on_some(self._value) if self._value is not None else on_nothing()
    
    def __repr__(self) -> str:
        if self._value is not None:
            return f"Option.Some({repr(self._value)})"
        return "Option.Nothing"


@dataclass
class Thunk(Generic[T]):
    """
    Thunk<T> - Lazy evaluation with memoization
    
    Usage:
        thunk = Thunk(lambda: expensive_computation())
        thunk.force()  # Evaluates once
        thunk.force()  # Returns cached value
    """
    _computation: Callable[[], T]
    _cached: Optional[T] = None
    _evaluated: bool = False
    
    def force(self) -> T:
        """Force evaluation (memoized)"""
        if not self._evaluated:
            self._cached = self._computation()
            self._evaluated = True
        return self._cached
    
    def is_evaluated(self) -> bool:
        """Check if already evaluated"""
        return self._evaluated
    
    def map(self, f: Callable[[T], U]) -> 'Thunk[U]':
        """Map over thunk result (lazy)"""
        return Thunk(lambda: f(self.force()))
    
    def __repr__(self) -> str:
        if self._evaluated:
            return f"Thunk(evaluated -> {repr(self._cached)})"
        return "Thunk(pending)"


# Railway-Oriented Programming utilities
def safe_divide(a: float, b: float) -> Result[float, str]:
    """Safe division returning Result"""
    if b == 0:
        return Result.err("Division by zero")
    return Result.ok(a / b)


def safe_sqrt(x: float) -> Result[float, str]:
    """Safe square root returning Result"""
    if x < 0:
        return Result.err("Negative number")
    return Result.ok(x ** 0.5)


def safe_access(data: dict, key: str) -> Option:
    """Safe dictionary access returning Option"""
    value = data.get(key)
    return Option.some(value) if value is not None else Option.nothing()


# Example: Railway-oriented pipeline
def safe_compute_pipeline(x: float, y: float) -> Result[float, str]:
    """
    Demonstration of railway-oriented programming:
    - Divide x by y
    - Take square root
    - Multiply by 2
    
    Any error propagates through the chain
    """
    return (safe_divide(x, y)
            .and_then(safe_sqrt)
            .map(lambda v: v * 2))


__all__ = [
    'Result',
    'Option', 
    'Thunk',
    'safe_divide',
    'safe_sqrt',
    'safe_access',
    'safe_compute_pipeline',
]
