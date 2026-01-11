"""
Save File Diff Analyzer

Compares before/after save files to discover which flags changed.
Used for empirical offset discovery and formula verification.
"""
from __future__ import annotations

from pathlib import Path
from dataclasses import dataclass, field
from typing import List, Dict, Optional, Tuple, Any
from .save_parser import SaveParser, SlotData
from .flag_formulas import FlagFormulas


@dataclass
class FlagChange:
    """A single flag change detected between saves."""
    byte_offset: int
    bit_position: int
    direction: str              # "SET" or "CLEARED"

    # Calculated flag IDs (may be multiple due to formula ambiguity)
    possible_flag_ids: List[int] = field(default_factory=list)

    # Metadata
    before_byte: str = ""
    after_byte: str = ""


@dataclass
class DiffResult:
    """Result of comparing two save files."""
    before_file: Path
    after_file: Path
    slot_index: int

    # Event flags changes
    flag_changes: List[FlagChange]
    total_bytes_changed: int
    total_flags_changed: int

    # Slot metadata
    before_validation_score: int
    after_validation_score: int
    before_graces: List[str]
    after_graces: List[str]

    # New graces discovered
    new_graces: List[str] = field(default_factory=list)


class DiffAnalyzer:
    """
    Analyzes differences between save files to discover flag offsets.

    Usage:
        analyzer = DiffAnalyzer()
        result = analyzer.compare(
            "before.sl2", "after.sl2",
            slot_index=1
        )
        for change in result.flag_changes:
            print(f"Flag at byte {change.byte_offset}, bit {change.bit_position}: {change.direction}")
    """

    def __init__(self):
        self.parser = SaveParser()
        self.formulas = FlagFormulas()

    def compare(
        self,
        before_file: str | Path,
        after_file: str | Path,
        slot_index: int = 0,
        focus_offset_range: Optional[Tuple[int, int]] = None
    ) -> DiffResult:
        """
        Compare two save files and identify flag changes.

        Args:
            before_file: Path to save before action
            after_file: Path to save after action
            slot_index: Character slot to compare
            focus_offset_range: Optional (start, end) to limit search

        Returns:
            DiffResult with all detected changes
        """
        before_path = Path(before_file)
        after_path = Path(after_file)

        # Parse both saves
        before_save = self.parser.parse(before_path, [slot_index])
        after_save = self.parser.parse(after_path, [slot_index])

        if not before_save.slots or not after_save.slots:
            raise ValueError(f"Slot {slot_index} not found in one or both saves")

        before_slot = before_save.slots[0]
        after_slot = after_save.slots[0]

        # Compare event flags
        changes = self._find_flag_changes(
            before_slot.event_flags,
            after_slot.event_flags,
            focus_offset_range
        )

        # Calculate reverse flag IDs for each change
        for change in changes:
            change.possible_flag_ids = self._reverse_calculate_flag_ids(
                change.byte_offset,
                change.bit_position
            )

        # Find new graces
        new_graces = [g for g in after_slot.validated_graces if g not in before_slot.validated_graces]

        return DiffResult(
            before_file=before_path,
            after_file=after_path,
            slot_index=slot_index,
            flag_changes=changes,
            total_bytes_changed=sum(1 for c in changes),
            total_flags_changed=len(changes),
            before_validation_score=before_slot.validation_score,
            after_validation_score=after_slot.validation_score,
            before_graces=before_slot.validated_graces,
            after_graces=after_slot.validated_graces,
            new_graces=new_graces
        )

    def _find_flag_changes(
        self,
        before: bytes,
        after: bytes,
        focus_range: Optional[Tuple[int, int]] = None
    ) -> List[FlagChange]:
        """Find all flag changes between two event flag arrays."""
        changes = []

        start = focus_range[0] if focus_range else 0
        end = focus_range[1] if focus_range else min(len(before), len(after))

        for byte_off in range(start, end):
            if byte_off >= len(before) or byte_off >= len(after):
                break

            if before[byte_off] != after[byte_off]:
                before_byte = before[byte_off]
                after_byte = after[byte_off]

                for bit in range(8):
                    before_bit = (before_byte >> bit) & 1
                    after_bit = (after_byte >> bit) & 1

                    if before_bit != after_bit:
                        # Standard bit position (7 - bit for big-endian style)
                        bit_pos = 7 - bit

                        changes.append(FlagChange(
                            byte_offset=byte_off,
                            bit_position=bit_pos,
                            direction="SET" if after_bit else "CLEARED",
                            before_byte=f"0x{before_byte:02X}",
                            after_byte=f"0x{after_byte:02X}"
                        ))

        return changes

    def _reverse_calculate_flag_ids(
        self,
        byte_offset: int,
        bit_position: int
    ) -> List[int]:
        """
        Reverse-calculate possible flag IDs from a byte offset and bit.

        Returns multiple candidates because different flag ranges may
        map to the same offset.
        """
        candidates = []

        # Reverse block-based calculation
        for block_start, config in self.formulas.BLOCK_BASES.items():
            # offset = base + (flag - block_start) / 8
            # flag - block_start = (offset - base) * 8
            # flag = block_start + (offset - base) * 8 + bit_correction

            relative_offset = byte_offset - config.base_offset
            if 0 <= relative_offset < config.block_size // 8:
                base_flag = block_start + relative_offset * 8
                # Account for bit position
                flag_id = base_flag + (7 - bit_position)
                if block_start <= flag_id < block_start + config.block_size:
                    candidates.append(flag_id)

        # Reverse tile-based calculation
        tc = self.formulas.TILE_CONFIG
        if byte_offset >= tc.base_offset:
            relative = byte_offset - tc.base_offset

            # offset = base + ((row - row_base) * slots_per_row + (col - col_base)) * bytes_per_slot + local_id // 8
            # relative = ((row - row_base) * slots_per_row + (col - col_base)) * bytes_per_slot + local_id // 8

            tile_slot = relative // tc.bytes_per_slot
            local_byte = relative % tc.bytes_per_slot

            # Calculate row and col from tile_slot
            row_offset = tile_slot // tc.slots_per_row
            col_offset = tile_slot % tc.slots_per_row

            row = tc.row_base + row_offset
            col = tc.col_base + col_offset

            # Calculate local_id from local_byte and bit
            local_id_base = local_byte * 8
            local_id = local_id_base + (7 - bit_position)

            if 0 <= local_id < tc.max_local_id:
                # Construct 10-digit flag ID
                flag_id = 1_000_000_000 + row * 100_000_000 + col * 1_000_000 + local_id
                # Validate it's in expected range
                if 33 <= row <= 54 and 31 <= col <= 58:
                    candidates.append(flag_id)

        return candidates

    def analyze_with_expected_flags(
        self,
        before_file: str | Path,
        after_file: str | Path,
        slot_index: int,
        expected_flags: List[int]
    ) -> Dict[int, Dict[str, Any]]:
        """
        Compare saves and check if expected flags were set.

        Returns a dict mapping expected flag IDs to their verification status.
        """
        result = self.compare(before_file, after_file, slot_index)

        analysis = {}
        for flag_id in expected_flags:
            # Calculate expected offset
            formulas = self.formulas.calculate_offset(flag_id)

            best_formula = None
            expected_offset = None
            expected_bit = None

            for name in ["block", "tile", "dungeon"]:
                if name in formulas and formulas[name].is_valid:
                    best_formula = name
                    expected_offset = formulas[name].byte_offset
                    expected_bit = formulas[name].bit_position
                    break

            # Check if this offset was in the changes
            found_change = None
            for change in result.flag_changes:
                if change.byte_offset == expected_offset and change.bit_position == expected_bit:
                    found_change = change
                    break

            # Check if flag ID appears in any change's possible IDs
            found_in_reverse = False
            actual_change = None
            for change in result.flag_changes:
                if flag_id in change.possible_flag_ids:
                    found_in_reverse = True
                    actual_change = change
                    break

            analysis[flag_id] = {
                "expected_offset": expected_offset,
                "expected_bit": expected_bit,
                "formula_used": best_formula,
                "formula_valid": best_formula is not None,
                "found_at_expected": found_change is not None,
                "found_in_reverse": found_in_reverse,
                "actual_offset": actual_change.byte_offset if actual_change else None,
                "actual_bit": actual_change.bit_position if actual_change else None,
                "matches": (found_change is not None) or (
                    found_in_reverse and actual_change and
                    actual_change.byte_offset == expected_offset and
                    actual_change.bit_position == expected_bit
                )
            }

        return analysis

    def print_diff_report(self, result: DiffResult, max_changes: int = 50):
        """Print a formatted diff report."""
        print("=" * 70)
        print("SAVE FILE DIFF REPORT")
        print("=" * 70)
        print(f"\nBefore: {result.before_file.name}")
        print(f"After:  {result.after_file.name}")
        print(f"Slot:   {result.slot_index}")

        print(f"\nValidation scores: {result.before_validation_score} -> {result.after_validation_score}")
        print(f"Graces before: {', '.join(result.before_graces) or 'None'}")
        print(f"Graces after:  {', '.join(result.after_graces) or 'None'}")

        if result.new_graces:
            print(f"\nNEW GRACES DISCOVERED: {', '.join(result.new_graces)}")

        print(f"\n{'=' * 70}")
        print(f"Total flags changed: {result.total_flags_changed}")
        print(f"{'=' * 70}")

        # Group by SET vs CLEARED
        set_flags = [c for c in result.flag_changes if c.direction == "SET"]
        cleared_flags = [c for c in result.flag_changes if c.direction == "CLEARED"]

        print(f"\nFlags SET: {len(set_flags)}")
        print(f"Flags CLEARED: {len(cleared_flags)}")

        if set_flags:
            print(f"\n--- FLAGS SET ---")
            for i, change in enumerate(set_flags[:max_changes]):
                candidates = ", ".join(str(f) for f in change.possible_flag_ids[:3])
                if len(change.possible_flag_ids) > 3:
                    candidates += f" (+{len(change.possible_flag_ids) - 3} more)"
                print(f"  byte={change.byte_offset:6d} bit={change.bit_position}  | candidates: {candidates}")

            if len(set_flags) > max_changes:
                print(f"  ... and {len(set_flags) - max_changes} more")

        if cleared_flags:
            print(f"\n--- FLAGS CLEARED ---")
            for i, change in enumerate(cleared_flags[:max_changes]):
                candidates = ", ".join(str(f) for f in change.possible_flag_ids[:3])
                print(f"  byte={change.byte_offset:6d} bit={change.bit_position}  | candidates: {candidates}")

            if len(cleared_flags) > max_changes:
                print(f"  ... and {len(cleared_flags) - max_changes} more")


# Convenience functions
def diff_saves(before: str, after: str, slot: int = 0) -> DiffResult:
    """Compare two save files."""
    analyzer = DiffAnalyzer()
    return analyzer.compare(before, after, slot)


if __name__ == "__main__":
    import sys

    if len(sys.argv) < 3:
        print("Usage: python diff_analyzer.py <before.sl2> <after.sl2> [slot_index]")
        sys.exit(1)

    before = sys.argv[1]
    after = sys.argv[2]
    slot = int(sys.argv[3]) if len(sys.argv) > 3 else 0

    analyzer = DiffAnalyzer()
    result = analyzer.compare(before, after, slot)
    analyzer.print_diff_report(result)
