# model.py

from itertools import cycle
from typing import List

# Define a list of Groq model names.
MODELS: List[str] = [
    "whisper-large-v3",
    "whisper-large-v3-turbo",
    "distil-whisper-large-v3-en",
    # Add additional models as needed.
]

# Create a cycle iterator for round-robin selection.
_model_cycle = cycle(MODELS)

def get_next_model() -> str:
    """
    Return the next Groq model from the MODELS list in a round-robin fashion.

    Returns:
        str: The next model name.
    """
    return next(_model_cycle)

if __name__ == "__main__":
    # Example usage: print the first 10 model selections.
    for _ in range(10):
        print(get_next_model())
