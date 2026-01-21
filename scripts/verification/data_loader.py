"""
Data Loader for Verification

Loads extracted event flags and manual completions for verification testing.
"""
from __future__ import annotations

import json
import re
from pathlib import Path
from dataclasses import dataclass, field
from typing import List, Dict, Optional, Any
from .verification_data import FlagVerification, FlagCategory, VerificationStatus


@dataclass
class ExtractedFlag:
    """A flag entry from extracted_event_flags.json."""
    flag_id: int
    name: str
    category: str
    region: str
    source_file: Optional[str] = None
    source_row_id: Optional[int] = None
    item_id: Optional[int] = None
    map_tile: Optional[str] = None
    pos_x: Optional[float] = None
    pos_y: Optional[float] = None
    pos_z: Optional[float] = None
    is_dlc: bool = False
    treasure_type: Optional[str] = None


@dataclass
class ManualCompletion:
    """A manually verified completion from user log."""
    name: str
    flag_id: Optional[int] = None
    category: str = "Unknown"
    slot: Optional[int] = None
    completed: bool = True
    notes: Optional[str] = None


class DataLoader:
    """
    Loads and parses verification data from various sources.
    """

    # Map category strings from extracted_event_flags.json to FlagCategory enum
    CATEGORY_MAP = {
        "Grace": FlagCategory.GRACE,
        "Boss Defeat": FlagCategory.BOSS_DEFEAT,
        "Great Boss Defeat": FlagCategory.GREAT_BOSS_DEFEAT,
        "Field Boss Defeat": FlagCategory.FIELD_BOSS_DEFEAT,
        "World Pickup": FlagCategory.WORLD_PICKUP,
        "Dungeon Pickup": FlagCategory.DUNGEON_PICKUP,
        "DLC Pickup": FlagCategory.DLC_PICKUP,
        "Cookbook": FlagCategory.COOKBOOK,
        "Whetblade": FlagCategory.WHETBLADE,
        "Map Fragment": FlagCategory.MAP_FRAGMENT,
        "Progression": FlagCategory.PROGRESSION,
        "NPC": FlagCategory.NPC,
        "Quest NPC": FlagCategory.NPC,
        "Merchant": FlagCategory.MERCHANT,
        "Finger Reader": FlagCategory.MERCHANT,
        "Stake of Marika": FlagCategory.STAKE_OF_MARIKA,
        "Spirit Spring": FlagCategory.SPIRIT_SPRING,
        "Boss Arena": FlagCategory.BOSS_ARENA,
        "Shop Stock": FlagCategory.SHOP_STOCK,
        "Shop Unlock": FlagCategory.SHOP_UNLOCK,
        "Ash of War Unlock": FlagCategory.SHOP_UNLOCK,
        "Remembrance": FlagCategory.REMEMBRANCE,
        "Pot Upgrade": FlagCategory.POT_UPGRADE,
        "Crystal Tear": FlagCategory.CRYSTAL_TEAR,
        "Crystal Tear (DLC)": FlagCategory.CRYSTAL_TEAR,
        "Great Rune Possession": FlagCategory.GREAT_RUNE,
        "Great Rune Activation": FlagCategory.GREAT_RUNE,
        "Mausoleum Duplication": FlagCategory.MAUSOLEUM,
        "Dungeon Cleared": FlagCategory.BOSS_DEFEAT,
        "Boss World Drop": FlagCategory.BOSS_DEFEAT,
        "Boss Discovery": FlagCategory.BOSS_DEFEAT,
        "Invasion Defeat": FlagCategory.BOSS_DEFEAT,
        "Enemy Defeat": FlagCategory.BOSS_DEFEAT,
        "Elite Enemy Defeat": FlagCategory.BOSS_DEFEAT,
        "Unknown": FlagCategory.UNKNOWN,
    }

    def __init__(self):
        self.extracted_flags: List[ExtractedFlag] = []
        self.manual_completions: List[ManualCompletion] = []

    def load_extracted_flags(self, filepath: str | Path) -> List[ExtractedFlag]:
        """
        Load flags from extracted_event_flags.json.
        """
        filepath = Path(filepath)

        with open(filepath, 'r', encoding='utf-8') as f:
            data = json.load(f)

        flags = []
        for entry in data.get("flags", []):
            flag = ExtractedFlag(
                flag_id=entry.get("flag_id", 0),
                name=entry.get("name", "Unknown"),
                category=entry.get("category", "Unknown"),
                region=entry.get("region", "Unknown"),
                source_file=entry.get("source_file"),
                source_row_id=entry.get("source_row_id"),
                item_id=entry.get("item_id"),
                map_tile=entry.get("map_tile"),
                pos_x=entry.get("pos_x"),
                pos_y=entry.get("pos_y"),
                pos_z=entry.get("pos_z"),
                is_dlc=entry.get("is_dlc", False),
                treasure_type=entry.get("treasure_type"),
            )
            flags.append(flag)

        self.extracted_flags = flags

        # Print summary
        print(f"Loaded {len(flags)} extracted flags from {filepath.name}")

        # Count by category
        categories = {}
        for flag in flags:
            categories[flag.category] = categories.get(flag.category, 0) + 1

        print("Category distribution:")
        for cat, count in sorted(categories.items(), key=lambda x: -x[1]):
            print(f"  {cat}: {count}")

        return flags

    def load_manual_completions(self, filepath: str | Path) -> List[ManualCompletion]:
        """
        Load manual completions from flag-correlation-candidates.jsonl or legacy txt format.

        JSONL format (preferred):
        {"flagId": 76117, "flagName": "Saintsbridge", "flagCategory": "Grace",
         "slotIndex": 5, "userMarkedComplete": true, ...}

        Legacy txt format:
        - Lines starting with # are comments
        - Each completion is a line with: Name (optional details)
        """
        filepath = Path(filepath)
        completions = []

        # Handle JSONL format
        if filepath.suffix == '.jsonl':
            with open(filepath, 'r', encoding='utf-8') as f:
                for line in f:
                    line = line.strip()
                    if not line:
                        continue
                    try:
                        record = json.loads(line)
                        # Only include records where userMarkedComplete is true
                        if record.get('userMarkedComplete', False):
                            completions.append(ManualCompletion(
                                name=record.get('flagName', 'Unknown'),
                                flag_id=record.get('flagId'),
                                category=record.get('flagCategory', 'Unknown'),
                                slot=record.get('slotIndex'),
                                completed=True,
                                notes=record.get('flagRegion')
                            ))
                    except json.JSONDecodeError:
                        continue
        else:
            # Legacy txt format parsing
            current_category = "Unknown"

            with open(filepath, 'r', encoding='utf-8') as f:
                lines = f.readlines()

            for line in lines:
                line = line.strip()

                if not line:
                    continue

                if line.startswith("## "):
                    current_category = line[3:].strip()
                    continue

                if line.startswith("#"):
                    continue

                name = line
                flag_id = None
                slot = None
                notes = None

                flag_match = re.search(r'flag[:\s]+(\d+)', line, re.IGNORECASE)
                if flag_match:
                    flag_id = int(flag_match.group(1))
                    name = line[:flag_match.start()].strip(' -')

                slot_match = re.search(r'\(slot\s*(\d+)\)', line, re.IGNORECASE)
                if slot_match:
                    slot = int(slot_match.group(1))
                    name = re.sub(r'\(slot\s*\d+\)', '', name).strip()

                notes_match = re.search(r'\(([^)]+)\)', line)
                if notes_match and not slot_match:
                    notes = notes_match.group(1)

                completions.append(ManualCompletion(
                    name=name.strip(' -:'),
                    flag_id=flag_id,
                    category=current_category,
                    slot=slot,
                    completed=True,
                    notes=notes
                ))

        self.manual_completions = completions

        print(f"\nLoaded {len(completions)} manual completions from {filepath.name}")

        # Count by category
        categories = {}
        for comp in completions:
            categories[comp.category] = categories.get(comp.category, 0) + 1

        print("Category distribution:")
        for cat, count in sorted(categories.items(), key=lambda x: -x[1]):
            print(f"  {cat}: {count}")

        return completions

    def match_manual_to_extracted(self) -> Dict[str, Any]:
        """
        Try to match manual completions to extracted flags.

        Returns a dict with matched, unmatched_manual, and unmatched_extracted.
        """
        matched = []
        unmatched_manual = []

        # Build lookup by name (normalized)
        extracted_by_name = {}
        extracted_by_flag = {}
        for flag in self.extracted_flags:
            # Normalize name for matching
            norm_name = self._normalize_name(flag.name)
            if norm_name not in extracted_by_name:
                extracted_by_name[norm_name] = []
            extracted_by_name[norm_name].append(flag)

            extracted_by_flag[flag.flag_id] = flag

        matched_flag_ids = set()

        for manual in self.manual_completions:
            match = None

            # Try flag ID match first
            if manual.flag_id and manual.flag_id in extracted_by_flag:
                match = extracted_by_flag[manual.flag_id]
            else:
                # Try name match
                norm_name = self._normalize_name(manual.name)
                if norm_name in extracted_by_name:
                    candidates = extracted_by_name[norm_name]
                    # If multiple candidates, try to find best match by category
                    if len(candidates) == 1:
                        match = candidates[0]
                    else:
                        # Try category match
                        for cand in candidates:
                            if self._categories_match(manual.category, cand.category):
                                match = cand
                                break
                        if not match:
                            match = candidates[0]  # Take first if no category match

            if match:
                matched.append({
                    "manual": manual,
                    "extracted": match,
                    "flag_id": match.flag_id
                })
                matched_flag_ids.add(match.flag_id)
            else:
                unmatched_manual.append(manual)

        # Find unmatched extracted flags (optional - for completeness)
        # Focusing only on priority categories
        priority_categories = {"Grace", "Boss Defeat", "Great Boss Defeat", "Field Boss Defeat",
                              "Cookbook", "Whetblade", "World Pickup", "Dungeon Pickup"}

        unmatched_extracted = [
            f for f in self.extracted_flags
            if f.flag_id not in matched_flag_ids and f.category in priority_categories
        ]

        return {
            "matched": matched,
            "unmatched_manual": unmatched_manual,
            "unmatched_extracted": unmatched_extracted
        }

    def _normalize_name(self, name: str) -> str:
        """Normalize a name for matching."""
        # Remove common suffixes
        name = re.sub(r'\s*-\s*(Grace|Defeated|Pickup|Boss).*$', '', name, flags=re.IGNORECASE)
        # Remove parenthetical content
        name = re.sub(r'\s*\([^)]*\)', '', name)
        # Normalize whitespace
        name = ' '.join(name.split())
        # Lowercase
        return name.lower().strip()

    def _categories_match(self, manual_cat: str, extracted_cat: str) -> bool:
        """Check if categories are compatible."""
        manual_cat = manual_cat.lower()
        extracted_cat = extracted_cat.lower()

        if manual_cat == extracted_cat:
            return True

        # Grace variations
        if "grace" in manual_cat and "grace" in extracted_cat:
            return True

        # Boss variations
        if "boss" in manual_cat and "boss" in extracted_cat:
            return True

        return False

    def create_verification_entries(
        self,
        categories_filter: Optional[List[str]] = None
    ) -> List[FlagVerification]:
        """
        Create FlagVerification entries from extracted flags.

        Args:
            categories_filter: Only include these categories (or all if None)

        Returns:
            List of FlagVerification objects ready for testing
        """
        entries = []

        for flag in self.extracted_flags:
            # Filter by category if specified
            if categories_filter:
                if flag.category not in categories_filter:
                    continue

            # Map category string to enum
            category_enum = self.CATEGORY_MAP.get(flag.category, FlagCategory.UNKNOWN)

            entry = FlagVerification(
                flag_id=flag.flag_id,
                name=flag.name,
                category=category_enum,
                region=flag.region,
                source_file=flag.source_file,
                source_row_id=flag.source_row_id,
                coordinates={
                    "x": flag.pos_x,
                    "y": flag.pos_y,
                    "z": flag.pos_z
                } if flag.pos_x is not None else None
            )

            entries.append(entry)

        return entries

    def get_priority_flags(self) -> List[FlagVerification]:
        """
        Get flags for priority categories (Graces + Bosses).
        """
        priority = [
            "Grace",
            "Boss Defeat",
            "Great Boss Defeat",
            "Field Boss Defeat",
        ]
        return self.create_verification_entries(priority)

    def get_all_trackable_flags(self) -> List[FlagVerification]:
        """
        Get all trackable flags (excluding known untrackable categories).
        """
        # Exclude categories that can't be tracked
        untrackable = ["Unknown"]

        all_categories = set(f.category for f in self.extracted_flags)
        trackable = [c for c in all_categories if c not in untrackable]

        return self.create_verification_entries(trackable)


