#!/usr/bin/env python3
"""
Event Flag Schema and Allocation Bitmap System

This module provides schema-based detection of event flag allocation.
Instead of trying to detect boundaries dynamically, it uses known flag IDs
from game data to create a schema, then probes the save data to generate
an allocation bitmap showing which positions are actually used vs padding.

Terminology:
- Schema: A predefined map of known flag IDs to their expected byte offsets
- Allocation Bitmap: The result showing which schema positions have real data
- Sparse Allocation: When the game only allocates memory for flags actually used

Usage:
    schema = BlockSchema(520000, base_offset=1341)
    schema.load_flags_from_extracted('scripts/extracted_event_flags.json')
    bitmap = schema.probe_allocation(save_path, slots=[0, 1, 2, 3, 4])
    print(bitmap.summary())
"""

import json
import sys
from pathlib import Path
from dataclasses import dataclass, field
from typing import Dict, List, Optional, Tuple, Set, Union
from enum import Enum


class AllocationStatus(Enum):
    """Status of a flag position in the save data."""
    ALLOCATED = "allocated"      # Has real data (varies between slots or has non-0xFF value)
    UNALLOCATED = "unallocated"  # Padding (0xFF in all slots)
    PARTIAL = "partial"          # Mixed signals (needs investigation)
    UNKNOWN = "unknown"          # Couldn't determine


@dataclass
class FlagDefinition:
    """A known flag's position as defined in the schema."""
    flag_id: int
    item_name: str
    byte_offset: int
    bit_position: int
    category: str = ""
    source: str = ""  # Where we learned about this flag


@dataclass
class AllocationEntry:
    """Result of probing a single flag position."""
    flag_id: int
    item_name: str
    expected_offset: int
    expected_bit: int
    status: AllocationStatus
    slot_values: Dict[int, int] = field(default_factory=dict)  # slot -> byte value
    notes: str = ""


@dataclass
class AllocationBitmap:
    """
    Complete allocation bitmap for a block schema.

    Shows which flag positions are allocated (have real data) vs
    unallocated (padding gaps in sparse allocation).
    """
    block_start: int
    base_offset: int
    total_flags: int
    allocated: List[AllocationEntry] = field(default_factory=list)
    unallocated: List[AllocationEntry] = field(default_factory=list)
    partial: List[AllocationEntry] = field(default_factory=list)

    def summary(self) -> str:
        """Generate a human-readable summary."""
        lines = [
            f"Allocation Bitmap for Block {self.block_start}",
            "=" * 60,
            f"Base offset: {self.base_offset}",
            f"Total flags in schema: {self.total_flags}",
            f"Allocated (trackable): {len(self.allocated)}",
            f"Unallocated (sparse gaps): {len(self.unallocated)}",
            f"Partial (needs investigation): {len(self.partial)}",
            "",
        ]

        if self.allocated:
            lines.append("ALLOCATED FLAGS:")
            for r in self.allocated[:10]:
                lines.append(f"  {r.flag_id}: {r.item_name} (offset {r.expected_offset})")
            if len(self.allocated) > 10:
                lines.append(f"  ... and {len(self.allocated) - 10} more")
            lines.append("")

        if self.unallocated:
            lines.append("UNALLOCATED FLAGS (sparse gaps):")
            for r in self.unallocated:
                lines.append(f"  {r.flag_id}: {r.item_name} (offset {r.expected_offset})")
            lines.append("")

        return "\n".join(lines)

    def get_bitmap(self) -> Dict[int, bool]:
        """Return a simple flag_id -> is_allocated mapping."""
        result = {}
        for r in self.allocated:
            result[r.flag_id] = True
        for r in self.unallocated:
            result[r.flag_id] = False
        for r in self.partial:
            result[r.flag_id] = None  # Unknown
        return result

    def get_trackable_flags(self) -> List[int]:
        """Return list of flag IDs that can be tracked (allocated)."""
        return [r.flag_id for r in self.allocated]

    def get_untrackable_flags(self) -> List[int]:
        """Return list of flag IDs that cannot be tracked (unallocated)."""
        return [r.flag_id for r in self.unallocated]

    def is_trackable(self, flag_id: int) -> Optional[bool]:
        """Check if a specific flag is trackable. Returns None if not in schema."""
        bitmap = self.get_bitmap()
        return bitmap.get(flag_id)


