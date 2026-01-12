"""
Elden Ring Save File Parser

Parses the BND4 container format to extract character slot data and event flags.
Handles the variable-size GaItems section to correctly locate event flags.

Based on ER-Save-Editor Rust implementation and empirical analysis.
"""
from __future__ import annotations

import struct
from pathlib import Path
from dataclasses import dataclass, field
from typing import List, Optional, Dict, Tuple, BinaryIO, Union
from .flag_formulas import FlagFormulas


# ============================================================================
# CONSTANTS
# ============================================================================

# Save file structure
# NOTE: Slot offsets are NOT at fixed intervals! They must be read from BND4 entries.
# Each BND4 entry points to a 16-byte checksum header, followed by the actual slot data.
BND4_HEADER_SIZE = 0x40             # BND4 header before file entries
BND4_ENTRY_SIZE = 0x20              # Each BND4 file entry is 32 bytes
BND4_ENTRY_OFFSET_POS = 0x10        # Offset within entry for data offset (4-byte LE)
SLOT_CHECKSUM_SIZE = 16             # 16-byte checksum header before slot data
SLOT_SIZE = 0x280000                # 2,621,440 bytes per slot (approximate)
SLOT_COUNT = 10                     # Maximum 10 character slots
EVENT_FLAGS_SIZE = 0x1BF99F         # 1,833,375 bytes

# GaItems structure
GAITEM_SIZE = 48                    # Each GaItem entry is 48 bytes
GAITEM_MAX_COUNT = 0x1400           # 5,120 entries max
GAITEM_HEADER_SIZE = 8              # 4-byte count + 4-byte padding

# Section offsets (relative to slot start, AFTER 16-byte checksum header)
FIXED_HEADER_SIZE = 0x20            # Version (4) + MapID (4) + Padding (24)

# EventFlags offset VARIES per slot due to variable-size GaItems section
# The Rust code's fixed offset (0x1a104) is incorrect for our saves
# Empirically, the offset is around 0x12B00-0x13800 depending on GaItems count
EVENT_FLAGS_SEARCH_MIN = 0x10000  # Minimum search offset
EVENT_FLAGS_SEARCH_MAX = 0x20000  # Maximum search offset

# Validation flags for verifying event flags offset is correct
VALIDATION_FLAGS = [
    (71800, 2725, 7, "Cave of Knowledge"),
    (71801, 2725, 6, "Stranded Graveyard"),
    (76100, 3262, 3, "The First Step"),
    (76101, 3262, 2, "Church of Elleh"),
]


@dataclass
class SlotData:
    """Parsed character slot data."""
    slot_index: int
    slot_offset: int                    # Absolute offset in save file
    version: int                        # Save format version
    map_id: int                         # Current map

    # GaItems (variable size)
    gaitem_count: int                   # Number of GaItems
    gaitem_offset: int                  # Offset of GaItems in slot
    gaitem_size: int                    # Total size of GaItems section

    # Event flags
    event_flags_offset: int             # Offset of event flags in slot
    event_flags_offset_absolute: int    # Absolute offset in file
    event_flags: bytes                  # Raw event flags data

    # Validation
    validation_score: int = 0           # How many validation flags matched
    validated_graces: List[str] = field(default_factory=list)

    # Character info (if available)
    character_name: Optional[str] = None


@dataclass
class ParsedSave:
    """Complete parsed save file."""
    file_path: Path
    file_size: int
    slots: List[SlotData]
    active_slots: List[int]             # Indices of slots with characters


