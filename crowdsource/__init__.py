__version__ = "0.1.0"

# Re-export the compiled pyo3 client so `from crowdsource import Client` works.
# Guarded so the pure-Python CLI helpers remain importable even when the native
# extension hasn't been built yet (e.g. during isolated unit tests).
try:
    from .crowdsource import Client  # noqa: F401
except ImportError:  # pragma: no cover - extension not built
    Client = None  # type: ignore