class BlockSchema:
    """
    Schema defining known event flags for a block.

    A schema maps known flag IDs to their expected byte offsets based on
    the block formula. It can be probed against save data to generate an
    allocation bitmap showing which positions are actually used.
    """

    def __init__(self, block_start: int, base_offset: int, block_size: int = 1000):
        """
        Initialize a block schema.

        Args:
            block_start: First flag ID in the block (e.g., 520000)
            base_offset: Byte offset where this block starts in event_flags
            block_size: Number of flag IDs in the block (default 1000)
        """
        self.block_start = block_start
        self.base_offset = base_offset
        self.block_size = block_size
        self.flags: Dict[int, FlagDefinition] = {}

    def add_flag(self, flag_id: int, item_name: str, category: str = "", source: str = ""):
        """Add a known flag to the schema."""
        if not (self.block_start <= flag_id < self.block_start + self.block_size):
            return  # Flag not in this block

        relative = flag_id - self.block_start
        byte_offset = self.base_offset + relative // 8
        bit_position = 7 - (relative % 8)

        self.flags[flag_id] = FlagDefinition(
            flag_id=flag_id,
            item_name=item_name,
            byte_offset=byte_offset,
            bit_position=bit_position,
            category=category,
            source=source,
        )

    def load_flags_from_extracted(self, json_path: Union[str, Path]) -> int:
        """
        Load flags from extracted_event_flags.json.

        Returns:
            Number of flags loaded for this block
        """
        json_path = Path(json_path)
        if not json_path.exists():
            return 0

        data = json.load(open(json_path))
        flags_list = data.get('flags', [])

        count = 0
        for entry in flags_list:
            if not isinstance(entry, dict):
                continue

            flag_id = entry.get('flag_id')
            if flag_id is None:
                continue

            if self.block_start <= flag_id < self.block_start + self.block_size:
                self.add_flag(
                    flag_id=flag_id,
                    item_name=entry.get('name', f'Flag {flag_id}'),
                    category=entry.get('category', ''),
                    source=entry.get('source_file', 'extracted_event_flags.json'),
                )
                count += 1

        return count

    def load_flags_from_item_lot_param(self, xml_path: Union[str, Path]) -> int:
        """
        Load flags from ItemLotParam_map.param.xml.

        Returns:
            Number of flags loaded for this block
        """
        import re

        xml_path = Path(xml_path)
        if not xml_path.exists():
            return 0

        content = xml_path.read_text()
        pattern = r'<row id="(\d+)".*?getItemFlagId="(\d+)"'

        count = 0
        for match in re.finditer(pattern, content):
            row_id = int(match.group(1))
            flag_id = int(match.group(2))

            if self.block_start <= flag_id < self.block_start + self.block_size:
                self.add_flag(
                    flag_id=flag_id,
                    item_name=f'ItemLot {row_id}',
                    category='item_lot',
                    source='ItemLotParam_map.param.xml',
                )
                count += 1

        return count

    def probe_allocation(
        self,
        save_path: Union[str, Path],
        slots: List[int] = None,
    ) -> AllocationBitmap:
        """
        Probe the save file to generate an allocation bitmap.

        Compares the schema against actual save data to determine which
        flag positions are allocated (have real data) vs unallocated
        (0xFF padding in all slots).

        Args:
            save_path: Path to .sl2 save file
            slots: List of slot indices to check (default: [0, 1, 2, 3, 4])

        Returns:
            AllocationBitmap showing which flags are trackable
        """
        # Add project root to path for imports
        script_dir = Path(__file__).parent.parent.parent
        if str(script_dir) not in sys.path:
            sys.path.insert(0, str(script_dir))
        from scripts.verification.save_parser import SaveParser

        if slots is None:
            slots = [0, 1, 2, 3, 4]

        save_path = Path(save_path)
        parser = SaveParser()
        save = parser.parse(save_path, slots)

        # Collect event_flags from each slot
        slot_ef: Dict[int, bytes] = {}
        for slot in save.slots:
            slot_ef[slot.slot_index] = slot.event_flags

        bitmap = AllocationBitmap(
            block_start=self.block_start,
            base_offset=self.base_offset,
            total_flags=len(self.flags),
        )

        for flag_id, defn in sorted(self.flags.items()):
            # Read byte value from each slot
            slot_values = {}
            for slot_idx, ef in slot_ef.items():
                if defn.byte_offset < len(ef):
                    slot_values[slot_idx] = ef[defn.byte_offset]
                else:
                    slot_values[slot_idx] = 0

            # Determine allocation status
            values = list(slot_values.values())
            all_ff = all(v == 0xFF for v in values)
            all_same = len(set(values)) == 1

            if all_ff:
                status = AllocationStatus.UNALLOCATED
                notes = "0xFF in all slots - sparse gap"
            elif all_same:
                status = AllocationStatus.ALLOCATED
                notes = f"Same value (0x{values[0]:02X}) in all slots"
            else:
                status = AllocationStatus.ALLOCATED
                notes = "Differential across slots"

            entry = AllocationEntry(
                flag_id=flag_id,
                item_name=defn.item_name,
                expected_offset=defn.byte_offset,
                expected_bit=defn.bit_position,
                status=status,
                slot_values=slot_values,
                notes=notes,
            )

            if status == AllocationStatus.ALLOCATED:
                bitmap.allocated.append(entry)
            elif status == AllocationStatus.UNALLOCATED:
                bitmap.unallocated.append(entry)
            else:
                bitmap.partial.append(entry)

        return bitmap

    def get_allocation_boundaries(
        self,
        save_path: Union[str, Path],
        slots: List[int] = None,
    ) -> List[Tuple[int, int, str]]:
        """
        Find boundaries between allocated and unallocated regions.

        Returns:
            List of (start_flag, end_flag, status) tuples
        """
        bitmap = self.probe_allocation(save_path, slots)
        alloc_map = bitmap.get_bitmap()

        if not alloc_map:
            return []

        boundaries = []
        sorted_flags = sorted(alloc_map.keys())

        current_status = alloc_map[sorted_flags[0]]
        region_start = sorted_flags[0]

        for flag_id in sorted_flags[1:]:
            status = alloc_map[flag_id]
            if status != current_status:
                status_str = "ALLOCATED" if current_status else "UNALLOCATED"
                boundaries.append((region_start, flag_id - 1, status_str))
                region_start = flag_id
                current_status = status

        status_str = "ALLOCATED" if current_status else "UNALLOCATED"
        boundaries.append((region_start, sorted_flags[-1], status_str))

        return boundaries


