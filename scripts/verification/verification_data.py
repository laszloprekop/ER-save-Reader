"""
Data structures for event flag verification.

These structures track the verification status of each flag, including:
- Calculated offsets from various formulas
- Empirical offsets discovered from save diffing
- Manual verification status from user testing
- Confidence levels and evidence sources
"""

from dataclasses import dataclass, field
from enum import Enum
from typing import Optional, Dict, List, Any
from datetime import datetime
import json


class VerificationStatus(Enum):
    """Status of a flag's verification."""
    PROVEN = "proven"           # Formula matches empirical evidence
    DISPROVEN = "disproven"     # Formula does NOT match empirical evidence
    UNVERIFIED = "unverified"   # No empirical evidence yet
    UNTRACKABLE = "untrackable" # Flag cannot be tracked (e.g., localId >= 7000)
    UNKNOWN = "unknown"         # Flag status is unknown


class FlagCategory(Enum):
    """Categories of event flags based on their purpose and flag range."""
    GRACE = "Grace"
    BOSS_DEFEAT = "Boss Defeat"
    GREAT_BOSS_DEFEAT = "Great Boss Defeat"
    FIELD_BOSS_DEFEAT = "Field Boss Defeat"
    WORLD_PICKUP = "World Pickup"
    DUNGEON_PICKUP = "Dungeon Pickup"
    DLC_PICKUP = "DLC Pickup"
    COOKBOOK = "Cookbook"
    WHETBLADE = "Whetblade"
    MAP_FRAGMENT = "Map Fragment"
    PROGRESSION = "Progression"
    NPC = "NPC"
    MERCHANT = "Merchant"
    STAKE_OF_MARIKA = "Stake of Marika"
    SPIRIT_SPRING = "Spirit Spring"
    BOSS_ARENA = "Boss Arena"
    SHOP_STOCK = "Shop Stock"
    SHOP_UNLOCK = "Shop Unlock"
    REMEMBRANCE = "Remembrance"
    POT_UPGRADE = "Pot Upgrade"
    CRYSTAL_TEAR = "Crystal Tear"
    GREAT_RUNE = "Great Rune Possession"
    MAUSOLEUM = "Mausoleum Duplication"
    UNKNOWN = "Unknown"


@dataclass
class FormulaResult:
    """Result of applying a formula to calculate flag offset."""
    formula_name: str           # "block", "tile", "dungeon"
    byte_offset: Optional[int]  # Calculated byte offset within event flags
    bit_position: Optional[int] # Calculated bit position (0-7)
    is_valid: bool              # Whether formula could be applied
    error_message: Optional[str] = None


@dataclass
class EmpiricalEvidence:
    """Evidence from save file diffing or manual testing."""
    source: str                   # "diff", "manual", "slot_comparison"
    byte_offset: Optional[int]    # Discovered byte offset
    bit_position: Optional[int]   # Discovered bit position
    save_file: Optional[str]      # Which save file was used
    slot_index: Optional[int]     # Which character slot
    confidence: float             # 0.0 - 1.0
    notes: Optional[str] = None


