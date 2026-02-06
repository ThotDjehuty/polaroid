"""Polarway exception classes."""


class PolarwayError(Exception):
    """Base exception for Polarway errors."""
    pass


class ConnectionError(PolarwayError):
    """Error connecting to Polarway server."""
    pass


class ServerError(PolarwayError):
    """Error from Polarway server."""
    pass


class HandleError(PolarwayError):
    """Error related to DataFrame handles."""
    pass


class HandleNotFoundError(HandleError):
    """Handle not found on server."""
    pass


class HandleExpiredError(HandleError):
    """Handle has expired (TTL exceeded)."""
    pass


class FileNotFoundError(PolarwayError):
    """File not found."""
    pass


class SchemaError(PolarwayError):
    """Schema-related error."""
    pass


class TypeMismatchError(SchemaError):
    """Type mismatch in operation."""
    pass


class OperationError(PolarwayError):
    """Error performing DataFrame operation."""
    pass


class TimeoutError(PolarwayError):
    """Operation timed out."""
    pass