def probe_block(
    block_start: int,
    base_offset: int,
    save_path: Union[str, Path],
    extracted_flags_path: Union[str, Path] = None,
) -> AllocationBitmap:
    """
    Convenience function to probe a block's allocation.

    Args:
        block_start: Block start (e.g., 520000)
        base_offset: Known base offset for this block
        save_path: Path to save file
        extracted_flags_path: Path to extracted_event_flags.json

    Returns:
        AllocationBitmap
    """
    schema = BlockSchema(block_start, base_offset)

    if extracted_flags_path:
        schema.load_flags_from_extracted(extracted_flags_path)

    return schema.probe_allocation(save_path)


def main():
    """CLI for schema probing."""
    import argparse

    parser = argparse.ArgumentParser(description="Probe block allocation using schema")
    parser.add_argument("--block", type=int, required=True, help="Block start (e.g., 520000)")
    parser.add_argument("--base", type=int, required=True, help="Base offset")
    parser.add_argument("--save", type=str, required=True, help="Path to save file")
    parser.add_argument("--flags-json", type=str, default="scripts/extracted_event_flags.json",
                        help="Path to extracted_event_flags.json")
    parser.add_argument("--boundaries", action="store_true", help="Show allocation boundaries")
    parser.add_argument("--json", action="store_true", help="Output as JSON")

    args = parser.parse_args()

    schema = BlockSchema(args.block, args.base)
    count = schema.load_flags_from_extracted(args.flags_json)

    if not args.json:
        print(f"Loaded {count} flags into schema for block {args.block}")

    if count == 0:
        print("No flags found for this block!")
        return

    bitmap = schema.probe_allocation(args.save)

    if args.json:
        output = {
            "block_start": bitmap.block_start,
            "base_offset": bitmap.base_offset,
            "total_flags": bitmap.total_flags,
            "allocated_count": len(bitmap.allocated),
            "unallocated_count": len(bitmap.unallocated),
            "trackable_flags": bitmap.get_trackable_flags(),
            "untrackable_flags": bitmap.get_untrackable_flags(),
        }
        if args.boundaries:
            output["boundaries"] = [
                {"start": s, "end": e, "status": st}
                for s, e, st in schema.get_allocation_boundaries(args.save)
            ]
        print(json.dumps(output, indent=2))
    else:
        print()
        print(bitmap.summary())

        if args.boundaries:
            print("ALLOCATION BOUNDARIES:")
            print("-" * 60)
            for start, end, status in schema.get_allocation_boundaries(args.save):
                print(f"  {start}-{end}: {status}")


if __name__ == "__main__":
    main()
