"""Public type surface of the `lang_detect` package.

All runtime types live in `_native` (the PyO3 extension module); this stub
re-exports them so `from lang_detect import X` resolves the same type.
"""

from ._native import (
    Detailed as Detailed,
    Detector as Detector,
    DetectorBuilder as DetectorBuilder,
    Lang as Lang,
    LangCode as LangCode,
    LangScore as LangScore,
    MatchSource as MatchSource,
    Output as Output,
    WordScore as WordScore,
    __version__ as __version__,
    supported_languages as supported_languages,
)

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
