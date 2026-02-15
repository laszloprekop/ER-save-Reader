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
import math
from .archive.flag_formulas import FlagFormulas
from .ground_truth_loader import get_player_coords_config


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

# Character name extraction
# Name offset varies due to variable-size sections before it
# These are empirically verified offsets that work for different slot configurations
# Note: Offsets must be 2-byte aligned for UTF-16
CHARACTER_NAME_OFFSET_CANDIDATES = [0xA27C, 0xAE96, 0xA463, 0xA462]
CHARACTER_NAME_MAX_LEN = 16         # Max 16 UTF-16 characters

# EventFlags offset VARIES per slot due to variable-size GaItems section
# Analysis of 66 save snapshots showed:
# - EF offsets range from 0x13xxx to 0x1Bxxx (not fixed!)
# - Multiple false positives exist at lower offsets with perfect validation scores
# - The REAL EF section is consistently at higher offsets
EVENT_FLAGS_SEARCH_MIN = 0x10000  # Minimum search offset
EVENT_FLAGS_SEARCH_MAX = 0x30000  # Maximum search offset (increased from 0x20000)

# Validation flags for verifying event flags offset is correct
# Format: (flag_id, byte_offset, bit_position, name, tier)
# Tier 1 = critical flags that MUST be set for any playable character
# Tier 2 = early game flags likely set for most characters
VALIDATION_FLAGS = [
    # Tier 1: Tutorial and first graces (MUST be set)
    (71800, 2725, 7, "Cave of Knowledge", 1),
    (71801, 2725, 6, "Stranded Graveyard", 1),
    (76100, 3262, 3, "The First Step", 1),
    (76101, 3262, 2, "Church of Elleh", 1),
    # Tier 2: Early game graces (likely set for progressed characters)
    (76102, 3262, 1, "Gatefront Ruins", 2),
    (76104, 3263, 7, "Agheel Lake South", 2),
    (76106, 3263, 5, "Church of Dragon Communion", 2),
]