# Convenience function
def load_verification_data(
    extracted_flags_path: str,
    manual_completions_path: str
) -> DataLoader:
    """Load all verification data."""
    loader = DataLoader()
    loader.load_extracted_flags(extracted_flags_path)
    loader.load_manual_completions(manual_completions_path)
    return loader


if __name__ == "__main__":
    import sys

    # Default paths
    base_path = Path(__file__).parent.parent.parent
    extracted_path = base_path / "scripts" / "extracted_event_flags.json"
    manual_path = Path("/Users/laszloprekop/dev/Elden Ring stuff/elden-map/server/data/flag-correlation-candidates.jsonl")

    loader = DataLoader()

    if extracted_path.exists():
        loader.load_extracted_flags(extracted_path)

    if manual_path.exists():
        loader.load_manual_completions(manual_path)

        if loader.extracted_flags and loader.manual_completions:
            print("\n" + "=" * 60)
            print("MATCHING MANUAL TO EXTRACTED")
            print("=" * 60)

            matches = loader.match_manual_to_extracted()
            print(f"\nMatched: {len(matches['matched'])}")
            print(f"Unmatched manual: {len(matches['unmatched_manual'])}")

            if matches['unmatched_manual']:
                print("\nUnmatched manual completions:")
                for m in matches['unmatched_manual']:
                    print(f"  - {m.name} ({m.category})")

            print("\n" + "=" * 60)
            print("PRIORITY FLAGS FOR VERIFICATION")
            print("=" * 60)
            priority = loader.get_priority_flags()
            print(f"Total priority flags: {len(priority)}")

            by_cat = {}
            for p in priority:
                cat = p.category.value
                by_cat[cat] = by_cat.get(cat, 0) + 1

            for cat, count in sorted(by_cat.items()):
                print(f"  {cat}: {count}")
