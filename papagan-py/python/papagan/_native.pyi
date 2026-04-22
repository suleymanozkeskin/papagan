"""Type stubs for the PyO3 extension module.

This file describes the runtime shape of `papagan._native` so type checkers
can resolve the import in `__init__.py`. Keep in sync with `src/lib.rs`.
"""

from typing import Literal

__version__: str

LangCode = Literal["de", "en", "tr", "ru", "fr", "es", "it", "nl", "pt", "pl", "?"]
MatchSource = Literal["dict", "ngram", "unknown"]
LangScore = tuple[LangCode, float]

class Lang:
    De: LangCode
    En: LangCode
    Tr: LangCode
    Ru: LangCode
    Fr: LangCode
    Es: LangCode
    It: LangCode
    Nl: LangCode
    Pt: LangCode
    Pl: LangCode
    Unknown: LangCode
    @staticmethod
    def all_enabled() -> list[LangCode]: ...

class Output:
    def top(self) -> LangScore: ...
    def distribution(self) -> list[LangScore]: ...

class WordScore:
    @property
    def token(self) -> str: ...
    @property
    def scores(self) -> list[LangScore]: ...
    @property
    def source(self) -> MatchSource: ...

class Detailed:
    @property
    def words(self) -> list[WordScore]: ...
    @property
    def aggregate(self) -> Output: ...

class DetectorBuilder:
    def only(self, langs: list[LangCode]) -> "DetectorBuilder": ...
    def unknown_threshold(self, threshold: float) -> "DetectorBuilder": ...
    def parallel_threshold(self, threshold: int) -> "DetectorBuilder": ...
    def build(self) -> "Detector": ...

class Detector:
    def __init__(
        self,
        *,
        only: list[LangCode] | None = None,
        unknown_threshold: float | None = None,
        parallel_threshold: int | None = None,
    ) -> None: ...
    @staticmethod
    def builder() -> DetectorBuilder: ...
    @staticmethod
    def supported_languages() -> list[LangCode]: ...
    def detect(self, input: str) -> Output: ...
    def detect_detailed(self, input: str) -> Detailed: ...
    def detect_batch(self, inputs: list[str]) -> list[Output]: ...
    def detect_detailed_batch(self, inputs: list[str]) -> list[Detailed]: ...

def supported_languages() -> list[LangCode]: ...