@dataclass
class FlagVerification:
    """Complete verification record for a single event flag."""
    flag_id: int
    name: str
    category: FlagCategory
    region: str

    # Formula calculation results
    formula_results: Dict[str, FormulaResult] = field(default_factory=dict)

    # Empirical evidence (from diffing or manual testing)
    empirical_evidence: List[EmpiricalEvidence] = field(default_factory=list)

    # Manual completion status
    manual_completion: Optional[bool] = None  # User says completed
    auto_completion: Optional[bool] = None    # Formula says completed
    matches: Optional[bool] = None            # manual == auto

    # Overall verification status
    status: VerificationStatus = VerificationStatus.UNKNOWN
    best_offset: Optional[int] = None         # Most reliable offset
    best_bit: Optional[int] = None            # Most reliable bit position
    confidence: float = 0.0                   # 0.0 - 1.0

    # Metadata
    source_file: Optional[str] = None
    source_row_id: Optional[int] = None
    coordinates: Optional[Dict[str, float]] = None

    def add_formula_result(self, result: FormulaResult):
        """Add a formula calculation result."""
        self.formula_results[result.formula_name] = result

    def add_empirical_evidence(self, evidence: EmpiricalEvidence):
        """Add empirical evidence from diffing or testing."""
        self.empirical_evidence.append(evidence)

    def determine_status(self):
        """Determine verification status based on all evidence."""
        # Check if flag is structurally untrackable
        if self.flag_id >= 1_000_000_000:  # 10-digit flag
            local_id = self.flag_id % 10000
            if local_id >= 7000:
                self.status = VerificationStatus.UNTRACKABLE
                self.confidence = 1.0
                return

        # Check empirical evidence vs formula results
        if self.empirical_evidence:
            best_empirical = max(self.empirical_evidence, key=lambda e: e.confidence)

            # Check if any formula matches empirical evidence
            formula_matches = False
            for name, result in self.formula_results.items():
                if result.is_valid and result.byte_offset == best_empirical.byte_offset:
                    if result.bit_position == best_empirical.bit_position:
                        formula_matches = True
                        break

            if formula_matches:
                self.status = VerificationStatus.PROVEN
                self.best_offset = best_empirical.byte_offset
                self.best_bit = best_empirical.bit_position
                self.confidence = best_empirical.confidence
            else:
                # Empirical evidence differs from formula
                self.status = VerificationStatus.DISPROVEN
                self.best_offset = best_empirical.byte_offset
                self.best_bit = best_empirical.bit_position
                self.confidence = best_empirical.confidence
        elif self.formula_results:
            # No empirical evidence, use formula
            self.status = VerificationStatus.UNVERIFIED
            # Prefer certain formulas in order
            for formula_name in ["block", "tile", "dungeon"]:
                if formula_name in self.formula_results:
                    result = self.formula_results[formula_name]
                    if result.is_valid:
                        self.best_offset = result.byte_offset
                        self.best_bit = result.bit_position
                        self.confidence = 0.5  # Medium confidence without evidence
                        break
        else:
            self.status = VerificationStatus.UNKNOWN
            self.confidence = 0.0

    def to_dict(self) -> Dict[str, Any]:
        """Convert to dictionary for JSON export."""
        return {
            "flag_id": self.flag_id,
            "name": self.name,
            "category": self.category.value,
            "region": self.region,
            "status": self.status.value,
            "offset": self.best_offset,
            "bit": self.best_bit,
            "confidence": self.confidence,
            "manual_completion": self.manual_completion,
            "auto_completion": self.auto_completion,
            "matches": self.matches,
            "formula_results": {
                name: {
                    "offset": r.byte_offset,
                    "bit": r.bit_position,
                    "valid": r.is_valid,
                    "error": r.error_message
                }
                for name, r in self.formula_results.items()
            },
            "evidence_count": len(self.empirical_evidence),
            "source_file": self.source_file,
            "source_row_id": self.source_row_id,
            "coordinates": self.coordinates,
        }