class SaveParser:
    """
    Parser for Elden Ring save files (*.sl2).

    Handles:
    - BND4 container format
    - Variable-size GaItems section
    - Event flags extraction
    - Multiple character slots
    """

    def __init__(self):
        self.formulas = FlagFormulas()

    def _read_bnd4_slot_offsets(self, data: bytes) -> List[int]:
        """
        Read slot data offsets from BND4 file entries.

        The BND4 header contains 12 file entries (10 slots + 2 other files).
        Each entry has the data offset at position 0x10 (4-byte little-endian).
        The offset points to a 16-byte checksum header; actual slot data is +16 bytes.
        """
        offsets = []
        for i in range(SLOT_COUNT):
            entry_offset = BND4_HEADER_SIZE + (i * BND4_ENTRY_SIZE) + BND4_ENTRY_OFFSET_POS
            if entry_offset + 4 <= len(data):
                # Read the BND4 entry's data offset
                bnd4_offset = struct.unpack_from('<I', data, entry_offset)[0]
                # Add 16 bytes to skip the checksum header
                slot_offset = bnd4_offset + SLOT_CHECKSUM_SIZE
                offsets.append(slot_offset)
            else:
                offsets.append(0)
        return offsets

    def parse(self, filepath: str | Path, slots_to_parse: Optional[List[int]] = None) -> ParsedSave:
        """
        Parse a save file and extract all character slot data.

        Args:
            filepath: Path to ER0000.sl2 save file
            slots_to_parse: List of slot indices to parse (default: all)

        Returns:
            ParsedSave with all slot data
        """
        filepath = Path(filepath)

        with open(filepath, 'rb') as f:
            data = f.read()

        # Verify BND4 header
        if data[:4] != b'BND4':
            raise ValueError(f"Not a valid BND4 file: {filepath}")

        # Read slot offsets from BND4 entries
        slot_offsets = self._read_bnd4_slot_offsets(data)

        # Parse each slot
        slots = []
        active_slots = []

        indices = slots_to_parse or range(SLOT_COUNT)
        for i in indices:
            slot_offset = slot_offsets[i] if i < len(slot_offsets) else 0
            slot = self._parse_slot(data, i, slot_offset)
            if slot:
                slots.append(slot)
                if slot.validation_score > 0:  # Has at least one grace discovered
                    active_slots.append(i)

        return ParsedSave(
            file_path=filepath,
            file_size=len(data),
            slots=slots,
            active_slots=active_slots
        )

    def _parse_slot(self, data: bytes, slot_index: int, slot_offset: int) -> Optional[SlotData]:
        """Parse a single character slot.

        Args:
            data: Full save file bytes
            slot_index: Index of slot (0-9)
            slot_offset: Absolute offset of slot data (after 16-byte checksum header)
        """
        # Check if slot offset is valid
        if slot_offset == 0:
            return None

        # Check if slot is within file bounds
        if slot_offset + SLOT_SIZE > len(data):
            # Allow partial reads for last valid data
            slot_data = data[slot_offset:]
            if len(slot_data) < FIXED_HEADER_SIZE:
                return None
        else:
            slot_data = data[slot_offset:slot_offset + SLOT_SIZE]

        # Parse header
        version = struct.unpack_from('<I', slot_data, 0)[0]
        map_id = struct.unpack_from('<I', slot_data, 4)[0]

        # Skip if slot appears empty (version 0)
        if version == 0:
            return None

        # Calculate GaItems size
        # GaItems starts at offset 0x20 (after fixed header)
        gaitem_offset = FIXED_HEADER_SIZE
        gaitem_count = struct.unpack_from('<I', slot_data, gaitem_offset)[0]

        # Sanity check
        if gaitem_count > GAITEM_MAX_COUNT:
            # Probably empty or corrupted slot
            gaitem_count = 0

        gaitem_size = GAITEM_HEADER_SIZE + (gaitem_count * GAITEM_SIZE)

        # Find EventFlags offset by searching for validation pattern
        event_flags_offset = self._find_event_flags_offset(slot_data)

        # Extract event flags
        event_flags_end = event_flags_offset + EVENT_FLAGS_SIZE
        if event_flags_end <= len(slot_data):
            event_flags = slot_data[event_flags_offset:event_flags_end]
        else:
            event_flags = slot_data[event_flags_offset:]

        # Validate using anchor flags
        validation_score, validated_graces = self._validate_event_flags(event_flags)

        return SlotData(
            slot_index=slot_index,
            slot_offset=slot_offset,
            version=version,
            map_id=map_id,
            gaitem_count=gaitem_count,
            gaitem_offset=gaitem_offset,
            gaitem_size=gaitem_size,
            event_flags_offset=event_flags_offset,
            event_flags_offset_absolute=slot_offset + event_flags_offset,
            event_flags=event_flags,
            validation_score=validation_score,
            validated_graces=validated_graces
        )

    def _find_event_flags_offset(self, slot_data: bytes) -> int:
        """
        Find the event flags section offset using validation flag patterns.

        The offset varies due to variable-size GaItems section (depends on inventory).
        Empirically, the offset is around 0x12B00-0x13800 for our test saves.
        We search for the offset where at least 2 validation flags match (First Step + Church of Elleh).
        """
        best_offset = 0x12B00  # Default fallback based on empirical testing
        best_score = 0

        # Search in 4-byte increments for speed
        for test_offset in range(EVENT_FLAGS_SEARCH_MIN, min(EVENT_FLAGS_SEARCH_MAX, len(slot_data) - EVENT_FLAGS_SIZE), 4):
            score = 0
            for flag_id, byte_off, bit_pos, name in VALIDATION_FLAGS:
                abs_pos = test_offset + byte_off
                if abs_pos < len(slot_data):
                    if (slot_data[abs_pos] & (1 << bit_pos)) != 0:
                        score += 1

            if score > best_score:
                best_score = score
                best_offset = test_offset

                if best_score == len(VALIDATION_FLAGS):
                    # Perfect match - stop searching
                    break

        return best_offset

    def _validate_event_flags(self, event_flags: bytes) -> Tuple[int, List[str]]:
        """Validate event flags section using anchor flags."""
        score = 0
        matched = []

        for flag_id, byte_off, bit_pos, name in VALIDATION_FLAGS:
            if byte_off < len(event_flags):
                if (event_flags[byte_off] & (1 << bit_pos)) != 0:
                    score += 1
                    matched.append(name)

        return score, matched

    def check_flag(self, event_flags: bytes, flag_id: int) -> Tuple[bool, Optional[str]]:
        """
        Check if a flag is set in the event flags data.

        Returns:
            (is_set, error_message)
        """
        results = self.formulas.calculate_offset(flag_id)

        # Try each formula that returns a valid result
        for formula_name in ["block", "tile", "dungeon"]:
            if formula_name in results:
                result = results[formula_name]
                if result.is_valid:
                    byte_off = result.byte_offset
                    bit_pos = result.bit_position

                    if byte_off is not None and bit_pos is not None:
                        if byte_off < len(event_flags):
                            is_set = (event_flags[byte_off] & (1 << bit_pos)) != 0
                            return (is_set, None)
                        else:
                            return (False, f"Offset {byte_off} exceeds event flags size")

        # No valid formula
        error = "; ".join([
            f"{name}: {r.error_message}"
            for name, r in results.items()
            if not r.is_valid and r.error_message
        ])
        return (False, error or "No applicable formula")

    def check_flag_at_offset(self, event_flags: bytes, byte_offset: int, bit_position: int) -> bool:
        """Check if a flag is set at a specific offset (bypassing formulas)."""
        if byte_offset < len(event_flags):
            return (event_flags[byte_offset] & (1 << bit_position)) != 0
        return False

    def scan_flag_changes(
        self,
        before_flags: bytes,
        after_flags: bytes,
        start_offset: int = 0,
        end_offset: Optional[int] = None
    ) -> List[Dict]:
        """
        Scan for flag changes between two event flags snapshots.

        Returns list of changes with byte offset, bit position, and direction (set/cleared).
        """
        end = end_offset or min(len(before_flags), len(after_flags))
        changes = []

        for byte_off in range(start_offset, end):
            if before_flags[byte_off] != after_flags[byte_off]:
                for bit in range(8):
                    before_bit = (before_flags[byte_off] >> bit) & 1
                    after_bit = (after_flags[byte_off] >> bit) & 1

                    if before_bit != after_bit:
                        # Convert to standard bit position (7 - bit)
                        bit_pos = 7 - bit

                        changes.append({
                            "byte_offset": byte_off,
                            "bit_position": bit_pos,
                            "direction": "SET" if after_bit else "CLEARED",
                            "before_byte": hex(before_flags[byte_off]),
                            "after_byte": hex(after_flags[byte_off]),
                        })

        return changes


