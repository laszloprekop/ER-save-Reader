"""
Event Flag Calculation Formulas

==============================================================================
DEPRECATED: This file contains OUTDATED values!
==============================================================================

Use ground_truth_loader.py instead, which reads from ground_truth_offsets.json
(the single source of truth synced with src/generated/ground_truth.rs).

Known desync issues between this file and ground_truth:
  - Block 62000: 1500 here vs 9359 in ground_truth
  - Block 65000: 1875 here vs 37412 in ground_truth
  - Block 67000: 3546 here vs 37411 in ground_truth
  - Block 71000: 2625 here vs 9315 in ground_truth

This file is kept for reference only. DO NOT use BLOCK_BASES values from this
file for new verification scripts.

==============================================================================

Contains all known formulas for calculating event flag byte offsets and bit positions.
Each formula has known limitations - this module documents them and provides
tools to test which formulas work for which flag categories.

IMPORTANT: These formulas are NOT error-free! The verification framework exists
specifically to identify which formulas work and which don't.

Known Issues (as of 2026-01):
- Block-based formulas: Generally reliable for 5-6 digit flags
- Tile formulas: Work for localId 0-6999, FAIL for localId >= 7000
- Dungeon formulas: Partially verified, needs more testing
- Consumable treasure flags (localId 7300+): IMPOSSIBLE - no storage space
"""

from dataclasses import dataclass
from typing import Optional, Dict, Tuple, Any
from .verification_data import FormulaResult


@dataclass
class BlockConfig:
    """Configuration for a block-based flag range."""
    block_start: int    # Starting flag ID for this block
    base_offset: int    # Byte offset in event flags for block start
    block_size: int     # Number of flags in block (default 1000)
    status: str         # "verified", "unverified", "deprecated"
    notes: str = ""


@dataclass
class TileConfig:
    """Configuration for tile-based (10-digit) flag calculation."""
    base_offset: int        # Starting byte offset for tile 33,42
    bytes_per_slot: int     # Bytes per map tile slot (875)
    slots_per_row: int      # Number of tile columns (40)
    row_base: int           # First tile row (33)
    col_base: int           # First tile column (42)
    max_local_id: int       # Maximum valid localId (6999)
    status: str             # "partial", "verified", "broken"


@dataclass
class DungeonConfig:
    """Configuration for dungeon (8-digit) flag calculation."""
    map_area: int           # Map area code (10 = Stormveil, etc.)
    base_offset: int        # Byte offset for this dungeon
    section_size: int       # Bytes per section
    status: str             # "verified", "unverified", "broken"
    notes: str = ""


