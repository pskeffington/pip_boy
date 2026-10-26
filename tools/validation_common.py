"""Small shared assertions for the offline, standard-library validation tools."""


def require(condition, message):
    """Fail closed with a readable diagnostic when a repository invariant drifts."""
    if not condition:
        raise ValueError(message)
