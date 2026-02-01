#!/usr/bin/env python3
"""
Case-Based Verification System

A Case is a structured hypothesis about a flag's location that must survive
multiple rounds of defense and challenge before being accepted as verified.

Usage:
    from case_manager import CaseManager, VerificationCase

    manager = CaseManager()
    case = manager.create_case(
        flag_id=520000,
        item_name="Lhutel the Headless",
        category="spirit_ash",
        hypothesis=FlagHypothesis(byte_offset=1341, bit_position=7)
    )

    # Run defense/challenge cycles
    manager.defend(case)
    manager.challenge(case)

    # Check result
    if case.status == CaseStatus.VERIFIED:
        print(f"Verified: {case.flag_id} at offset {case.hypothesis.byte_offset}")
"""

import json
import struct
from collections import defaultdict
from dataclasses import dataclass, field
from datetime import datetime
from enum import Enum
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

import sys
PROJECT_ROOT = Path(__file__).parent.parent.parent
sys.path.insert(0, str(PROJECT_ROOT))

from scripts.verification.save_parser import SaveParser
from scripts.verification.ground_truth_loader import (
    load_block_bases,
    calculate_block_offset,
)


# =============================================================================
# ENUMS AND CONSTANTS
# =============================================================================

class CaseStatus(Enum):
    """Status of a verification case."""
    OPEN = "open"                    # Just created
    DEFENDING = "defending"          # Gathering supporting evidence
    CHALLENGING = "challenging"      # Attempting to disprove
    VERIFIED = "verified"            # Passed all challenges
    PARTIAL = "partial"              # Verified with caveats
    REJECTED = "rejected"            # Disproven
    INCONCLUSIVE = "inconclusive"    # Not enough evidence


CONFIDENCE_WEIGHTS = {
    "inventory_present": 0.30,
    "flag_detected": 0.25,
    "manual_completion": 0.20,
    "cross_slot_differential": 0.15,
    "chain_anchor": 0.10,
    "temporal_consistency": 0.10,
    "formula_consistency": 0.05,
}

THRESHOLDS = {
    "verified": 0.85,
    "high_confidence": 0.70,
    "medium_confidence": 0.50,
    "low_confidence": 0.30,
}


# =============================================================================
# DATA CLASSES
# =============================================================================

@dataclass
class EvidenceSource:
    """Track where evidence came from."""
    save_file: str
    slot_index: int
    evidence_type: str  # "inventory", "flag_state", "differential", "temporal"
    method: str = ""    # Script/function that generated this
    timestamp: str = ""

    def __post_init__(self):
        if not self.timestamp:
            self.timestamp = datetime.now().isoformat()

    def __str__(self):
        return f"{Path(self.save_file).name}:slot{self.slot_index}:{self.evidence_type}"


@dataclass
class FlagHypothesis:
    """A proposed location for a flag."""
    byte_offset: int
    bit_position: int
    implied_base: Optional[int] = None
    block_start: Optional[int] = None

    def __str__(self):
        return f"offset={self.byte_offset}, bit={self.bit_position}"


@dataclass
class CaseEvidence:
    """A single piece of evidence for/against a case."""
    evidence_type: str
    source: EvidenceSource
    supports_hypothesis: bool
    confidence_contribution: float

    # Observed data
    byte_offset: int = 0
    bit_position: int = 0
    observed_value: int = 0
    expected_value: Optional[int] = None

    # Formula tracking (for feedback loop)
    formula_type: str = ""      # "block", "tile", "dungeon"
    base_source: str = ""       # "ground_truth", "discovered", "manual"
    base_offset: Optional[int] = None  # The base offset used in calculation

    # Context
    slot_context: Dict[str, Any] = field(default_factory=dict)
    notes: str = ""


@dataclass
class CaseChallenge:
    """An attempt to disprove a case."""
    challenge_type: str
    description: str
    result: str  # "survived", "failed", "inconclusive"

    test_method: str = ""
    test_data: Dict[str, Any] = field(default_factory=dict)

    disproves_hypothesis: bool = False
    alternative_hypothesis: Optional[FlagHypothesis] = None
    notes: str = ""


@dataclass
class VerificationCase:
    """A structured hypothesis about a flag's location."""
    # Identity
    case_id: str
    flag_id: int
    item_name: str
    category: str
    item_id: Optional[int] = None  # Game item ID if applicable

    # Hypothesis
    hypothesis: Optional[FlagHypothesis] = None
    block_start: int = 0
    formula_type: str = "block"

    # Evidence
    evidence: List[CaseEvidence] = field(default_factory=list)
    supporting_sources: List[EvidenceSource] = field(default_factory=list)

    # Challenges
    challenges: List[CaseChallenge] = field(default_factory=list)
    surviving_challenges: int = 0

    # Status
    status: CaseStatus = CaseStatus.OPEN
    confidence: float = 0.0
    iterations: int = 0

    # Normalized confidence tracking (prevents score inflation)
    _evidence_counts: Dict[str, int] = field(default_factory=lambda: defaultdict(int))
    _evidence_contributions: Dict[str, float] = field(default_factory=lambda: defaultdict(float))

    # Metadata
    created_at: str = ""
    last_updated: str = ""
    notes: List[str] = field(default_factory=list)

    # Confidence caps per evidence type (prevents inflation from repeated evidence)
    CONFIDENCE_CAPS = {
        "inventory_presence": 0.35,
        "differential": 0.25,
        "cross_save": 0.20,
        "chain_anchor": 0.15,
        "temporal": 0.15,
    }

    # Diminishing factor for repeated evidence of same type
    DIMINISHING_FACTOR = 0.5

    def __post_init__(self):
        if not self.created_at:
            self.created_at = datetime.now().isoformat()
        if not self.last_updated:
            self.last_updated = self.created_at

    def add_evidence(self, evidence: CaseEvidence):
        """Add evidence with normalized confidence (diminishing returns)."""
        self.evidence.append(evidence)

        # Apply diminishing returns
        ev_type = evidence.evidence_type
        count = self._evidence_counts[ev_type]
        base_contrib = evidence.confidence_contribution

        # Calculate diminished contribution
        diminished = base_contrib * (self.DIMINISHING_FACTOR ** count)

        # Apply cap
        cap = self.CONFIDENCE_CAPS.get(ev_type, 0.50)
        current = self._evidence_contributions[ev_type]

        if evidence.supports_hypothesis:
            actual_contrib = min(diminished, max(0, cap - current))
            self._evidence_contributions[ev_type] += actual_contrib
        else:
            # Negative evidence doesn't diminish as quickly
            actual_contrib = diminished * 0.7
            self._evidence_contributions[ev_type] -= actual_contrib

        self._evidence_counts[ev_type] += 1
        self.recalculate_confidence()
        self.last_updated = datetime.now().isoformat()

    def add_challenge(self, challenge: CaseChallenge):
        """Add challenge result and update status."""
        self.challenges.append(challenge)
        if not challenge.disproves_hypothesis:
            self.surviving_challenges += 1
        self.last_updated = datetime.now().isoformat()

    def recalculate_confidence(self):
        """Recalculate confidence from normalized contributions."""
        total = sum(self._evidence_contributions.values())
        self.confidence = max(0.0, min(1.0, total))

    def get_confidence_breakdown(self) -> Dict[str, Any]:
        """Get detailed breakdown of confidence contributions by type."""
        return {
            "total": self.confidence,
            "by_type": dict(self._evidence_contributions),
            "counts": dict(self._evidence_counts),
            "caps": self.CONFIDENCE_CAPS,
        }

    def get_status_summary(self) -> str:
        """Get a human-readable status summary."""
        support = len([e for e in self.evidence if e.supports_hypothesis])
        oppose = len([e for e in self.evidence if not e.supports_hypothesis])
        survived = self.surviving_challenges
        total_challenges = len(self.challenges)

        return (
            f"Status: {self.status.value}\n"
            f"Confidence: {self.confidence:.2f}\n"
            f"Evidence: {support} supporting, {oppose} opposing\n"
            f"Challenges: {survived}/{total_challenges} survived\n"
            f"Iterations: {self.iterations}"
        )


