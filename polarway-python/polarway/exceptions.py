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


# ─── Lakehouse Exceptions ───


class LakehouseError(PolarwayError):
    """Base exception for Lakehouse operations."""
    pass


class AuthenticationError(LakehouseError):
    """Authentication failed (invalid credentials, expired token, etc.)."""
    pass


class AuthorizationError(LakehouseError):
    """Insufficient permissions for the requested operation."""
    pass


class UserNotFoundError(LakehouseError):
    """User not found in the lakehouse."""
    pass


class UserAlreadyExistsError(LakehouseError):
    """User with this email already exists."""
    pass


class TokenExpiredError(AuthenticationError):
    """JWT token has expired."""
    pass


class VersionNotFoundError(LakehouseError):
    """Requested table version does not exist."""
    pass


class TableNotFoundError(LakehouseError):
    """Delta table not found."""
    pass


class AuditError(LakehouseError):
    """Error writing or querying audit logs."""
    pass