@dataclass
class VerificationReport:
    """Complete verification report for all tested flags."""

    generated_date: str = field(default_factory=lambda: datetime.now().isoformat())
    verification_method: str = "empirical_multi_save"

    # All verified flags
    flags: Dict[int, FlagVerification] = field(default_factory=dict)

    # Summary statistics
    total_flags: int = 0
    proven_count: int = 0
    disproven_count: int = 0
    unverified_count: int = 0
    untrackable_count: int = 0

    # Category-specific summaries
    category_stats: Dict[str, Dict[str, int]] = field(default_factory=dict)

    # Formula reliability
    formula_stats: Dict[str, Dict[str, int]] = field(default_factory=dict)

    # Known good formulas
    block_bases: Dict[int, Dict[str, Any]] = field(default_factory=dict)
    tile_formula_config: Dict[str, Any] = field(default_factory=dict)
    dungeon_formula_config: Dict[str, Any] = field(default_factory=dict)

    # List of untrackable categories
    untrackable_categories: List[str] = field(default_factory=list)

    def add_flag(self, flag: FlagVerification):
        """Add a flag verification result."""
        self.flags[flag.flag_id] = flag

    def compute_statistics(self):
        """Compute summary statistics from all flags."""
        self.total_flags = len(self.flags)
        self.proven_count = sum(1 for f in self.flags.values() if f.status == VerificationStatus.PROVEN)
        self.disproven_count = sum(1 for f in self.flags.values() if f.status == VerificationStatus.DISPROVEN)
        self.unverified_count = sum(1 for f in self.flags.values() if f.status == VerificationStatus.UNVERIFIED)
        self.untrackable_count = sum(1 for f in self.flags.values() if f.status == VerificationStatus.UNTRACKABLE)

        # Category statistics
        self.category_stats = {}
        for flag in self.flags.values():
            cat = flag.category.value
            if cat not in self.category_stats:
                self.category_stats[cat] = {"total": 0, "proven": 0, "disproven": 0, "unverified": 0, "untrackable": 0, "unknown": 0}
            self.category_stats[cat]["total"] += 1
            status_key = flag.status.value
            if status_key in self.category_stats[cat]:
                self.category_stats[cat][status_key] += 1
            else:
                self.category_stats[cat][status_key] = 1

        # Formula statistics
        self.formula_stats = {}
        for flag in self.flags.values():
            for formula_name, result in flag.formula_results.items():
                if formula_name not in self.formula_stats:
                    self.formula_stats[formula_name] = {"total": 0, "correct": 0, "incorrect": 0, "invalid": 0}
                self.formula_stats[formula_name]["total"] += 1

                if not result.is_valid:
                    self.formula_stats[formula_name]["invalid"] += 1
                elif flag.status == VerificationStatus.PROVEN:
                    # Check if this formula produced the correct result
                    if result.byte_offset == flag.best_offset and result.bit_position == flag.best_bit:
                        self.formula_stats[formula_name]["correct"] += 1
                    else:
                        self.formula_stats[formula_name]["incorrect"] += 1
                elif flag.status == VerificationStatus.DISPROVEN:
                    self.formula_stats[formula_name]["incorrect"] += 1

        # Identify untrackable categories
        self.untrackable_categories = [
            cat for cat, stats in self.category_stats.items()
            if stats["untrackable"] > 0 and stats["untrackable"] / stats["total"] > 0.5
        ]

    def export_ground_truth(self, filepath: str):
        """Export ground truth to JSON file."""
        self.compute_statistics()

        # Build verified flags dict (only proven and high-confidence)
        verified_flags = {}
        for flag_id, flag in self.flags.items():
            if flag.status == VerificationStatus.PROVEN or (
                flag.status == VerificationStatus.UNVERIFIED and flag.confidence >= 0.7
            ):
                if flag.best_offset is not None and flag.best_bit is not None:
                    verified_flags[str(flag_id)] = {
                        "offset": flag.best_offset,
                        "bit": flag.best_bit,
                        "name": flag.name,
                        "category": flag.category.value,
                        "status": flag.status.value,
                        "confidence": flag.confidence
                    }

        output = {
            "metadata": {
                "generated_date": self.generated_date,
                "verification_method": self.verification_method,
                "total_flags_tested": self.total_flags,
                "proven_count": self.proven_count,
                "disproven_count": self.disproven_count,
                "unverified_count": self.unverified_count,
                "untrackable_count": self.untrackable_count,
            },
            "summary": {
                "by_category": self.category_stats,
                "by_formula": self.formula_stats,
            },
            "verified_flags": verified_flags,
            "formulas": {
                "block_bases": self.block_bases,
                "tile_formula": self.tile_formula_config,
                "dungeon_formula": self.dungeon_formula_config,
            },
            "untrackable_categories": self.untrackable_categories,
            "all_flags": [f.to_dict() for f in self.flags.values()],
        }

        with open(filepath, 'w', encoding='utf-8') as f:
            json.dump(output, f, indent=2, ensure_ascii=False)

    def print_summary(self):
        """Print a summary of verification results."""
        self.compute_statistics()

        print("=" * 70)
        print("VERIFICATION REPORT SUMMARY")
        print("=" * 70)
        print(f"\nTotal flags tested: {self.total_flags}")
        print(f"  PROVEN:      {self.proven_count:5d} ({100*self.proven_count/self.total_flags:.1f}%)")
        print(f"  DISPROVEN:   {self.disproven_count:5d} ({100*self.disproven_count/self.total_flags:.1f}%)")
        print(f"  UNVERIFIED:  {self.unverified_count:5d} ({100*self.unverified_count/self.total_flags:.1f}%)")
        print(f"  UNTRACKABLE: {self.untrackable_count:5d} ({100*self.untrackable_count/self.total_flags:.1f}%)")

        print("\n" + "-" * 70)
        print("BY CATEGORY")
        print("-" * 70)
        print(f"{'Category':<25} {'Total':>8} {'Proven':>8} {'Disproven':>10} {'Rate':>8}")
        print("-" * 70)
        for cat, stats in sorted(self.category_stats.items()):
            rate = 100 * stats.get("proven", 0) / stats["total"] if stats["total"] > 0 else 0
            print(f"{cat:<25} {stats['total']:>8} {stats.get('proven', 0):>8} {stats.get('disproven', 0):>10} {rate:>7.1f}%")

        if self.formula_stats:
            print("\n" + "-" * 70)
            print("BY FORMULA")
            print("-" * 70)
            print(f"{'Formula':<15} {'Total':>8} {'Correct':>10} {'Incorrect':>10} {'Rate':>8}")
            print("-" * 70)
            for formula, stats in sorted(self.formula_stats.items()):
                rate = 100 * stats.get("correct", 0) / stats["total"] if stats["total"] > 0 else 0
                print(f"{formula:<15} {stats['total']:>8} {stats.get('correct', 0):>10} {stats.get('incorrect', 0):>10} {rate:>7.1f}%")

        if self.untrackable_categories:
            print("\n" + "-" * 70)
            print("UNTRACKABLE CATEGORIES (>50% untrackable)")
            print("-" * 70)
            for cat in self.untrackable_categories:
                print(f"  - {cat}")

        print("\n" + "=" * 70)
