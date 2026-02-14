import os
from pathlib import Path
from typing import Optional

MAX_RETRIES = 3

class Processor:
    """Processes data files."""

    default_timeout = 30

    def __init__(self, path: str):
        """Initialize with path."""
        self.path = Path(path)
        self.results = []

    def process(self, data: bytes) -> Optional[str]:
        """Process raw data into a string."""
        decoded = data.decode('utf-8')
        result = decoded.strip().upper()
        self.results.append(result)
        return result

@staticmethod
def standalone_function(a: int, b: int) -> int:
    """Add two numbers."""
    return a + b