class FlagFormulas:
    """
    Collection of all known flag calculation formulas.

    Usage:
        formulas = FlagFormulas()
        result = formulas.calculate_offset(flag_id)
        # result contains byte_offset, bit_position, and metadata
    """

    # =========================================================================
    # BLOCK-BASED FORMULAS (5-6 digit flags)
    # =========================================================================
    # These map specific flag ranges to base byte offsets
    # Formula: byte_offset = base + (flag_id - block_start) // 8
    # bit_position = 7 - (flag_id % 8)

    BLOCK_BASES: Dict[int, BlockConfig] = {
        # IMPORTANT: Block bases are NOT contiguous! Different flag categories
        # are stored in different regions. Only use empirically verified bases.
        #
        # Empirically verified bases (2026-01-11):
        # - 71000 (tutorial graces): 2625 - from validation flags 71800, 71801
        # - 76000 (world graces): 3250 - from validation flags 76100, 76101
        # - 67000 (cookbooks): 3987 - from Missionary's Cookbook [4] diff

        # System and progression flags (VERIFIED 2026-01-11)
        # Cross-validated with 60100 (Crafting Kit), 60130 (Whetstone Knife), 60220 (Furled Finger)
        60000: BlockConfig(60000, 2548, 1000, "verified", "Progression flags - verified from multiple items"),
        # Map fragments (VERIFIED 2026-01-11 from 62174 Ailing Village match at offset 1521)
        62000: BlockConfig(62000, 1500, 1000, "verified", "Map fragments - verified from 62174 match"),
        65000: BlockConfig(65000, 1875, 1000, "unverified", "Whetblades - base unconfirmed"),

        # Cookbooks (RE-VERIFIED 2026-01-11 from precise diff analysis)
        # Previous value of 3987 was WRONG - byte 3990 did not change during pickup
        # Actual change was at byte 3549, which gives base=3546
        67000: BlockConfig(67000, 3546, 1000, "verified", "Cookbooks - re-verified from Missionary's Cookbook [4], byte 3549"),
        68000: BlockConfig(68000, 3671, 1000, "calculated", "Cookbooks continued (67000 + 125)"),

        # Tutorial graces (VERIFIED from validation flags)
        71000: BlockConfig(71000, 2625, 1000, "verified", "Tutorial graces (71800, 71801)"),

        # Dungeon graces (PARTIALLY VERIFIED 2026-01-11)
        72000: BlockConfig(72000, 2750, 1000, "unverified", "Dungeon graces - base unconfirmed"),
        # 73xxx VERIFIED from slot comparison - 13/13 dungeon graces matched at base 2664
        73000: BlockConfig(73000, 2664, 1000, "verified", "Dungeon graces - verified from 13 catacombs/caves/tunnels"),
        74000: BlockConfig(74000, 3000, 1000, "unverified", "Extended dungeon graces - base unconfirmed"),
        75000: BlockConfig(75000, 3125, 1000, "unverified", "Extended graces - base unconfirmed"),

        # World graces (VERIFIED from validation flags)
        76000: BlockConfig(76000, 3250, 1000, "verified", "World graces - The First Step (76100)"),
        77000: BlockConfig(77000, 3375, 1000, "calculated", "Extended world graces (76000 + 125)"),
        78000: BlockConfig(78000, 3500, 1000, "unverified", "Landmark flags - base unconfirmed"),
    }

    # Validation flags - these are ALWAYS correct (anchors for detection)
    VALIDATION_FLAGS: Dict[int, Tuple[int, int, str]] = {
        # flag_id: (byte_offset, bit_position, name)
        71800: (2725, 7, "Cave of Knowledge"),
        71801: (2725, 6, "Stranded Graveyard"),
        76100: (3262, 3, "The First Step"),
        76101: (3262, 2, "Church of Elleh"),
    }

    # =========================================================================
    # TILE-BASED FORMULA (10-digit base game flags)
    # =========================================================================
    # Format: 10XXYYZZZZ where 10=prefix (base game), XX=row, YY=col, ZZZZ=localId
    # Example: 1043500010 = prefix 10, row 43, col 50, local 10
    # Parse: row = flag_str[2:4], col = flag_str[4:6], local = flag_str[6:]
    # Formula: offset = base + ((row - row_base) * slots_per_row + (col - col_base)) * bytes_per_slot + localId // 8
    # Bit: 7 - (local_id % 8), same convention as block formula
    #
    # LIMITATIONS:
    # 1. Only works for localId 0-6999 (slots are 875 bytes = 7000 flags)
    # 2. col_base=30 verified; tiles with col < 30 may use different storage region
    #    (empirical testing needed for western map tiles)
    #
    # VERIFIED 2026-01-20: Flag 1043500010 (Smoldering Butterfly) at byte 857482, bit 5
    # Previous base_offset=485330 was wrong by 4651 bytes. Corrected via temporal diff.

    TILE_CONFIG = TileConfig(
        base_offset=485330,     # REVERTED 2026-01-25: 489981 was WRONG. Re-verified via Smoldering Butterfly at offset 852831
        bytes_per_slot=875,     # 875 bytes = 7000 flags per slot
        slots_per_row=40,       # 40 columns per row
        row_base=33,            # First tile row
        col_base=30,            # First tile column
        max_local_id=6999,      # LocalId >= 7000 has no storage!
        status="verified"       # Re-verified 2026-01-25: 1043500010 at offset 852831 bit 5
    )

    # =========================================================================
    # DUNGEON FORMULAS (8-digit flags)
    # =========================================================================
    # Format: AASSZZZZ where AA=area, SS=section, ZZZZ=localId
    # Each dungeon has its own base offset

    DUNGEON_CONFIGS: Dict[int, DungeonConfig] = {
        # Legacy Dungeons (need more investigation)
        10: DungeonConfig(10, 1383375, 1125, "unverified", "Stormveil Castle"),
        11: DungeonConfig(11, 0, 1125, "unverified", "Leyndell, Royal Capital"),
        12: DungeonConfig(12, 0, 1125, "unverified", "Underground (Siofra, etc.)"),
        13: DungeonConfig(13, 0, 1125, "unverified", "Crumbling Farum Azula"),
        14: DungeonConfig(14, 0, 1125, "unverified", "Academy of Raya Lucaria"),
        15: DungeonConfig(15, 0, 1125, "unverified", "Caria Manor"),
        16: DungeonConfig(16, 0, 1125, "unverified", "Volcano Manor"),
        18: DungeonConfig(18, 0, 1125, "unverified", "Roundtable Hold"),
        19: DungeonConfig(19, 0, 1125, "unverified", "Chapel of Anticipation"),
        20: DungeonConfig(20, 0, 1125, "unverified", "Stranded Graveyard"),
        21: DungeonConfig(21, 0, 1125, "unverified", "Miquella's Haligtree"),

        # Minor Dungeons (VERIFIED 2026-01-12 via slot comparison)
        # Formula: byte = base + section * 1125 + local_id // 8
        30: DungeonConfig(30, 27411, 1125, "verified", "Catacombs - 5 bosses matched"),
        31: DungeonConfig(31, 28634, 1125, "verified", "Caves - 5 bosses matched"),
        32: DungeonConfig(32, 31577, 1125, "verified", "Tunnels - 4 bosses matched"),

        34: DungeonConfig(34, 0, 1125, "unverified", "Divine Towers"),
        35: DungeonConfig(35, 0, 1125, "unverified", "Mohgwyn Palace"),
        39: DungeonConfig(39, 0, 1125, "unverified", "Elden Throne"),
    }

    # =========================================================================
    # DLC FORMULA (10-digit starting with 2)
    # =========================================================================
    # Format: 2XXYYZZZZ - Shadow of the Erdtree
    # Uses similar tile formula but with different base offset

    DLC_CONFIG = {
        "base_offset": 0,  # TODO: Determine empirically
        "status": "unknown",
    }

    def __init__(self):
        """Initialize formula calculator."""
        pass

    def calculate_offset(self, flag_id: int) -> Dict[str, FormulaResult]:
        """
        Apply all applicable formulas to a flag ID.

        Returns a dict of formula name -> FormulaResult
        """
        results = {}

        # Try block-based formula
        block_result = self._calc_block_offset(flag_id)
        if block_result:
            results["block"] = block_result

        # Try tile-based formula
        tile_result = self._calc_tile_offset(flag_id)
        if tile_result:
            results["tile"] = tile_result

        # Try dungeon formula
        dungeon_result = self._calc_dungeon_offset(flag_id)
        if dungeon_result:
            results["dungeon"] = dungeon_result

        # Try DLC formula
        dlc_result = self._calc_dlc_offset(flag_id)
        if dlc_result:
            results["dlc"] = dlc_result

        return results

    def _calc_block_offset(self, flag_id: int) -> Optional[FormulaResult]:
        """Calculate offset using block-based formula."""
        # Check if flag is in a known block range
        block_start = (flag_id // 1000) * 1000

        if block_start in self.BLOCK_BASES:
            config = self.BLOCK_BASES[block_start]
            relative = flag_id - block_start
            byte_offset = config.base_offset + relative // 8
            bit_position = 7 - (flag_id % 8)

            return FormulaResult(
                formula_name="block",
                byte_offset=byte_offset,
                bit_position=bit_position,
                is_valid=True,
                error_message=None
            )

        # Check validation flags specifically
        if flag_id in self.VALIDATION_FLAGS:
            offset, bit, name = self.VALIDATION_FLAGS[flag_id]
            return FormulaResult(
                formula_name="block",
                byte_offset=offset,
                bit_position=bit,
                is_valid=True,
                error_message=None
            )

        return None

    def _calc_tile_offset(self, flag_id: int) -> Optional[FormulaResult]:
        """Calculate offset using tile-based formula for 10-digit flags.

        Format: 10XXYYZZZZ where:
        - 10 = base game prefix
        - XX = tile row (e.g., 43)
        - YY = tile column (e.g., 50)
        - ZZZZ = local ID within tile (e.g., 0010)

        Example: 1043500010 = row 43, col 50, local 10
        """
        # Check if this is a 10-digit base game flag (10XXXXXXXX)
        if not (1_000_000_000 <= flag_id < 2_000_000_000):
            return None

        # Extract components - note prefix is "10" (2 digits), not "1" (1 digit)
        flag_str = str(flag_id)
        if len(flag_str) != 10:
            return FormulaResult(
                formula_name="tile",
                byte_offset=None,
                bit_position=None,
                is_valid=False,
                error_message=f"Invalid flag format: {flag_id}"
            )

        try:
            # Correct parsing: 10XXYYZZZZ
            # prefix = flag_str[0:2]  # "10" for base game
            row = int(flag_str[2:4])     # XX (tile row)
            col = int(flag_str[4:6])     # YY (tile column)
            local_id = int(flag_str[6:]) # ZZZZ (local ID)
        except (ValueError, IndexError) as e:
            return FormulaResult(
                formula_name="tile",
                byte_offset=None,
                bit_position=None,
                is_valid=False,
                error_message=f"Parse error: {e}"
            )

        # Check if localId is trackable
        if local_id >= self.TILE_CONFIG.max_local_id:
            return FormulaResult(
                formula_name="tile",
                byte_offset=None,
                bit_position=None,
                is_valid=False,
                error_message=f"LocalId {local_id} >= {self.TILE_CONFIG.max_local_id}: UNTRACKABLE (no storage space)"
            )

        # Check row/col produce valid offset
        # Note: col can be < col_base (giving negative col_index), which is valid
        # as long as the final offset is non-negative
        config = self.TILE_CONFIG

        # Basic sanity check - row and col should be reasonable values
        if not (30 <= row <= 60):  # Reasonable range for Elden Ring map tiles
            return FormulaResult(
                formula_name="tile",
                byte_offset=None,
                bit_position=None,
                is_valid=False,
                error_message=f"Row {row} out of reasonable range (30-60)"
            )
        if not (30 <= col <= 60):  # Reasonable range for Elden Ring map tiles
            return FormulaResult(
                formula_name="tile",
                byte_offset=None,
                bit_position=None,
                is_valid=False,
                error_message=f"Col {col} out of reasonable range (30-60)"
            )

        # Calculate tile slot offset
        tile_offset = (
            (row - config.row_base) * config.slots_per_row +
            (col - config.col_base)
        ) * config.bytes_per_slot

        # Final offset calculation
        byte_offset = config.base_offset + tile_offset + (local_id // 8)

        # Validate final offset is non-negative
        if byte_offset < 0:
            return FormulaResult(
                formula_name="tile",
                byte_offset=None,
                bit_position=None,
                is_valid=False,
                error_message=f"Calculated offset {byte_offset} is negative (row={row}, col={col})"
            )

        # Bit position formula - SAME convention as block formula
        # Returns physical bit position (0-7 from right) for direct checking
        # Verified empirically: flag 1043500010 (localId=10) at physical bit 5
        # (byte 0x20 = 0b00100000 has bit 5 set)
        bit_position = 7 - (local_id % 8)

        return FormulaResult(
            formula_name="tile",
            byte_offset=byte_offset,
            bit_position=bit_position,
            is_valid=True,
            error_message=None
        )

    def _calc_dungeon_offset(self, flag_id: int) -> Optional[FormulaResult]:
        """Calculate offset using dungeon formula for 8-digit flags."""
        # Check if this is an 8-digit dungeon flag (AASSZZZZ)
        if not (10_000_000 <= flag_id < 100_000_000):
            return None

        flag_str = f"{flag_id:08d}"

        try:
            map_area = int(flag_str[0:2])     # AA
            section = int(flag_str[2:4])       # SS
            local_id = int(flag_str[4:8])      # ZZZZ
        except (ValueError, IndexError) as e:
            return FormulaResult(
                formula_name="dungeon",
                byte_offset=None,
                bit_position=None,
                is_valid=False,
                error_message=f"Parse error: {e}"
            )

        if map_area not in self.DUNGEON_CONFIGS:
            return FormulaResult(
                formula_name="dungeon",
                byte_offset=None,
                bit_position=None,
                is_valid=False,
                error_message=f"Unknown dungeon area: {map_area}"
            )

        config = self.DUNGEON_CONFIGS[map_area]

        if config.base_offset == 0:
            return FormulaResult(
                formula_name="dungeon",
                byte_offset=None,
                bit_position=None,
                is_valid=False,
                error_message=f"Dungeon {map_area} base offset unknown"
            )

        # Calculate offset
        # Each section has section_size bytes
        section_offset = section * config.section_size
        byte_offset = config.base_offset + section_offset + (local_id // 8)
        bit_position = 7 - (flag_id % 8)

        return FormulaResult(
            formula_name="dungeon",
            byte_offset=byte_offset,
            bit_position=bit_position,
            is_valid=True,
            error_message=None
        )

    def _calc_dlc_offset(self, flag_id: int) -> Optional[FormulaResult]:
        """Calculate offset for DLC flags (starting with 2)."""
        if not (2_000_000_000 <= flag_id < 3_000_000_000):
            return None

        # DLC formula not yet determined
        return FormulaResult(
            formula_name="dlc",
            byte_offset=None,
            bit_position=None,
            is_valid=False,
            error_message="DLC formula not yet determined empirically"
        )

    def get_validation_flags(self) -> Dict[int, Tuple[int, int, str]]:
        """Get the anchor validation flags."""
        return self.VALIDATION_FLAGS.copy()

    def export_config(self) -> Dict[str, Any]:
        """Export all formula configurations as JSON-serializable dict."""
        return {
            "block_bases": {
                str(k): {
                    "block_start": v.block_start,
                    "base_offset": v.base_offset,
                    "block_size": v.block_size,
                    "status": v.status,
                    "notes": v.notes,
                }
                for k, v in self.BLOCK_BASES.items()
            },
            "tile_formula": {
                "base_offset": self.TILE_CONFIG.base_offset,
                "bytes_per_slot": self.TILE_CONFIG.bytes_per_slot,
                "slots_per_row": self.TILE_CONFIG.slots_per_row,
                "row_base": self.TILE_CONFIG.row_base,
                "col_base": self.TILE_CONFIG.col_base,
                "max_local_id": self.TILE_CONFIG.max_local_id,
                "status": self.TILE_CONFIG.status,
            },
            "dungeon_configs": {
                str(k): {
                    "map_area": v.map_area,
                    "base_offset": v.base_offset,
                    "section_size": v.section_size,
                    "status": v.status,
                    "notes": v.notes,
                }
                for k, v in self.DUNGEON_CONFIGS.items()
            },
            "validation_flags": {
                str(k): {"offset": v[0], "bit": v[1], "name": v[2]}
                for k, v in self.VALIDATION_FLAGS.items()
            }
        }


# Convenience function for testing
def test_formula(flag_id: int):
    """Test all formulas against a single flag ID."""
    formulas = FlagFormulas()
    results = formulas.calculate_offset(flag_id)

    print(f"\nFlag ID: {flag_id}")
    print("-" * 50)
    for name, result in results.items():
        if result.is_valid:
            print(f"  {name}: offset={result.byte_offset}, bit={result.bit_position}")
        else:
            print(f"  {name}: INVALID - {result.error_message}")

    if not results:
        print("  No applicable formulas found")


if __name__ == "__main__":
    # Test various flag types
    test_flags = [
        71800,        # Tutorial grace (block-based)
        76100,        # The First Step (block-based)
        67030,        # Cookbook (block-based)
        1042507020,   # World pickup (tile-based, valid localId)
        1044367300,   # Golden Rune (tile-based, INVALID localId >= 7000)
        10007990,     # Stormveil dungeon (dungeon-based)
        2012345678,   # DLC flag
    ]

    for flag_id in test_flags:
        test_formula(flag_id)
