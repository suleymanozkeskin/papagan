from ._native import (
    Detailed,
    Detector,
    DetectorBuilder,
    Lang,
    Output,
    WordScore,
    __version__,
    supported_languages,
)

# Runtime-friendly aliases for the typed API surface.
# The accompanying stubs narrow these to Literals / concrete tuple shapes.
LangCode = str
MatchSource = str
LangScore = tuple[str, float]

__all__ = [
    "Detailed",
    "Detector",
    "DetectorBuilder",
    "Lang",
    "LangCode",
    "LangScore",
    "MatchSource",
    "Output",
    "WordScore",
    "__version__",
    "supported_languages",
]