# =============================================================================
# CASE MANAGER
# =============================================================================

class CaseManager:
    """Orchestrates the case-based verification lifecycle."""

    def __init__(self, save_dir: str = None):
        self.save_dir = Path(save_dir) if save_dir else Path(
            "/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files"
        )
        self.cases: Dict[str, VerificationCase] = {}
        self.parser = SaveParser()
        self._parsed_cache: Dict[str, Any] = {}
        self._raw_cache: Dict[str, bytes] = {}

    def create_case(
        self,
        flag_id: int,
        item_name: str,
        category: str,
        hypothesis: FlagHypothesis = None,
        item_id: int = None,
    ) -> VerificationCase:
        """Create a new verification case."""
        case_id = f"{flag_id}_{datetime.now().strftime('%Y%m%d%H%M%S')}"

        # Auto-generate hypothesis if not provided
        if hypothesis is None:
            result = calculate_block_offset(flag_id)
            if result:
                byte_offset, bit_position = result
                hypothesis = FlagHypothesis(
                    byte_offset=byte_offset,
                    bit_position=bit_position,
                )

        # Determine block start
        block_start = (flag_id // 1000) * 1000

        case = VerificationCase(
            case_id=case_id,
            flag_id=flag_id,
            item_name=item_name,
            category=category,
            item_id=item_id,
            hypothesis=hypothesis,
            block_start=block_start,
        )

        self.cases[case_id] = case
        return case

    def _load_save(self, save_path: str) -> Tuple[Any, bytes]:
        """Load and cache a save file."""
        if save_path not in self._parsed_cache:
            self._parsed_cache[save_path] = self.parser.parse(save_path)
            with open(save_path, 'rb') as f:
                self._raw_cache[save_path] = f.read()
        return self._parsed_cache[save_path], self._raw_cache[save_path]

    def _check_inventory(self, raw_slot: bytes, item_id: int) -> bool:
        """Check if item is in inventory."""
        patterns = [
            struct.pack('<I', item_id),
            struct.pack('<I', 0x40000000 | (item_id & 0x0FFFFFFF)),
            struct.pack('<I', 0x20000000 | (item_id & 0x0FFFFFFF)),
        ]
        return any(p in raw_slot for p in patterns)

    def _get_slot_raw(self, raw_save: bytes, slot) -> bytes:
        """Extract raw bytes for a slot."""
        slot_start = slot.slot_offset
        slot_end = slot_start + 2000000
        return raw_save[slot_start:min(slot_end, len(raw_save))]

    # =========================================================================
    # DEFENSE METHODS
    # =========================================================================

    def defend_with_inventory_differential(
        self,
        case: VerificationCase,
        save_path: str,
        slot_with_item: int,
        slot_without_item: int,
    ) -> CaseEvidence:
        """
        Defense: Compare slots where item IS present vs ABSENT.
        """
        parsed, raw_save = self._load_save(save_path)

        if case.hypothesis is None:
            raise ValueError("Case has no hypothesis")

        ef_with = parsed.slots[slot_with_item].event_flags
        ef_without = parsed.slots[slot_without_item].event_flags

        offset = case.hypothesis.byte_offset
        bit = case.hypothesis.bit_position

        byte_with = ef_with[offset] if offset < len(ef_with) else 0
        byte_without = ef_without[offset] if offset < len(ef_without) else 0

        bit_with = (byte_with >> bit) & 1
        bit_without = (byte_without >> bit) & 1

        # Expected: SET in slot with item, UNSET in slot without
        supports = (bit_with == 1 and bit_without == 0)

        # Check for padding (both 0xFF)
        is_padding = (byte_with == 0xFF and byte_without == 0xFF)
        if is_padding:
            supports = False

        evidence = CaseEvidence(
            evidence_type="differential",
            source=EvidenceSource(
                save_file=save_path,
                slot_index=slot_with_item,
                evidence_type="inventory_differential",
                method="defend_with_inventory_differential",
            ),
            supports_hypothesis=supports,
            confidence_contribution=CONFIDENCE_WEIGHTS["cross_slot_differential"] if supports else -0.20,
            byte_offset=offset,
            bit_position=bit,
            observed_value=byte_with,
            notes=f"S{slot_with_item}=0x{byte_with:02X}(bit{bit}={bit_with}), "
                  f"S{slot_without_item}=0x{byte_without:02X}(bit{bit}={bit_without})"
                  + (" [PADDING]" if is_padding else ""),
        )

        case.add_evidence(evidence)
        return evidence

    def defend_with_inventory_presence(
        self,
        case: VerificationCase,
        save_path: str,
        slot_index: int,
    ) -> CaseEvidence:
        """
        Defense: Check if item presence matches flag state.
        """
        if case.item_id is None:
            raise ValueError("Case has no item_id for inventory check")

        parsed, raw_save = self._load_save(save_path)
        slot = parsed.slots[slot_index]
        slot_raw = self._get_slot_raw(raw_save, slot)

        item_present = self._check_inventory(slot_raw, case.item_id)

        ef_data = slot.event_flags
        offset = case.hypothesis.byte_offset
        bit = case.hypothesis.bit_position

        byte_val = ef_data[offset] if offset < len(ef_data) else 0
        flag_set = (byte_val >> bit) & 1

        # Item present should match flag set
        supports = (item_present == bool(flag_set))

        evidence = CaseEvidence(
            evidence_type="inventory_presence",
            source=EvidenceSource(
                save_file=save_path,
                slot_index=slot_index,
                evidence_type="inventory",
                method="defend_with_inventory_presence",
            ),
            supports_hypothesis=supports,
            confidence_contribution=CONFIDENCE_WEIGHTS["inventory_present"] if supports else -0.25,
            byte_offset=offset,
            bit_position=bit,
            observed_value=byte_val,
            slot_context={"item_present": item_present, "flag_set": bool(flag_set)},
            notes=f"Item {'present' if item_present else 'absent'}, "
                  f"flag {'set' if flag_set else 'unset'}",
        )

        case.add_evidence(evidence)
        return evidence

    # =========================================================================
    # CHALLENGE METHODS
    # =========================================================================

    def challenge_padding_detection(
        self,
        case: VerificationCase,
        save_path: str,
    ) -> CaseChallenge:
        """
        Challenge: Check if hypothesis offset lands in 0xFF padding.
        """
        parsed, _ = self._load_save(save_path)
        offset = case.hypothesis.byte_offset

        all_ff = True
        slot_values = []

        for slot_idx, slot in enumerate(parsed.slots):
            if slot.event_flags:
                byte_val = slot.event_flags[offset] if offset < len(slot.event_flags) else 0
                slot_values.append(byte_val)
                if byte_val != 0xFF:
                    all_ff = False

        if all_ff and slot_values:
            challenge = CaseChallenge(
                challenge_type="padding_check",
                description=f"Check if offset {offset} is 0xFF padding",
                result="failed",
                test_method="challenge_padding_detection",
                test_data={"slot_values": [f"0x{v:02X}" for v in slot_values]},
                disproves_hypothesis=True,
                notes=f"Offset {offset} is 0xFF in all {len(slot_values)} slots - padding region",
            )
        else:
            challenge = CaseChallenge(
                challenge_type="padding_check",
                description=f"Check if offset {offset} is 0xFF padding",
                result="survived",
                test_method="challenge_padding_detection",
                test_data={"slot_values": [f"0x{v:02X}" for v in slot_values]},
                disproves_hypothesis=False,
                notes=f"Offset {offset} contains data (not all 0xFF)",
            )

        case.add_challenge(challenge)
        return challenge

    def challenge_false_positive(
        self,
        case: VerificationCase,
        save_path: str,
    ) -> CaseChallenge:
        """
        Challenge: Check for false positive matches.
        """
        if case.item_id is None:
            return CaseChallenge(
                challenge_type="false_positive",
                description="Check false positive rate",
                result="inconclusive",
                notes="No item_id available for inventory check",
            )

        parsed, raw_save = self._load_save(save_path)
        mismatches = 0
        total = 0

        for slot_idx, slot in enumerate(parsed.slots):
            if not slot.event_flags:
                continue

            slot_raw = self._get_slot_raw(raw_save, slot)
            item_present = self._check_inventory(slot_raw, case.item_id)

            offset = case.hypothesis.byte_offset
            bit = case.hypothesis.bit_position
            flag_set = (slot.event_flags[offset] >> bit) & 1 if offset < len(slot.event_flags) else 0

            if item_present != bool(flag_set):
                mismatches += 1
            total += 1

        false_positive_rate = mismatches / total if total > 0 else 0

        if false_positive_rate > 0.20:
            challenge = CaseChallenge(
                challenge_type="false_positive",
                description=f"Check false positive rate across {total} slots",
                result="failed",
                test_data={"mismatches": mismatches, "total": total, "rate": false_positive_rate},
                disproves_hypothesis=True,
                notes=f"False positive rate {false_positive_rate:.1%} exceeds 20% threshold",
            )
        else:
            challenge = CaseChallenge(
                challenge_type="false_positive",
                description=f"Check false positive rate across {total} slots",
                result="survived",
                test_data={"mismatches": mismatches, "total": total, "rate": false_positive_rate},
                disproves_hypothesis=False,
                notes=f"False positive rate {false_positive_rate:.1%} acceptable",
            )

        case.add_challenge(challenge)
        return challenge

    def challenge_alternative_base(
        self,
        case: VerificationCase,
        save_path: str,
        related_flags: List[Tuple[int, int]] = None,  # [(flag_id, item_id), ...]
        search_range: int = 100,
    ) -> CaseChallenge:
        """
        Challenge: Search for a better base offset.
        """
        if related_flags is None:
            # Can't test without related flags
            return CaseChallenge(
                challenge_type="alternative_base",
                description="Search for alternative base offset",
                result="inconclusive",
                notes="No related flags provided for comparison",
            )

        parsed, raw_save = self._load_save(save_path)
        slot = parsed.slots[0]  # Use first slot
        ef_data = slot.event_flags
        slot_raw = self._get_slot_raw(raw_save, slot)

        current_base = case.hypothesis.implied_base or (
            case.hypothesis.byte_offset - (case.flag_id - case.block_start) // 8
        )

        def count_matches(base: int) -> int:
            matches = 0
            for flag_id, item_id in related_flags:
                byte_offset = base + (flag_id - case.block_start) // 8
                bit = 7 - (flag_id % 8)

                if byte_offset < 0 or byte_offset >= len(ef_data):
                    continue

                flag_set = (ef_data[byte_offset] >> bit) & 1
                item_present = self._check_inventory(slot_raw, item_id) if item_id else True

                if item_present == bool(flag_set):
                    matches += 1
            return matches

        current_matches = count_matches(current_base)
        best_alternative = None
        best_matches = current_matches

        for test_base in range(current_base - search_range, current_base + search_range):
            if test_base == current_base:
                continue
            matches = count_matches(test_base)
            if matches > best_matches:
                best_matches = matches
                best_alternative = test_base

        if best_alternative and best_matches > current_matches * 1.2:
            challenge = CaseChallenge(
                challenge_type="alternative_base",
                description=f"Search for better base in range {current_base - search_range} to {current_base + search_range}",
                result="failed",
                test_data={
                    "current_base": current_base,
                    "current_matches": current_matches,
                    "better_base": best_alternative,
                    "better_matches": best_matches,
                },
                disproves_hypothesis=True,
                alternative_hypothesis=FlagHypothesis(
                    byte_offset=best_alternative + (case.flag_id - case.block_start) // 8,
                    bit_position=case.hypothesis.bit_position,
                    implied_base=best_alternative,
                ),
                notes=f"Base {best_alternative} has {best_matches} matches vs {current_matches}",
            )
        else:
            challenge = CaseChallenge(
                challenge_type="alternative_base",
                description=f"Search for better base in range {current_base - search_range} to {current_base + search_range}",
                result="survived",
                test_data={
                    "current_base": current_base,
                    "current_matches": current_matches,
                    "best_alternative": best_alternative,
                    "best_matches": best_matches,
                },
                disproves_hypothesis=False,
                notes=f"No significantly better base found (best alternative: {best_alternative} with {best_matches} matches)",
            )

        case.add_challenge(challenge)
        return challenge

    # =========================================================================
    # ADDITIONAL DEFENSE METHODS
    # =========================================================================

    def defend_with_cross_save(
        self,
        case: VerificationCase,
        save_paths: List[str],
        slot_index: int = 0,
    ) -> List[CaseEvidence]:
        """
        Defense: Test hypothesis against multiple save files.

        Same item in different saves should have flag at same offset
        (validates formula consistency across saves).
        """
        evidence_list = []

        for save_path in save_paths:
            try:
                parsed, raw_save = self._load_save(save_path)
                slot = parsed.slots[slot_index]

                if not slot.event_flags:
                    continue

                slot_raw = self._get_slot_raw(raw_save, slot)

                # Check item and flag
                item_present = self._check_inventory(slot_raw, case.item_id) if case.item_id else None
                offset = case.hypothesis.byte_offset
                bit = case.hypothesis.bit_position

                byte_val = slot.event_flags[offset] if offset < len(slot.event_flags) else 0
                flag_set = (byte_val >> bit) & 1

                # Item presence should match flag state
                if item_present is not None:
                    supports = (item_present == bool(flag_set))
                else:
                    # No item to check, just verify flag is accessible
                    supports = True

                evidence = CaseEvidence(
                    evidence_type="cross_save",
                    source=EvidenceSource(
                        save_file=save_path,
                        slot_index=slot_index,
                        evidence_type="cross_save_validation",
                        method="defend_with_cross_save",
                    ),
                    supports_hypothesis=supports,
                    confidence_contribution=0.10 if supports else -0.15,
                    byte_offset=offset,
                    bit_position=bit,
                    observed_value=byte_val,
                    slot_context={
                        "item_present": item_present,
                        "flag_set": bool(flag_set),
                        "save_file": Path(save_path).name,
                    },
                    notes=f"Cross-save ({Path(save_path).name}): "
                          f"item={'present' if item_present else 'absent' if item_present is not None else 'N/A'}, "
                          f"flag={'set' if flag_set else 'unset'}",
                )

                case.add_evidence(evidence)
                evidence_list.append(evidence)

            except Exception as e:
                case.notes.append(f"Cross-save error ({save_path}): {e}")

        return evidence_list

    def _load_save_config(self):
        """Load save_config.json for multi-save validation."""
        config_path = PROJECT_ROOT / "scripts" / "verification" / "save_config.json"
        self._save_config = {}

        if not config_path.exists():
            return

        try:
            with open(config_path) as f:
                self._save_config = json.load(f)
        except Exception as e:
            print(f"Warning: Could not load save_config.json: {e}")

    def defend_with_cross_save_auto(
        self,
        case: VerificationCase,
        differential_set: str = None,
    ) -> List[CaseEvidence]:
        """
        Defense: Auto-run cross-save validation using save_config.json.

        Uses configured differential_sets to compare slots with/without items.
        """
        if not hasattr(self, '_save_config'):
            self._load_save_config()

        if not self._save_config:
            return []

        evidence_list = []
        save_dir = Path(self._save_config.get('save_directory', ''))

        # Get differential set configuration
        diff_sets = self._save_config.get('differential_sets', {})
        if differential_set and differential_set in diff_sets:
            diff_config = diff_sets[differential_set]
        elif case.category in diff_sets:
            diff_config = diff_sets[case.category]
        else:
            # Default to spirit_ash set
            diff_config = diff_sets.get('spirit_ash', {
                'with_item': [0],
                'without_item': [1, 2, 3, 4],
            })

        slots_with = diff_config.get('with_item', [0])
        slots_without = diff_config.get('without_item', [1, 2, 3, 4])

        # Iterate all configured saves
        for save_config in self._save_config.get('saves', []):
            save_path = save_dir / save_config.get('path', '')
            if not save_path.exists():
                continue

            save_id = save_config.get('id', save_path.name)

            # Run differential for each configured slot pair
            for with_slot in slots_with:
                for without_slot in slots_without:
                    # Check if both slots exist in this save's config
                    slots_config = save_config.get('slots', {})
                    if str(with_slot) not in slots_config or str(without_slot) not in slots_config:
                        continue

                    try:
                        evidence = self.defend_with_inventory_differential(
                            case, str(save_path), with_slot, without_slot
                        )
                        evidence.notes += f" [{save_id}]"
                        evidence_list.append(evidence)
                    except Exception as e:
                        case.notes.append(f"Cross-save auto error ({save_id}, S{with_slot} vs S{without_slot}): {e}")

        return evidence_list

    def get_all_configured_saves(self) -> List[str]:
        """Get list of all save file paths from config."""
        if not hasattr(self, '_save_config'):
            self._load_save_config()

        if not self._save_config:
            return []

        save_dir = Path(self._save_config.get('save_directory', ''))
        saves = []

        for save_config in self._save_config.get('saves', []):
            save_path = save_dir / save_config.get('path', '')
            if save_path.exists():
                saves.append(str(save_path))

        return saves

    def get_save_config_summary(self) -> Dict[str, Any]:
        """Get summary of save configuration."""
        if not hasattr(self, '_save_config'):
            self._load_save_config()

        if not self._save_config:
            return {"error": "No save config loaded"}

        saves = self._save_config.get('saves', [])
        total_slots = sum(len(s.get('slots', {})) for s in saves)

        return {
            "saves": len(saves),
            "total_slots": total_slots,
            "differential_sets": list(self._save_config.get('differential_sets', {}).keys()),
            "save_files": [s.get('path') for s in saves],
        }

    def defend_with_chain_anchor(
        self,
        case: VerificationCase,
        save_path: str,
        slot_index: int,
        anchor_flags: List[Tuple[int, str]] = None,  # [(flag_id, name), ...]
    ) -> CaseEvidence:
        """
        Defense: Verify flag is connected to already-verified related flags.

        Example: Spirit Ash from catacomb should correlate with:
          - Catacomb boss defeat flag
          - Catacomb discovery/completion flags
        """
        if anchor_flags is None:
            # Try to auto-detect related flags based on category
            anchor_flags = self._get_related_anchors(case)

        if not anchor_flags:
            return CaseEvidence(
                evidence_type="chain_anchor",
                source=EvidenceSource(
                    save_file=save_path,
                    slot_index=slot_index,
                    evidence_type="chain_anchor",
                    method="defend_with_chain_anchor",
                ),
                supports_hypothesis=False,
                confidence_contribution=0.0,
                notes="No anchor flags available for chain verification",
            )

        parsed, raw_save = self._load_save(save_path)
        slot = parsed.slots[slot_index]
        ef_data = slot.event_flags
        slot_raw = self._get_slot_raw(raw_save, slot)

        # Check if case item is present
        item_present = self._check_inventory(slot_raw, case.item_id) if case.item_id else None

        # Check anchor flags
        anchor_matches = 0
        anchor_details = []

        for anchor_flag, anchor_name in anchor_flags:
            anchor_result = calculate_block_offset(anchor_flag)
            if anchor_result:
                anchor_offset, anchor_bit = anchor_result
                if anchor_offset < len(ef_data):
                    anchor_set = (ef_data[anchor_offset] >> anchor_bit) & 1

                    # Anchor should match item presence
                    if item_present is not None:
                        matches = (bool(anchor_set) == item_present)
                    else:
                        matches = True  # Can't verify without item

                    if matches:
                        anchor_matches += 1
                    anchor_details.append(f"{anchor_name}({'set' if anchor_set else 'unset'})")

        total_anchors = len(anchor_flags)
        match_rate = anchor_matches / total_anchors if total_anchors > 0 else 0
        supports = match_rate >= 0.7  # 70% anchor match required

        evidence = CaseEvidence(
            evidence_type="chain_anchor",
            source=EvidenceSource(
                save_file=save_path,
                slot_index=slot_index,
                evidence_type="chain_anchor",
                method="defend_with_chain_anchor",
            ),
            supports_hypothesis=supports,
            confidence_contribution=CONFIDENCE_WEIGHTS["chain_anchor"] if supports else -0.05,
            slot_context={
                "anchor_matches": anchor_matches,
                "total_anchors": total_anchors,
                "anchors": anchor_details,
            },
            notes=f"Chain anchors: {anchor_matches}/{total_anchors} match ({', '.join(anchor_details)})",
        )

        case.add_evidence(evidence)
        return evidence

    def _get_related_anchors(self, case: VerificationCase) -> List[Tuple[int, str]]:
        """Get related anchor flags based on case category.

        Queries multiple data sources:
        1. flag_relationships.json (2,796 relationship edges - PRIMARY)
        2. event_graph.json (dependencies, enables, progression_chains)
        3. anchor_database.json (curated category anchors)
        """
        # Lazy-load data sources
        if not hasattr(self, '_flag_rels'):
            self._load_flag_relationships()
        if not hasattr(self, '_event_graph'):
            self._load_event_graph()
        if not hasattr(self, '_anchor_db'):
            self._load_anchor_database()

        anchors = []
        flag_str = str(case.flag_id)
        seen_flags = set()

        # 1. Query flag_relationships.json (PRIMARY source)
        if flag_str in self._flag_rels_index:
            for rel in self._flag_rels_index[flag_str]:
                target = rel.get('target')
                if target and target not in seen_flags:
                    rel_type = rel.get('type', 'related')
                    item_name = rel.get('item', f'flag {target}')
                    anchors.append((target, f"{rel_type}: {item_name}"))
                    seen_flags.add(target)

        # 2. Query event_graph for dependencies/enables
        if flag_str in self._event_graph.get('flag_dependencies', {}):
            deps = self._event_graph['flag_dependencies'][flag_str]
            for dep in deps.get('depends_on', []):
                req_flag = dep.get('required_flag')
                if req_flag and req_flag not in seen_flags:
                    cond = dep.get('condition_type', 'dependency')
                    anchors.append((req_flag, f"Prerequisite: {cond}"))
                    seen_flags.add(req_flag)
            for en in deps.get('enables', []):
                en_flag = en.get('enabled_flag')
                if en_flag and en_flag not in seen_flags:
                    rel = en.get('relationship', 'unlock')
                    anchors.append((en_flag, f"Enables: {rel}"))
                    seen_flags.add(en_flag)

        # 3. Query event_graph progression chains (remembrance → boss defeat)
        for chain_key, chain in self._event_graph.get('progression_chains', {}).items():
            if chain.get('possession_flag') == case.flag_id:
                boss_defeat = chain.get('boss_defeat')
                if boss_defeat and boss_defeat not in seen_flags:
                    anchors.append((boss_defeat, f"Boss defeat for {case.item_name}"))
                    seen_flags.add(boss_defeat)
            # Also check if this flag is the boss defeat
            if chain.get('boss_defeat') == case.flag_id:
                poss_flag = chain.get('possession_flag')
                if poss_flag and poss_flag not in seen_flags:
                    anchors.append((poss_flag, f"Remembrance from boss"))
                    seen_flags.add(poss_flag)

        # 4. Query boss_defeat_chains from anchor_database
        # Note: boss_defeat_chains has nested structure: {"description": "...", "chains": {...}}
        for defeat_str, chain in self._anchor_db.get('boss_defeat_chains', {}).get('chains', {}).items():
            defeat_flag = chain.get('defeat_flag')
            remem_flag = chain.get('remembrance_flag')
            rune_flag = chain.get('great_rune_flag')
            act_flag = chain.get('activation_flag')

            # If case flag matches any chain element, add related flags
            if case.flag_id == defeat_flag:
                if remem_flag and remem_flag not in seen_flags:
                    anchors.append((remem_flag, f"Remembrance: {chain.get('name')}"))
                    seen_flags.add(remem_flag)
                if rune_flag and rune_flag not in seen_flags:
                    anchors.append((rune_flag, f"Great Rune: {chain.get('name')}"))
                    seen_flags.add(rune_flag)
            elif case.flag_id == remem_flag:
                if defeat_flag and defeat_flag not in seen_flags:
                    anchors.append((defeat_flag, f"Boss Defeat: {chain.get('name')}"))
                    seen_flags.add(defeat_flag)

        # 5. Query anchor_database by category (curated fallback)
        if case.category in self._anchor_db.get('category_anchors', {}):
            cat_data = self._anchor_db['category_anchors'][case.category]
            for example in cat_data.get('examples', []):
                ex_flag = example.get('flag')
                if ex_flag and ex_flag not in seen_flags:
                    anchors.append((ex_flag, example.get('name', f'flag {ex_flag}')))
                    seen_flags.add(ex_flag)

        # 6. Query geographic regions for grace/landmark flags
        # Note: geographic_regions has nested structure: {"description": "...", "regions": {...}}
        if case.category in ('grace', 'landmark'):
            for region_name, region_data in self._anchor_db.get('geographic_regions', {}).get('regions', {}).items():
                landmark_range = region_data.get('landmark_range', [0, 0])
                grace_range = region_data.get('grace_range', [0, 0])
                map_frag = region_data.get('map_fragment')

                # Check if case flag is in this region
                in_landmark = landmark_range and landmark_range[0] <= case.flag_id <= landmark_range[1]
                in_grace = grace_range and grace_range[0] <= case.flag_id <= grace_range[1]

                if in_landmark or in_grace:
                    # Add map fragment as anchor
                    if map_frag and map_frag not in seen_flags:
                        anchors.append((map_frag, f"Map Fragment: {region_name}"))
                        seen_flags.add(map_frag)
                    # Add sample grace/landmark from region
                    if in_landmark and grace_range:
                        sample_grace = grace_range[0]
                        if sample_grace not in seen_flags:
                            anchors.append((sample_grace, f"Grace: {region_name}"))
                            seen_flags.add(sample_grace)
                    break

        return anchors

    def _load_flag_relationships(self):
        """Load flag_relationships.json (PRIMARY source for anchors)."""
        rel_path = PROJECT_ROOT / "scripts" / "flag_relationships.json"
        self._flag_rels_index = {}

        if not rel_path.exists():
            return

        try:
            with open(rel_path) as f:
                data = json.load(f)

            # Build index by source flag
            for edge in data.get('edges', []):
                source = str(edge.get('source'))
                target = edge.get('target')
                if source not in self._flag_rels_index:
                    self._flag_rels_index[source] = []
                self._flag_rels_index[source].append({
                    'target': target,
                    'type': edge.get('type'),
                    'item': edge.get('item'),
                })

                # Also index by target for reverse lookups
                target_str = str(target)
                if target_str not in self._flag_rels_index:
                    self._flag_rels_index[target_str] = []
                self._flag_rels_index[target_str].append({
                    'target': edge.get('source'),
                    'type': f"reverse_{edge.get('type', 'related')}",
                    'item': edge.get('item'),
                })
        except Exception as e:
            print(f"Warning: Could not load flag_relationships.json: {e}")

    def _load_event_graph(self):
        """Load event_graph.json (SECONDARY source for dependencies/enables)."""
        graph_path = PROJECT_ROOT / "scripts" / "event_graph.json"
        self._event_graph = {}

        if not graph_path.exists():
            return

        try:
            with open(graph_path) as f:
                data = json.load(f)

            # Extract relevant sections
            self._event_graph = {
                'flag_dependencies': data.get('flag_dependencies', {}),
                'progression_chains': data.get('progression_chains', {}),
                'flag_triggers': data.get('flag_triggers', {}),
            }
        except Exception as e:
            print(f"Warning: Could not load event_graph.json: {e}")

    def _load_anchor_database(self):
        """Load curated anchor_database.json."""
        db_path = PROJECT_ROOT / "scripts" / "verification" / "anchor_database.json"
        self._anchor_db = {}

        if not db_path.exists():
            return

        try:
            with open(db_path) as f:
                self._anchor_db = json.load(f)
        except Exception as e:
            print(f"Warning: Could not load anchor_database.json: {e}")

    # =========================================================================
    # CATALOG INTEGRATION
    # =========================================================================

    CATALOG_PATH = Path("/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files/Granular snapshots for debugging/capture_catalog.json")
    SNAPSHOT_DIR = Path("/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files/Granular snapshots for debugging")

    def _load_capture_catalog(self):
        """Load capture_catalog.json for temporal snapshot pairs."""
        if not self.CATALOG_PATH.exists():
            self._catalog = {"captures": [], "pairs": []}
            return

        try:
            with open(self.CATALOG_PATH) as f:
                self._catalog = json.load(f)
        except Exception as e:
            print(f"Warning: Could not load capture_catalog.json: {e}")
            self._catalog = {"captures": [], "pairs": []}

    def _resolve_snapshot_path(self, capture_id: str) -> Optional[str]:
        """Resolve a capture ID to its full file path."""
        if not hasattr(self, '_catalog'):
            self._load_capture_catalog()

        # Find capture by ID
        for capture in self._catalog.get('captures', []):
            if capture.get('id') == capture_id:
                filename = capture.get('filename')
                if filename:
                    full_path = self.SNAPSHOT_DIR / filename
                    if full_path.exists():
                        return str(full_path)
        return None

    def find_temporal_pairs(self, flag_id: int) -> List[Dict[str, Any]]:
        """Find before/after pairs matching a flag_id from the catalog."""
        if not hasattr(self, '_catalog'):
            self._load_capture_catalog()

        pairs = []
        for pair in self._catalog.get('pairs', []):
            pair_flag = pair.get('flag_id')
            # Match exact flag or allow untagged pairs with name matching
            if pair_flag == flag_id:
                pairs.append(pair)
            elif pair_flag is None:
                # Check if POI name might relate to this flag
                # This is a heuristic - pairs with flag_id are preferred
                before_id = pair.get('before_capture')
                for capture in self._catalog.get('captures', []):
                    if capture.get('id') == before_id:
                        poi = capture.get('poi', {})
                        if poi.get('flag_id') == flag_id:
                            pairs.append(pair)
                        break
        return pairs

    def find_temporal_pairs_by_category(self, category: str) -> List[Dict[str, Any]]:
        """Find before/after pairs for a given category."""
        if not hasattr(self, '_catalog'):
            self._load_capture_catalog()

        pairs = []
        for pair in self._catalog.get('pairs', []):
            before_id = pair.get('before_capture')
            for capture in self._catalog.get('captures', []):
                if capture.get('id') == before_id:
                    poi = capture.get('poi', {})
                    if poi.get('category') == category:
                        pairs.append(pair)
                    break
        return pairs

    def defend_with_temporal_auto(
        self,
        case: VerificationCase,
        slot_index: int = 0,
    ) -> List[CaseEvidence]:
        """
        Auto-find temporal pairs from catalog and run temporal defense.

        Returns list of evidence from all matching temporal pairs.
        """
        if not hasattr(self, '_catalog'):
            self._load_capture_catalog()

        pairs = self.find_temporal_pairs(case.flag_id)
        evidence_list = []

        for pair in pairs:
            before_path = self._resolve_snapshot_path(pair.get('before_capture'))
            after_path = self._resolve_snapshot_path(pair.get('after_capture'))

            if not before_path or not after_path:
                case.notes.append(f"Temporal pair {pair.get('pair_id')} missing files")
                continue

            try:
                evidence = self.defend_with_temporal_snapshot(
                    case, before_path, after_path, slot_index
                )
                evidence_list.append(evidence)
            except Exception as e:
                case.notes.append(f"Temporal defense error ({pair.get('pair_id')}): {e}")

        # If no direct flag matches, try category-based matching
        if not evidence_list and case.category:
            category_pairs = self.find_temporal_pairs_by_category(case.category)
            for pair in category_pairs[:2]:  # Limit to 2 category matches
                before_path = self._resolve_snapshot_path(pair.get('before_capture'))
                after_path = self._resolve_snapshot_path(pair.get('after_capture'))

                if not before_path or not after_path:
                    continue

                try:
                    evidence = self.defend_with_temporal_snapshot(
                        case, before_path, after_path, slot_index
                    )
                    evidence.notes += " (category match)"
                    evidence_list.append(evidence)
                except Exception as e:
                    case.notes.append(f"Category temporal error: {e}")

        return evidence_list

    def get_catalog_coverage(self) -> Dict[str, Any]:
        """Get statistics about catalog coverage."""
        if not hasattr(self, '_catalog'):
            self._load_capture_catalog()

        captures = self._catalog.get('captures', [])
        pairs = self._catalog.get('pairs', [])

        # Count pairs with flag_id set
        tagged_pairs = [p for p in pairs if p.get('flag_id') is not None]

        # Get unique flag IDs
        unique_flags = set()
        for pair in pairs:
            if pair.get('flag_id'):
                unique_flags.add(pair['flag_id'])

        # Group by category
        by_category = {}
        for capture in captures:
            poi = capture.get('poi', {})
            cat = poi.get('category', 'unknown')
            if cat not in by_category:
                by_category[cat] = 0
            by_category[cat] += 1

        return {
            "total_captures": len(captures),
            "total_pairs": len(pairs),
            "tagged_pairs": len(tagged_pairs),
            "unique_flags": len(unique_flags),
            "by_category": by_category,
        }

    def defend_with_temporal_snapshot(
        self,
        case: VerificationCase,
        before_path: str,
        after_path: str,
        slot_index: int = 0,
    ) -> CaseEvidence:
        """
        Defense: Use before/after snapshots to verify flag changes with action.

        If player collected item between snapshots:
          - Flag should be UNSET in "before"
          - Flag should be SET in "after"
        """
        parsed_before, _ = self._load_save(before_path)
        parsed_after, _ = self._load_save(after_path)

        ef_before = parsed_before.slots[slot_index].event_flags
        ef_after = parsed_after.slots[slot_index].event_flags

        offset = case.hypothesis.byte_offset
        bit = case.hypothesis.bit_position

        before_byte = ef_before[offset] if offset < len(ef_before) else 0
        after_byte = ef_after[offset] if offset < len(ef_after) else 0

        before_set = (before_byte >> bit) & 1
        after_set = (after_byte >> bit) & 1

        # Expected: 0 → 1 transition (flag was set between snapshots)
        supports = (before_set == 0 and after_set == 1)

        evidence = CaseEvidence(
            evidence_type="temporal",
            source=EvidenceSource(
                save_file=after_path,
                slot_index=slot_index,
                evidence_type="temporal_snapshot",
                method="defend_with_temporal_snapshot",
            ),
            supports_hypothesis=supports,
            confidence_contribution=CONFIDENCE_WEIGHTS["temporal_consistency"] if supports else -0.15,
            byte_offset=offset,
            bit_position=bit,
            observed_value=after_byte,
            expected_value=before_byte,
            slot_context={
                "before_file": Path(before_path).name,
                "after_file": Path(after_path).name,
                "before_set": bool(before_set),
                "after_set": bool(after_set),
            },
            notes=f"Temporal: {'0→1 transition' if supports else f'{before_set}→{after_set}'} "
                  f"(before={Path(before_path).name}, after={Path(after_path).name})",
        )

        case.add_evidence(evidence)
        return evidence

    # =========================================================================
    # ADDITIONAL CHALLENGE METHODS
    # =========================================================================

    def challenge_bit_collision(
        self,
        case: VerificationCase,
        known_flags: List[Tuple[int, str]] = None,  # [(flag_id, name), ...]
    ) -> CaseChallenge:
        """
        Challenge: Check if multiple flags share the same byte/bit location.

        If two different items map to same location, one is wrong.
        """
        offset = case.hypothesis.byte_offset
        bit = case.hypothesis.bit_position

        collisions = []

        if known_flags:
            for other_flag_id, other_name in known_flags:
                if other_flag_id == case.flag_id:
                    continue

                other_result = calculate_block_offset(other_flag_id)
                if other_result and other_result == (offset, bit):
                    collisions.append((other_flag_id, other_name))

        if collisions:
            challenge = CaseChallenge(
                challenge_type="bit_collision",
                description=f"Check for flag collisions at offset {offset} bit {bit}",
                result="failed",
                test_data={
                    "colliding_flags": [(f, n) for f, n in collisions],
                },
                disproves_hypothesis=True,
                notes=f"Collides with: {', '.join(f'{n}({f})' for f, n in collisions)}",
            )
        else:
            challenge = CaseChallenge(
                challenge_type="bit_collision",
                description=f"Check for flag collisions at offset {offset} bit {bit}",
                result="survived",
                test_data={"checked_flags": len(known_flags) if known_flags else 0},
                disproves_hypothesis=False,
                notes=f"No collisions detected among {len(known_flags) if known_flags else 0} known flags",
            )

        case.add_challenge(challenge)
        return challenge

    def challenge_block_boundary(
        self,
        case: VerificationCase,
        save_path: str,
    ) -> CaseChallenge:
        """
        Challenge: Verify the flag doesn't fall outside the block's valid range.

        Checks if the offset is within expected block boundaries.
        """
        parsed, _ = self._load_save(save_path)
        ef_data = parsed.slots[0].event_flags

        offset = case.hypothesis.byte_offset
        block_start = case.block_start

        # Calculate expected block range
        implied_base = case.hypothesis.implied_base or (
            offset - (case.flag_id - block_start) // 8
        )

        # Check if offset is reasonable
        block_size_bytes = 125  # ~1000 flags / 8
        expected_end = implied_base + block_size_bytes

        in_range = implied_base <= offset < expected_end
        in_ef_section = 0 <= offset < len(ef_data)

        if not in_ef_section:
            challenge = CaseChallenge(
                challenge_type="block_boundary",
                description=f"Check if offset {offset} is within EF section",
                result="failed",
                test_data={
                    "offset": offset,
                    "ef_size": len(ef_data),
                },
                disproves_hypothesis=True,
                notes=f"Offset {offset} is outside EF section (size: {len(ef_data)})",
            )
        elif not in_range:
            challenge = CaseChallenge(
                challenge_type="block_boundary",
                description=f"Check if offset {offset} is within block range",
                result="survived",  # Warning but not fatal
                test_data={
                    "offset": offset,
                    "implied_base": implied_base,
                    "expected_end": expected_end,
                },
                disproves_hypothesis=False,
                notes=f"Offset {offset} is outside expected range [{implied_base}, {expected_end}] but valid",
            )
        else:
            challenge = CaseChallenge(
                challenge_type="block_boundary",
                description=f"Check if offset {offset} is within block range",
                result="survived",
                test_data={
                    "offset": offset,
                    "implied_base": implied_base,
                    "expected_end": expected_end,
                },
                disproves_hypothesis=False,
                notes=f"Offset {offset} is within block range [{implied_base}, {expected_end}]",
            )

        case.add_challenge(challenge)
        return challenge

    # =========================================================================
    # CASE PERSISTENCE
    # =========================================================================

    def save_case(self, case: VerificationCase, output_dir: Path = None) -> Path:
        """Save a case to JSON file."""
        if output_dir is None:
            output_dir = PROJECT_ROOT / "scripts" / "verification" / "cases"
        output_dir.mkdir(parents=True, exist_ok=True)

        filepath = output_dir / f"{case.case_id}.json"

        case_dict = {
            "case_id": case.case_id,
            "flag_id": case.flag_id,
            "item_name": case.item_name,
            "category": case.category,
            "item_id": case.item_id,
            "hypothesis": {
                "byte_offset": case.hypothesis.byte_offset,
                "bit_position": case.hypothesis.bit_position,
                "implied_base": case.hypothesis.implied_base,
                "block_start": case.hypothesis.block_start,
            } if case.hypothesis else None,
            "block_start": case.block_start,
            "formula_type": case.formula_type,
            "status": case.status.value,
            "confidence": case.confidence,
            "iterations": case.iterations,
            "surviving_challenges": case.surviving_challenges,
            "created_at": case.created_at,
            "last_updated": case.last_updated,
            "notes": case.notes,
            "evidence": [
                {
                    "evidence_type": e.evidence_type,
                    "source": str(e.source),
                    "supports_hypothesis": e.supports_hypothesis,
                    "confidence_contribution": e.confidence_contribution,
                    "byte_offset": e.byte_offset,
                    "bit_position": e.bit_position,
                    "observed_value": e.observed_value,
                    "notes": e.notes,
                }
                for e in case.evidence
            ],
            "challenges": [
                {
                    "challenge_type": c.challenge_type,
                    "description": c.description,
                    "result": c.result,
                    "disproves_hypothesis": c.disproves_hypothesis,
                    "notes": c.notes,
                }
                for c in case.challenges
            ],
        }

        with open(filepath, 'w') as f:
            json.dump(case_dict, f, indent=2)

        return filepath

    def load_case(self, filepath: Path) -> VerificationCase:
        """Load a case from JSON file."""
        with open(filepath, 'r') as f:
            data = json.load(f)

        hypothesis = None
        if data.get("hypothesis"):
            hypothesis = FlagHypothesis(
                byte_offset=data["hypothesis"]["byte_offset"],
                bit_position=data["hypothesis"]["bit_position"],
                implied_base=data["hypothesis"].get("implied_base"),
                block_start=data["hypothesis"].get("block_start"),
            )

        case = VerificationCase(
            case_id=data["case_id"],
            flag_id=data["flag_id"],
            item_name=data["item_name"],
            category=data["category"],
            item_id=data.get("item_id"),
            hypothesis=hypothesis,
            block_start=data.get("block_start", 0),
            formula_type=data.get("formula_type", "block"),
            status=CaseStatus(data.get("status", "open")),
            confidence=data.get("confidence", 0.0),
            iterations=data.get("iterations", 0),
            surviving_challenges=data.get("surviving_challenges", 0),
            created_at=data.get("created_at", ""),
            last_updated=data.get("last_updated", ""),
            notes=data.get("notes", []),
        )

        # Evidence and challenges are loaded as summaries (not full objects)
        # To fully reconstruct, would need more complex serialization

        self.cases[case.case_id] = case
        return case

    def save_all_cases(self, output_dir: Path = None) -> List[Path]:
        """Save all cases to JSON files."""
        paths = []
        for case in self.cases.values():
            path = self.save_case(case, output_dir)
            paths.append(path)
        return paths

    def load_all_cases(self, case_dir: Path = None) -> List[VerificationCase]:
        """Load all cases from a directory."""
        if case_dir is None:
            case_dir = PROJECT_ROOT / "scripts" / "verification" / "cases"

        if not case_dir.exists():
            return []

        cases = []
        for filepath in case_dir.glob("*.json"):
            try:
                case = self.load_case(filepath)
                cases.append(case)
            except Exception as e:
                print(f"Error loading {filepath}: {e}")

        return cases

    # =========================================================================
    # LIFECYCLE MANAGEMENT
    # =========================================================================

    def run_defense_phase(
        self,
        case: VerificationCase,
        save_path: str,
        slots_with_item: List[int] = None,
        slots_without_item: List[int] = None,
    ):
        """Run a complete defense phase."""
        case.status = CaseStatus.DEFENDING

        # 1. Inventory presence check (if item_id available)
        if case.item_id:
            for slot_idx in range(5):
                try:
                    self.defend_with_inventory_presence(case, save_path, slot_idx)
                except Exception as e:
                    case.notes.append(f"Defense error (inventory presence, slot {slot_idx}): {e}")

        # 2. Inventory differential (if slots specified)
        if slots_with_item and slots_without_item:
            for with_slot in slots_with_item:
                for without_slot in slots_without_item:
                    try:
                        self.defend_with_inventory_differential(
                            case, save_path, with_slot, without_slot
                        )
                    except Exception as e:
                        case.notes.append(f"Defense error (differential, {with_slot} vs {without_slot}): {e}")

        case.iterations += 1

    def run_challenge_phase(
        self,
        case: VerificationCase,
        save_path: str,
        related_flags: List[Tuple[int, int]] = None,
        known_flags: List[Tuple[int, str]] = None,
    ):
        """Run a complete challenge phase."""
        case.status = CaseStatus.CHALLENGING

        # 1. Padding detection
        self.challenge_padding_detection(case, save_path)

        # 2. False positive analysis
        if case.item_id:
            self.challenge_false_positive(case, save_path)

        # 3. Alternative base search
        if related_flags:
            self.challenge_alternative_base(case, save_path, related_flags)

        # 4. Bit collision check
        if known_flags:
            self.challenge_bit_collision(case, known_flags)

        # 5. Block boundary check
        self.challenge_block_boundary(case, save_path)

        # Update status based on challenges
        failed_challenges = [c for c in case.challenges if c.disproves_hypothesis]
        if failed_challenges:
            case.status = CaseStatus.REJECTED
        elif case.confidence >= THRESHOLDS["verified"]:
            case.status = CaseStatus.VERIFIED
        elif case.confidence >= THRESHOLDS["high_confidence"]:
            case.status = CaseStatus.PARTIAL

    def run_full_verification(
        self,
        case: VerificationCase,
        save_path: str,
        slots_with_item: List[int] = None,
        slots_without_item: List[int] = None,
        related_flags: List[Tuple[int, int]] = None,
        known_flags: List[Tuple[int, str]] = None,
        additional_saves: List[str] = None,
        min_iterations: int = 2,
    ) -> VerificationCase:
        """
        Run full verification with multiple defense/challenge iterations.

        Args:
            case: The case to verify
            save_path: Primary save file
            slots_with_item: Slots where item is present
            slots_without_item: Slots where item is absent
            related_flags: Related flags for alternative base search
            known_flags: Known flags for collision check
            additional_saves: Additional save files for cross-save validation
            min_iterations: Minimum defense/challenge cycles

        Returns:
            The updated case with final status
        """
        for iteration in range(min_iterations):
            # Defense phase
            self.run_defense_phase(
                case, save_path,
                slots_with_item=slots_with_item,
                slots_without_item=slots_without_item,
            )

            # Cross-save validation if additional saves provided
            if additional_saves and iteration == 0:
                self.defend_with_cross_save(case, additional_saves)

            # Challenge phase
            self.run_challenge_phase(
                case, save_path,
                related_flags=related_flags,
                known_flags=known_flags,
            )

            # Early exit if rejected
            if case.status == CaseStatus.REJECTED:
                break

            # Early exit if verified with high confidence
            if case.status == CaseStatus.VERIFIED and case.confidence >= 0.95:
                break

        return case

    def propose_formula_update(self, case: VerificationCase) -> Optional[Dict[str, Any]]:
        """
        Generate formula update proposal when verification fails.

        Analyzes failed challenges to suggest alternative base offsets.
        Returns None if case is not rejected or no better alternative found.

        Returns:
            Dict with proposed update or None:
            {
                "action": "update_block_base",
                "block": block_start,
                "current": current_base,
                "proposed": better_base,
                "confidence": match_rate,
                "reason": explanation,
            }
        """
        if case.status != CaseStatus.REJECTED:
            return None

        # Analyze failed challenges for alternative bases
        for challenge in case.challenges:
            if challenge.disproves_hypothesis and challenge.challenge_type == "alternative_base":
                test_data = challenge.test_data
                if test_data.get("better_base") is not None:
                    return {
                        "action": "update_block_base",
                        "block": case.block_start,
                        "current": test_data.get("current_base"),
                        "proposed": test_data.get("better_base"),
                        "confidence": test_data.get("better_matches", 0) / max(test_data.get("total_flags", 1), 1),
                        "reason": challenge.notes,
                    }

        # Check for padding-related failures
        for challenge in case.challenges:
            if challenge.disproves_hypothesis and challenge.challenge_type == "padding_check":
                return {
                    "action": "investigate_block",
                    "block": case.block_start,
                    "current": case.hypothesis.implied_base if case.hypothesis else None,
                    "proposed": None,
                    "confidence": 0.0,
                    "reason": f"Offset {case.hypothesis.byte_offset if case.hypothesis else 'unknown'} is in padding region",
                }

        return None

    def get_verification_summary(self) -> Dict[str, Any]:
        """Get summary statistics across all cases."""
        summary = {
            "total": len(self.cases),
            "by_status": {},
            "by_category": {},
            "average_confidence": 0.0,
        }

        confidences = []
        for case in self.cases.values():
            # Count by status
            status = case.status.value
            summary["by_status"][status] = summary["by_status"].get(status, 0) + 1

            # Count by category
            category = case.category
            if category not in summary["by_category"]:
                summary["by_category"][category] = {"total": 0, "verified": 0}
            summary["by_category"][category]["total"] += 1
            if case.status == CaseStatus.VERIFIED:
                summary["by_category"][category]["verified"] += 1

            confidences.append(case.confidence)

        if confidences:
            summary["average_confidence"] = sum(confidences) / len(confidences)

        return summary

    def get_case_report(self, case: VerificationCase) -> str:
        """Generate a detailed report for a case."""
        lines = [
            f"CASE: {case.case_id}",
            "=" * 70,
            "",
            f"Flag: {case.flag_id} ({case.item_name})",
            f"Category: {case.category}",
            f"Hypothesis: {case.hypothesis}",
            "",
            case.get_status_summary(),
            "",
            "EVIDENCE:",
            "-" * 40,
        ]

        for ev in case.evidence:
            marker = "+" if ev.supports_hypothesis else "-"
            lines.append(f"  {marker} [{ev.evidence_type}] {ev.notes}")
            lines.append(f"    Source: {ev.source}")
            lines.append(f"    Confidence: {ev.confidence_contribution:+.2f}")

        lines.extend([
            "",
            "CHALLENGES:",
            "-" * 40,
        ])

        for ch in case.challenges:
            marker = "SURVIVED" if not ch.disproves_hypothesis else "FAILED"
            lines.append(f"  [{marker}] {ch.challenge_type}: {ch.description}")
            lines.append(f"    {ch.notes}")

        if case.notes:
            lines.extend([
                "",
                "NOTES:",
                "-" * 40,
            ])
            for note in case.notes:
                lines.append(f"  - {note}")

        return "\n".join(lines)


# =============================================================================
# CLI / DEMO
# =============================================================================

def demo():
    """Demonstrate the case-based verification system."""
    print("=" * 70)
    print("CASE-BASED VERIFICATION DEMO")
    print("=" * 70)

    manager = CaseManager()

    # Create a case for a known flag
    case = manager.create_case(
        flag_id=520000,
        item_name="Lhutel the Headless",
        category="spirit_ash",
        item_id=258000,
        hypothesis=FlagHypothesis(byte_offset=1341, bit_position=7, implied_base=1341),
    )

    save_path = "/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files/ER0000-backup-2026-01-11.sl2"

    print(f"\nCreated case: {case.case_id}")
    print(f"Hypothesis: {case.hypothesis}")

    # Run defense phase
    print("\n--- DEFENSE PHASE ---")
    manager.run_defense_phase(
        case,
        save_path,
        slots_with_item=[0],
        slots_without_item=[1, 2, 3, 4],
    )

    # Run challenge phase
    print("\n--- CHALLENGE PHASE ---")
    related_flags = [
        (520030, 5050),   # Assassin's Crimson Dagger
        (520040, 202000), # Banished Knight Engvall
        (520050, 219000), # Twinsage Sorcerer Ashes
    ]
    manager.run_challenge_phase(case, save_path, related_flags)

    # Print report
    print("\n" + manager.get_case_report(case))


if __name__ == "__main__":
    demo()
