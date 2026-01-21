"""
Event Flag Verification Framework

A systematic tool to verify event flag calculations against actual save file data,
establish ground truth for Elden Ring save file parsing, and generate reliable
offset tables for autocompletion tools.

Modules:
- save_parser: Structural save file parsing with GaItems size calculation
- flag_formulas: All known flag calculation formulas (block, tile, dungeon)
- diff_analyzer: Before/after save comparison for empirical offset discovery
- verification_data: Data structures for verification results
- report_generator: Generate verification reports and ground truth JSON

Usage:
    from verification import run_verification

    result = run_verification(
        save_file="path/to/ER0000.sl2",
        extracted_flags="path/to/extracted_event_flags.json",
        manual_completions="path/to/flag-correlation-candidates.jsonl"
    )
    result.export_ground_truth("ground_truth_offsets.json")
"""

__version__ = "1.0.0"
__author__ = "ER-Save-Editor Project"

from .verification_data import (
    FlagVerification,
    VerificationStatus,
    FlagCategory,
    VerificationReport,
)
from .save_parser import SaveParser, SlotData
from .flag_formulas import FlagFormulas
from .diff_analyzer import DiffAnalyzer

__all__ = [
    "FlagVerification",
    "VerificationStatus",
    "FlagCategory",
    "VerificationReport",
    "SaveParser",
    "SlotData",
    "FlagFormulas",
    "DiffAnalyzer",
]