# Convenience function
def load_save(filepath: str | Path) -> ParsedSave:
    """Load and parse a save file."""
    parser = SaveParser()
    return parser.parse(filepath)


if __name__ == "__main__":
    import sys

    if len(sys.argv) < 2:
        print("Usage: python save_parser.py <save_file.sl2> [slot_index]")
        sys.exit(1)

    save_path = sys.argv[1]
    slot_filter = [int(sys.argv[2])] if len(sys.argv) > 2 else None

    print(f"Parsing: {save_path}")
    result = load_save(save_path)

    print(f"\nFile size: {result.file_size:,} bytes")
    print(f"Total slots parsed: {len(result.slots)}")
    print(f"Active slots: {result.active_slots}")

    for slot in result.slots:
        print(f"\n{'=' * 60}")
        print(f"Slot {slot.slot_index}")
        print(f"{'=' * 60}")
        print(f"  Version: {slot.version}")
        print(f"  Map ID: {slot.map_id}")
        print(f"  GaItem count: {slot.gaitem_count}")
        print(f"  GaItem size: {slot.gaitem_size:,} bytes")
        print(f"  Event flags offset: 0x{slot.event_flags_offset:X}")
        print(f"  Event flags absolute: 0x{slot.event_flags_offset_absolute:X}")
        print(f"  Event flags size: {len(slot.event_flags):,} bytes")
        print(f"  Validation score: {slot.validation_score}/4")
        print(f"  Validated graces: {', '.join(slot.validated_graces) or 'None'}")
