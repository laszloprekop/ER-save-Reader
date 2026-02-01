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
from .ground_truth_loader import (
    get_tile_config,
    load_block_bases,
    load_dungeon_bases,
    calculate_block_offset,
    calculate_tile_offset,
    calculate_dungeon_offset,
)


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
class VerificationDiagnostics:
    """Diagnostic information when verification fails or needs investigation."""
    status: str = "unknown"  # "verified", "investigation_needed", "failed"
    expected_flag_found: bool = False
    inventory_changed: bool = False
    bytes_changed_in_region: int = 0
    possible_causes: List[str] = field(default_factory=list)
    suggested_flags_to_check: List[int] = field(default_factory=list)
    character_context: Dict[str, Any] = field(default_factory=dict)
    notes: str = ""


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

    # Character context (discovered graces across all regions)
    before_context: Dict[str, Any] = field(default_factory=dict)
    after_context: Dict[str, Any] = field(default_factory=dict)

    # Verification diagnostics (populated when analyzing expected flags)
    diagnostics: Optional[VerificationDiagnostics] = None


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
        # Load ground truth data for reverse calculations
        self._block_bases = load_block_bases()
        self._tile_config = get_tile_config()
        self._dungeon_bases = load_dungeon_bases()

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

        # Extract full character context (discovered graces across ALL regions)
        before_context = self.parser.extract_character_context(before_slot)
        after_context = self.parser.extract_character_context(after_slot)

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
            new_graces=new_graces,
            before_context=before_context,
            after_context=after_context,
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

        Note: bit_position is stored as (7 - physical_bit) where physical_bit
        is the actual bit index in the byte (0-7 from right).
        """
        candidates = []

        # Reverse block-based calculation
        # Formula: bit_position = 7 - (flag_id % 8)
        # So: flag_id % 8 = 7 - bit_position
        for block_start, config in self._block_bases.items():
            base_offset = config["base_offset"]
            block_size = config.get("block_size", 1000)
            relative_offset = byte_offset - base_offset
            if 0 <= relative_offset < block_size // 8:
                base_flag = block_start + relative_offset * 8
                # bit_position = 7 - (flag_id % 8), so flag_id % 8 = 7 - bit_position
                flag_id = base_flag + (7 - bit_position)
                if block_start <= flag_id < block_start + block_size:
                    candidates.append(flag_id)

        # Reverse tile-based calculation
        # Format: 10XXYYZZZZ where XX=row, YY=col, ZZZZ=local_id
        # Bit formula: bit_position = 7 - (local_id % 8)
        tc = self._tile_config
        base_offset = tc.get("base_offset", 485330)
        bytes_per_slot = tc.get("bytes_per_slot", 875)
        slots_per_row = tc.get("slots_per_row", 40)
        row_base = tc.get("row_base", 33)
        col_base = tc.get("col_base", 30)
        max_local_id = tc.get("max_local_id", 6999)

        if byte_offset >= base_offset:
            relative = byte_offset - base_offset

            tile_slot = relative // bytes_per_slot
            local_byte = relative % bytes_per_slot

            # Calculate row and col from tile_slot
            row_offset = tile_slot // slots_per_row
            col_offset = tile_slot % slots_per_row

            row = row_base + row_offset
            col = col_base + col_offset

            # Bit formula: bit_position = 7 - (local_id % 8)
            # So: local_id % 8 = 7 - bit_position
            local_id_base = local_byte * 8
            local_id = local_id_base + (7 - bit_position)

            if 0 <= local_id < max_local_id:
                # Construct 10-digit flag ID: 10XXYYZZZZ
                # 10 (prefix) + XX (row) + YY (col) + ZZZZ (local)
                flag_id = int(f"10{row:02d}{col:02d}{local_id:04d}")
                # Validate it's in reasonable range (Elden Ring map bounds)
                if 30 <= row <= 60 and 30 <= col <= 60:
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
            # Calculate expected offset using ground_truth_loader functions
            best_formula = None
            expected_offset = None
            expected_bit = None

            # Try block formula (5-6 digit flags)
            block_result = calculate_block_offset(flag_id)
            if block_result:
                best_formula = "block"
                expected_offset, expected_bit = block_result

            # Try tile formula (10-digit base game flags)
            if best_formula is None:
                tile_result = calculate_tile_offset(flag_id)
                if tile_result:
                    best_formula = "tile"
                    expected_offset, expected_bit = tile_result

            # Try dungeon formula (8-digit flags)
            if best_formula is None:
                dungeon_result = calculate_dungeon_offset(flag_id)
                if dungeon_result:
                    best_formula = "dungeon"
                    expected_offset, expected_bit = dungeon_result

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

            # Build diagnostic info for failed verifications
            diagnostics = None
            matches = (found_change is not None) or (
                found_in_reverse and actual_change and
                actual_change.byte_offset == expected_offset and
                actual_change.bit_position == expected_bit
            )

            if not matches:
                diagnostics = self._build_verification_diagnostics(
                    flag_id, expected_offset, expected_bit, best_formula,
                    result, found_change, actual_change
                )

            analysis[flag_id] = {
                "expected_offset": expected_offset,
                "expected_bit": expected_bit,
                "formula_used": best_formula,
                "formula_valid": best_formula is not None,
                "found_at_expected": found_change is not None,
                "found_in_reverse": found_in_reverse,
                "actual_offset": actual_change.byte_offset if actual_change else None,
                "actual_bit": actual_change.bit_position if actual_change else None,
                "matches": matches,
                "diagnostics": diagnostics,
            }

        return analysis

    def _build_verification_diagnostics(
        self,
        flag_id: int,
        expected_offset: Optional[int],
        expected_bit: Optional[int],
        formula_used: Optional[str],
        diff_result: DiffResult,
        found_change: Optional[FlagChange],
        actual_change: Optional[FlagChange],
    ) -> VerificationDiagnostics:
        """
        Build diagnostic information when verification fails.

        This helps identify WHY a flag wasn't found at the expected location
        and suggests investigation paths.
        """
        diag = VerificationDiagnostics(status="investigation_needed")
        possible_causes = []
        suggested_flags = []

        # Check if inventory changed (GaItem count difference)
        before_gaitem = diff_result.before_context.get("gaitem_count", 0)
        after_gaitem = diff_result.after_context.get("gaitem_count", 0)
        diag.inventory_changed = after_gaitem != before_gaitem

        # Check bytes changed in the expected region (+-100 bytes)
        if expected_offset is not None:
            region_changes = [
                c for c in diff_result.flag_changes
                if abs(c.byte_offset - expected_offset) <= 100
            ]
            diag.bytes_changed_in_region = len(region_changes)

            # Suggest nearby flag IDs
            for change in region_changes[:5]:
                suggested_flags.extend(change.possible_flag_ids[:2])

        # Analyze possible causes
        if expected_offset is None:
            possible_causes.append("No valid formula for this flag ID")

        if diff_result.total_flags_changed == 0:
            possible_causes.append("No flags changed between saves - capture may have failed")
        elif diag.bytes_changed_in_region == 0 and expected_offset:
            possible_causes.append(f"Expected region (byte {expected_offset}) had no changes")

        # Check character context for NPC/boss drop issues
        if formula_used == "dungeon":
            area_id = flag_id // 1_000_000
            area_graces = diff_result.after_context.get("discovered_graces", {})

            # Check if character has been to this dungeon
            if area_id == 16:  # Volcano Manor
                vm_graces = area_graces.get("volcano_manor_graces", [])
                if not vm_graces:
                    possible_causes.append(
                        "Character hasn't discovered Volcano Manor graces - "
                        "may not have reached this area"
                    )
                else:
                    diag.character_context["volcano_manor_graces"] = len(vm_graces)

        # Check if this might be an NPC drop (uses defeat flag instead of item flag)
        # NPC drops typically use EMEVD event flags, not ItemLotParam flags
        if 16000000 <= flag_id < 17000000:  # Volcano Manor range
            possible_causes.append(
                "This may be an NPC/boss drop that uses EMEVD defeat flag "
                "instead of ItemLotParam flag. Check common.emevd.js for event 90005792"
            )
            # Suggest the potential defeat flag (usually XX000180 pattern)
            defeat_flag = (flag_id // 1000) * 1000 + 180
            suggested_flags.append(defeat_flag)

        diag.possible_causes = possible_causes
        diag.suggested_flags_to_check = list(set(suggested_flags))[:10]

        if possible_causes:
            diag.status = "investigation_needed"
        else:
            diag.status = "failed"

        return diag

    def print_diff_report(self, result: DiffResult, max_changes: int = 50):
        """Print a formatted diff report with character context."""
        print("=" * 70)
        print("SAVE FILE DIFF REPORT")
        print("=" * 70)
        print(f"\nBefore: {result.before_file.name}")
        print(f"After:  {result.after_file.name}")
        print(f"Slot:   {result.slot_index}")

        # Character context (full progression info)
        print(f"\n--- CHARACTER CONTEXT ---")
        print(f"Character: {result.after_context.get('character_name', 'Unknown')}")
        print(f"GaItem count: {result.before_context.get('gaitem_count', 0)} -> {result.after_context.get('gaitem_count', 0)}")

        # Show discovered graces by region
        after_graces = result.after_context.get("discovered_graces", {})
        total_graces = sum(len(g) for g in after_graces.values())
        print(f"Total discovered graces: {total_graces}")

        if after_graces:
            for region, graces in after_graces.items():
                if graces:
                    print(f"  {region}: {len(graces)}")

        # Show progression markers
        progression = result.after_context.get("progression_markers", {})
        if progression:
            active_markers = [k for k, v in progression.items() if v]
            if active_markers:
                print(f"Progression: {', '.join(active_markers)}")

        print(f"\n--- VALIDATION FLAGS (EF offset check only) ---")
        print(f"Validation scores: {result.before_validation_score} -> {result.after_validation_score}")
        print(f"Validation graces before: {', '.join(result.before_graces) or 'None'}")
        print(f"Validation graces after:  {', '.join(result.after_graces) or 'None'}")

        if result.new_graces:
            print(f"\nNEW VALIDATION GRACES: {', '.join(result.new_graces)}")

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