MIN_TIER1_VALIDATION_SCORE = 3  # Minimum Tier 1 flags required for valid detection


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

    # Player coordinates (extracted via signature-based search)
    player_coords: Optional[Tuple[float, float, float]] = None
    player_coords2: Optional[Tuple[float, float, float]] = None
    player_facing: Optional[float] = None  # Y-axis rotation in radians [-pi, pi]
    player_map_id: Optional[Tuple[int, int, int, int]] = None


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

        # Extract character name
        character_name = self._extract_character_name(slot_data)

        # Extract player coordinates
        pc = self._extract_player_coords(slot_data)
        player_coords = pc.get('coords') if pc else None
        player_coords2 = pc.get('coords2') if pc else None
        player_facing = pc.get('facing_angle') if pc else None
        player_map_id = pc.get('map_id') if pc else None

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
            validated_graces=validated_graces,
            character_name=character_name,
            player_coords=player_coords,
            player_coords2=player_coords2,
            player_facing=player_facing,
            player_map_id=player_map_id,
        )

    def _find_event_flags_offset(self, slot_data: bytes) -> int:
        """
        Find the event flags section offset using validation flag patterns.

        The offset varies due to variable-size GaItems section (depends on inventory).
        Empirically, the offset is around 0x12B00-0x13800 for our test saves.

        Algorithm:
        1. Search all candidate offsets and score each one
        2. Reject candidates where validation bytes are 0xFF (padding false positives)
        3. Prioritize Tier 1 flags (tutorial graces that MUST be set)
        4. Among candidates with equal Tier 1 scores, prefer LOWER offsets
           (first valid match is more likely correct than 0xFF padding at higher offsets)
        """
        best_offset = 0x12B00  # Default fallback based on empirical testing
        best_tier1_score = 0
        best_total_score = 0

        # Search in 4-byte increments for speed
        for test_offset in range(EVENT_FLAGS_SEARCH_MIN, min(EVENT_FLAGS_SEARCH_MAX, len(slot_data) - EVENT_FLAGS_SIZE), 4):
            tier1_score = 0
            total_score = 0
            has_0xff = False

            for flag_id, byte_off, bit_pos, name, tier in VALIDATION_FLAGS:
                abs_pos = test_offset + byte_off
                if abs_pos < len(slot_data):
                    byte_val = slot_data[abs_pos]
                    if byte_val == 0xFF:
                        has_0xff = True
                    if (byte_val & (1 << bit_pos)) != 0:
                        total_score += 1
                        if tier == 1:
                            tier1_score += 1

            # Skip candidates where ANY validation byte is 0xFF — these are
            # padding regions that produce false positives (all bits read as SET)
            if has_0xff:
                continue

            # Prefer higher tier1, then higher total, but do NOT prefer higher
            # offsets on tie (that causes false positives from 0xFF padding)
            is_better = (
                tier1_score > best_tier1_score or
                (tier1_score == best_tier1_score and total_score > best_total_score)
            )

            if is_better:
                best_tier1_score = tier1_score
                best_total_score = total_score
                best_offset = test_offset

        return best_offset

    def _validate_event_flags(self, event_flags: bytes) -> Tuple[int, List[str]]:
        """Validate event flags section using anchor flags."""
        score = 0
        matched = []

        for flag_id, byte_off, bit_pos, name, tier in VALIDATION_FLAGS:
            if byte_off < len(event_flags):
                if (event_flags[byte_off] & (1 << bit_pos)) != 0:
                    score += 1
                    matched.append(name)

        return score, matched

    def _extract_player_coords(self, slot_data: bytes) -> Dict:
        """
        Extract player coordinates from slot data using signature-based search.

        Uses the same algorithm as verify_player_coords.py and the WASM crate,
        with constants from ground_truth_offsets.json.

        Returns dict with coords, coords2, facing_angle, map_id or empty dict.
        """
        config = get_player_coords_config()
        search_start = config.get("search_start", 0x1D0000)
        search_end = config.get("search_end", 0x280000)
        struct_size = config.get("struct_size", 61)
        mid_size = config.get("mid_section_size", 17)
        mid_min_zeros = config.get("mid_section_min_zeros", 10)
        facing_offset = config.get("facing_angle_offset", 4)
        pad2_size = config.get("padding2_size", 16)
        pad2_min_zeros = config.get("padding2_min_zeros", 8)
        coord_max = config.get("coordinate_range_max", 10000.0)
        mag_threshold = config.get("magnitude_threshold", 10.0)

        if len(slot_data) < 12:
            return {}

        header_map_id = slot_data[4:8]
        actual_end = min(len(slot_data), search_end)

        candidates = []

        for i in range(search_start, actual_end - struct_size):
            if slot_data[i:i + 4] != header_map_id:
                continue

            # Check padding2
            pad2_start = i + 4 + mid_size + 12
            if pad2_start + pad2_size > len(slot_data):
                continue
            pad2_zeros = sum(1 for b in slot_data[pad2_start:pad2_start + pad2_size] if b == 0)
            if pad2_zeros < pad2_min_zeros:
                continue

            # Check mid_section
            mid_zeros = sum(1 for b in slot_data[i + 4:i + 4 + mid_size] if b == 0)
            if mid_zeros < mid_min_zeros:
                continue

            # Read coords before map_id
            if i < 12:
                continue
            coords_offset = i - 12
            x, y, z = struct.unpack_from('<fff', slot_data, coords_offset)

            if any(math.isnan(c) or math.isinf(c) or abs(c) > coord_max for c in (x, y, z)):
                continue

            # Read coords2
            x2, y2, z2 = struct.unpack_from('<fff', slot_data, i + 4 + mid_size)
            if any(math.isnan(c) or math.isinf(c) or abs(c) > coord_max for c in (x2, y2, z2)):
                continue

            # Facing angle
            facing = struct.unpack_from('<f', slot_data, i + 4 + facing_offset)[0]
            if not math.isfinite(facing):
                facing = 0.0

            magnitude = abs(x) + abs(y) + abs(z)
            has_position = magnitude > mag_threshold

            map_id = struct.unpack_from('<4B', slot_data, i)

            candidates.append({
                'offset': coords_offset,
                'coords': (x, y, z),
                'coords2': (x2, y2, z2),
                'facing_angle': facing,
                'map_id': map_id,
                'pad1_zeros': mid_zeros,
                'pad2_zeros': pad2_zeros,
                'has_position': has_position,
            })

        if not candidates:
            return {}

        # Select best candidate
        candidates.sort(key=lambda c: (c['has_position'], c['pad2_zeros'], c['pad1_zeros']), reverse=True)
        best = candidates[0]
        if not best['has_position']:
            return {}
        return best

    def _extract_character_name(self, slot_data: bytes) -> Optional[str]:
        """
        Extract character name from slot data.

        The name is stored as UTF-16LE at a variable offset (due to preceding
        variable-size sections). We try known offset candidates and return
        the first valid name found.
        """
        for offset in CHARACTER_NAME_OFFSET_CANDIDATES:
            if offset + CHARACTER_NAME_MAX_LEN * 2 > len(slot_data):
                continue

            # Read up to 32 bytes (16 UTF-16 chars)
            name_bytes = slot_data[offset:offset + CHARACTER_NAME_MAX_LEN * 2]

            try:
                # Decode as UTF-16LE and extract the name (null-terminated)
                name = name_bytes.decode('utf-16-le').split('\x00')[0]

                # Validate: must be printable, 1-16 chars
                if not name or not name.isprintable() or not 1 <= len(name) <= CHARACTER_NAME_MAX_LEN:
                    continue

                # First character should be ASCII letter or common Latin char
                # This filters out misaligned reads that produce CJK characters
                first_char = name[0]
                if not (first_char.isascii() and first_char.isalpha()):
                    # Allow extended Latin (accented chars) but not CJK
                    if ord(first_char) > 0x024F:  # Beyond Latin Extended-B
                        continue

                # Real names should be mostly letters/numbers
                alnum_count = sum(1 for c in name if c.isalnum())
                if alnum_count >= len(name) * 0.5:
                    return name
            except (UnicodeDecodeError, ValueError):
                continue

        return None

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

    def extract_character_context(self, slot_data: SlotData) -> Dict:
        """
        Extract full character context for verification.

        Returns comprehensive context including:
        - All discovered graces (not just validation flags)
        - Inventory counts (from GaItems)
        - Progression markers
        - Character metadata

        This provides proper context for verification instead of relying
        solely on VALIDATION_FLAGS which only checks 4 early-game graces.

        IMPORTANT: validated_graces in SlotData only contains 4 tutorial/early
        graces used for EF offset validation. This method checks actual
        character progression across all regions.
        """
        return {
            "slot_index": slot_data.slot_index,
            "character_name": slot_data.character_name,
            "gaitem_count": slot_data.gaitem_count,
            "discovered_graces": self._get_all_discovered_graces(slot_data),
            "progression_markers": self._check_progression_flags(slot_data),
            "validation_graces": slot_data.validated_graces,  # Original 4 validation flags
            "validation_score": slot_data.validation_score,
        }

    def _get_all_discovered_graces(self, slot_data: SlotData) -> Dict[str, List[Dict]]:
        """
        Check ALL grace flags, not just VALIDATION_FLAGS.

        Returns graces categorized by region/dungeon with their flag status.

        Grace flag ranges:
        - Block format (71xxx-76xxx): Tutorial, Limgrave, etc.
        - Dungeon format (16000xxx): Volcano Manor (area 16), etc.
        - Dungeon format (19000xxx): Mohgwyn Palace (area 19), etc.

        NOTE: VALIDATION_FLAGS are for offset validation only.
        This method provides actual progression tracking.
        """
        event_flags = slot_data.event_flags
        discovered = {
            "tutorial_graces": [],
            "limgrave_graces": [],
            "weeping_peninsula_graces": [],
            "liurnia_graces": [],
            "altus_graces": [],
            "caelid_graces": [],
            "mountaintops_graces": [],
            "volcano_manor_graces": [],
            "other_dungeon_graces": [],
            "underground_graces": [],
            "other_graces": [],
        }

        # Known grace flags by region (block format)
        # These are verified from BonfireWarpParam.param.xml
        grace_checks = [
            # Tutorial (71800-71802)
            {"flag_id": 71800, "name": "Cave of Knowledge", "region": "tutorial_graces"},
            {"flag_id": 71801, "name": "Stranded Graveyard", "region": "tutorial_graces"},

            # Limgrave (76100-76149)
            {"flag_id": 76100, "name": "The First Step", "region": "limgrave_graces"},
            {"flag_id": 76101, "name": "Church of Elleh", "region": "limgrave_graces"},
            {"flag_id": 76102, "name": "Gatefront Ruins", "region": "limgrave_graces"},
            {"flag_id": 76103, "name": "Stormfoot Catacombs", "region": "limgrave_graces"},
            {"flag_id": 76104, "name": "Agheel Lake South", "region": "limgrave_graces"},
            {"flag_id": 76105, "name": "Agheel Lake North", "region": "limgrave_graces"},
            {"flag_id": 76106, "name": "Church of Dragon Communion", "region": "limgrave_graces"},
            {"flag_id": 76107, "name": "Fort Haight West", "region": "limgrave_graces"},
            {"flag_id": 76108, "name": "Third Church of Marika", "region": "limgrave_graces"},
            {"flag_id": 76109, "name": "Artist's Shack", "region": "limgrave_graces"},
            {"flag_id": 76110, "name": "Summonwater Village Outskirts", "region": "limgrave_graces"},
            {"flag_id": 76111, "name": "Waypoint Ruins Cellar", "region": "limgrave_graces"},
            {"flag_id": 76112, "name": "Seaside Ruins", "region": "limgrave_graces"},
            {"flag_id": 76113, "name": "Mistwood Outskirts", "region": "limgrave_graces"},
            {"flag_id": 76114, "name": "Murkwater Coast", "region": "limgrave_graces"},

            # Weeping Peninsula (76150-76170)
            {"flag_id": 76150, "name": "Castle Morne Rampart", "region": "weeping_peninsula_graces"},
            {"flag_id": 76151, "name": "Tombsward", "region": "weeping_peninsula_graces"},
            {"flag_id": 76152, "name": "Church of Pilgrimage", "region": "weeping_peninsula_graces"},
            {"flag_id": 76153, "name": "Fourth Church of Marika", "region": "weeping_peninsula_graces"},
            {"flag_id": 76154, "name": "Ailing Village Outskirts", "region": "weeping_peninsula_graces"},
            {"flag_id": 76155, "name": "Beside the Crater-Pocked Glade", "region": "weeping_peninsula_graces"},
            {"flag_id": 76156, "name": "Isolated Merchant's Shack", "region": "weeping_peninsula_graces"},
            {"flag_id": 76157, "name": "Bridge of Sacrifice", "region": "weeping_peninsula_graces"},

            # Liurnia (76200-76299)
            {"flag_id": 76200, "name": "Lake-Facing Cliffs", "region": "liurnia_graces"},
            {"flag_id": 76201, "name": "Liurnia Lake Shore", "region": "liurnia_graces"},
            {"flag_id": 76202, "name": "Laskyar Ruins", "region": "liurnia_graces"},
            {"flag_id": 76203, "name": "Scenic Isle", "region": "liurnia_graces"},
            {"flag_id": 76204, "name": "Academy Gate Town", "region": "liurnia_graces"},
            {"flag_id": 76205, "name": "South Raya Lucaria Gate", "region": "liurnia_graces"},
            {"flag_id": 76206, "name": "Main Academy Gate", "region": "liurnia_graces"},
            {"flag_id": 76207, "name": "Crystalline Woods", "region": "liurnia_graces"},
            {"flag_id": 76208, "name": "East Gate Bridge Trestle", "region": "liurnia_graces"},
            {"flag_id": 76209, "name": "Raya Lucaria Academy Gate", "region": "liurnia_graces"},

            # Altus Plateau (76300-76399)
            {"flag_id": 76300, "name": "Altus Plateau", "region": "altus_graces"},
            {"flag_id": 76301, "name": "Erdtree-Gazing Hill", "region": "altus_graces"},
            {"flag_id": 76302, "name": "Altus Highway Junction", "region": "altus_graces"},
            {"flag_id": 76303, "name": "Forest-Spanning Greatbridge", "region": "altus_graces"},
            {"flag_id": 76304, "name": "Rampartside Path", "region": "altus_graces"},
            {"flag_id": 76305, "name": "Bower of Bounty", "region": "altus_graces"},
            {"flag_id": 76306, "name": "Road of Iniquity Side Path", "region": "altus_graces"},

            # Mt. Gelmir (76350-76380)
            {"flag_id": 76350, "name": "Bridge of Iniquity", "region": "altus_graces"},
            {"flag_id": 76351, "name": "First Mt. Gelmir Campsite", "region": "altus_graces"},
            {"flag_id": 76352, "name": "Ninth Mt. Gelmir Campsite", "region": "altus_graces"},
            {"flag_id": 76353, "name": "Road of Iniquity", "region": "altus_graces"},

            # Caelid (76400-76499)
            {"flag_id": 76400, "name": "Smoldering Church", "region": "caelid_graces"},
            {"flag_id": 76401, "name": "Rotview Balcony", "region": "caelid_graces"},
            {"flag_id": 76402, "name": "Fort Gael North", "region": "caelid_graces"},
            {"flag_id": 76403, "name": "Caelem Ruins", "region": "caelid_graces"},
            {"flag_id": 76404, "name": "Cathedral of Dragon Communion", "region": "caelid_graces"},
            {"flag_id": 76405, "name": "Caelid Highway South", "region": "caelid_graces"},
            {"flag_id": 76406, "name": "Smoldering Wall", "region": "caelid_graces"},
            {"flag_id": 76407, "name": "Deep Siofra Well", "region": "caelid_graces"},

            # Mountaintops (76500-76599)
            {"flag_id": 76500, "name": "Zamor Ruins", "region": "mountaintops_graces"},
            {"flag_id": 76501, "name": "Ancient Snow Valley Ruins", "region": "mountaintops_graces"},
            {"flag_id": 76502, "name": "Freezing Lake", "region": "mountaintops_graces"},
            {"flag_id": 76503, "name": "First Church of Marika", "region": "mountaintops_graces"},

            # Volcano Manor dungeon graces (area 16 - dungeon format)
            {"flag_id": 16000002, "name": "Temple of Eiglay (VM)", "region": "volcano_manor_graces"},
            {"flag_id": 16000003, "name": "Prison Town Church (VM)", "region": "volcano_manor_graces"},
            {"flag_id": 16000004, "name": "Guest Hall (VM)", "region": "volcano_manor_graces"},
            {"flag_id": 16000005, "name": "Audience Pathway (VM)", "region": "volcano_manor_graces"},
            {"flag_id": 16000006, "name": "Abductor Virgin (VM)", "region": "volcano_manor_graces"},
            {"flag_id": 16000007, "name": "Subterranean Inquisition Chamber (VM)", "region": "volcano_manor_graces"},

            # Stormveil Castle (area 10 - dungeon format)
            {"flag_id": 10000002, "name": "Stormveil Cliffside (SV)", "region": "other_dungeon_graces"},
            {"flag_id": 10000003, "name": "Rampart Tower (SV)", "region": "other_dungeon_graces"},
            {"flag_id": 10000004, "name": "Liftside Chamber (SV)", "region": "other_dungeon_graces"},
            {"flag_id": 10000005, "name": "Secluded Cell (SV)", "region": "other_dungeon_graces"},

            # Siofra River (area 12 - underground)
            {"flag_id": 12000002, "name": "Siofra River Bank", "region": "underground_graces"},
            {"flag_id": 12000003, "name": "Worshippers' Woods", "region": "underground_graces"},
            {"flag_id": 12000004, "name": "Below the Well", "region": "underground_graces"},

            # Mohgwyn Palace (area 19 - dungeon format)
            {"flag_id": 19000002, "name": "Palace Approach Ledge-Road", "region": "other_dungeon_graces"},
            {"flag_id": 19000003, "name": "Dynasty Mausoleum Entrance", "region": "other_dungeon_graces"},
            {"flag_id": 19000004, "name": "Dynasty Mausoleum Midpoint", "region": "other_dungeon_graces"},
            {"flag_id": 19000005, "name": "Cocoon of the Empyrean", "region": "other_dungeon_graces"},
        ]

        for grace in grace_checks:
            is_set, error = self.check_flag(event_flags, grace["flag_id"])
            if is_set:
                discovered[grace["region"]].append({
                    "flag_id": grace["flag_id"],
                    "name": grace["name"],
                })

        return discovered

    def _check_progression_flags(self, slot_data: SlotData) -> Dict[str, bool]:
        """
        Check key progression flags to determine character progress.

        Returns dict of progression markers and their status.
        """
        event_flags = slot_data.event_flags
        progression = {}

        # Key progression flags (from EMEVD and game analysis)
        progression_checks = [
            # Great Runes possessed (160-167)
            {"flag_id": 160, "name": "godrick_great_rune"},
            {"flag_id": 161, "name": "radahn_great_rune"},
            {"flag_id": 162, "name": "morgott_great_rune"},
            {"flag_id": 163, "name": "rykard_great_rune"},
            {"flag_id": 164, "name": "mohg_great_rune"},
            {"flag_id": 165, "name": "malenia_great_rune"},

            # Boss defeats (from GameAreaParam defeat flags)
            {"flag_id": 10000800, "name": "godrick_defeated"},
            {"flag_id": 14000800, "name": "rennala_defeated"},
            {"flag_id": 16000800, "name": "rykard_defeated"},
            {"flag_id": 35000800, "name": "radahn_defeated"},
            {"flag_id": 11000800, "name": "morgott_defeated"},

            # Divine Tower activations (180-187)
            {"flag_id": 180, "name": "godrick_rune_activated"},
            {"flag_id": 181, "name": "radahn_rune_activated"},
            {"flag_id": 183, "name": "rykard_rune_activated"},

            # Area access flags (using grace discovery as proxy)
            {"flag_id": 76200, "name": "reached_liurnia"},  # Lake-Facing Cliffs grace
            {"flag_id": 76300, "name": "reached_altus"},    # Altus Plateau grace
            {"flag_id": 76350, "name": "reached_mt_gelmir"},  # Bridge of Iniquity
            {"flag_id": 76400, "name": "reached_caelid"},   # Smoldering Church grace
            {"flag_id": 76500, "name": "reached_mountaintops"},  # Zamor Ruins grace
            {"flag_id": 16000002, "name": "reached_volcano_manor"},  # Temple of Eiglay
        ]

        for check in progression_checks:
            is_set, _ = self.check_flag(event_flags, check["flag_id"])
            progression[check["name"]] = is_set

        return progression

    def get_grace_summary(self, slot_data: SlotData) -> Dict:
        """
        Get a summary of discovered graces for quick context assessment.

        Returns counts by region for determining character progression level.
        """
        discovered = self._get_all_discovered_graces(slot_data)
        summary = {}
        total = 0

        for region, graces in discovered.items():
            count = len(graces)
            summary[region] = count
            total += count

        summary["total"] = total
        return summary

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
