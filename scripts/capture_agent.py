#!/usr/bin/env python3
"""
Capture Agent for Automated Snapshot Workflow

This script provides a local agent that:
1. Captures save file snapshots with proper naming
2. Extracts slot context (EF offset, calibrated bases) at capture time
3. Maintains the capture_catalog.json with before/after pairing
4. Optionally runs as HTTP server for webapp integration

Usage:
    # Capture a before snapshot
    python capture_agent.py capture --phase before --flag-id 1044360040 --poi-name "Somber Stone" --slot 0

    # Capture an after snapshot (auto-pairs with most recent before)
    python capture_agent.py capture --phase after --flag-id 1044360040 --poi-name "Somber Stone" --slot 0

    # Run as HTTP server for webapp integration
    python capture_agent.py serve --port 8765

    # Migrate existing snapshots into catalog
    python capture_agent.py migrate

    # Show catalog status
    python capture_agent.py status
"""

from __future__ import annotations

import argparse
import json
import os
import re
import shutil
import sys
from dataclasses import dataclass, field, asdict
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple
from http.server import HTTPServer, BaseHTTPRequestHandler
import urllib.parse

# Add parent to path for imports
sys.path.insert(0, str(Path(__file__).parent))

from verification.save_parser import SaveParser
from verification.utils import detect_event_flags_start, extract_event_flags


# ============================================================================
# CONFIGURATION
# ============================================================================

# Directories
SAVE_DIR = Path("/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files")
SNAPSHOT_DIR = SAVE_DIR / "Granular snapshots for debugging"
ACTIVE_SAVE = SAVE_DIR / "ER0000.sl2"
CATALOG_PATH = SNAPSHOT_DIR / "capture_catalog.json"

# Slot character mapping (from CLAUDE.md)
SLOT_CHARACTERS = {
    0: "Confessor",
    1: "Wretch",
    2: "V1",
    3: "V2",
    4: "V3",
    6: "Sam",
}

# Known tile base offset (from ground_truth) - REVERTED 2026-01-25: 489981 was wrong
TILE_BASE_OFFSET = 485330


# ============================================================================
# DATA CLASSES
# ============================================================================

@dataclass
class POIData:
    """Point of Interest data for a capture."""
    id: Optional[str] = None
    name: Optional[str] = None
    category: Optional[str] = None
    flag_id: Optional[int] = None
    flag_format: Optional[str] = None  # "tile", "dungeon", "block"
    map_tile: Optional[str] = None
    coordinates: Optional[Dict[str, float]] = None


@dataclass
class SlotContext:
    """Slot-specific context extracted at capture time."""
    slot_index: int
    character_name: Optional[str] = None
    ef_offset: Optional[int] = None
    calibrated_tile_base: Optional[int] = None
    calibrated_dungeon_bases: Optional[Dict[str, int]] = None


@dataclass
class Capture:
    """A single capture record."""
    id: str
    filename: str
    timestamp: str
    phase: str  # "before" or "after"
    poi: POIData
    slot_context: SlotContext
    pair_id: Optional[str] = None
    notes: Optional[str] = None


@dataclass
class CapturePair:
    """A before/after capture pair."""
    pair_id: str
    before_capture: str
    after_capture: str
    flag_id: Optional[int] = None
    action_type: Optional[str] = None  # "pickup", "grace", "boss", etc.
    auto_chained: bool = False
    verification_result: Optional[Dict[str, Any]] = None
    tags: List[str] = field(default_factory=list)
    notes: Optional[str] = None


@dataclass
class CaptureCatalog:
    """The complete capture catalog."""
    version: str = "1.0"
    metadata: Dict[str, Any] = field(default_factory=dict)
    slot_calibrations: Dict[str, Any] = field(default_factory=dict)
    captures: List[Dict[str, Any]] = field(default_factory=list)
    pairs: List[Dict[str, Any]] = field(default_factory=list)


# ============================================================================
# CATALOG MANAGEMENT
# ============================================================================

def load_catalog() -> Dict[str, Any]:
    """Load the capture catalog from disk."""
    if CATALOG_PATH.exists():
        with open(CATALOG_PATH, 'r') as f:
            return json.load(f)
    return {
        "version": "1.0",
        "metadata": {
            "created": datetime.now(timezone.utc).isoformat(),
            "last_updated": datetime.now(timezone.utc).isoformat(),
            "capture_count": 0,
            "pair_count": 0,
        },
        "slot_calibrations": {"slots": {}},
        "captures": [],
        "pairs": [],
    }


def save_catalog(catalog: Dict[str, Any]) -> None:
    """Save the capture catalog to disk."""
    catalog["metadata"]["last_updated"] = datetime.now(timezone.utc).isoformat()
    catalog["metadata"]["capture_count"] = len(catalog.get("captures", []))
    catalog["metadata"]["pair_count"] = len(catalog.get("pairs", []))

    with open(CATALOG_PATH, 'w') as f:
        json.dump(catalog, f, indent=2, default=str)


def generate_capture_id(catalog: Dict[str, Any]) -> str:
    """Generate next capture ID."""
    count = len(catalog.get("captures", []))
    return f"cap_{count + 1:03d}"


def generate_pair_id(catalog: Dict[str, Any]) -> str:
    """Generate next pair ID."""
    count = len(catalog.get("pairs", []))
    return f"pair_{count + 1:03d}"


# ============================================================================
# SLOT CONTEXT EXTRACTION
# ============================================================================

def extract_slot_context(save_path: Path, slot_index: int) -> SlotContext:
    """
    Extract slot-specific context from a save file.

    This includes:
    - Character name
    - Event flags offset (varies with GaItems count)
    - Calibrated formula bases (if determinable)
    """
    parser = SaveParser()

    try:
        parsed = parser.parse(save_path, slots_to_parse=[slot_index])
        if parsed.slots and len(parsed.slots) > 0:
            slot = parsed.slots[0]
            return SlotContext(
                slot_index=slot_index,
                character_name=slot.character_name or SLOT_CHARACTERS.get(slot_index, f"Slot{slot_index}"),
                ef_offset=slot.event_flags_offset,
                calibrated_tile_base=TILE_BASE_OFFSET,  # Use known good value
                calibrated_dungeon_bases=None,  # TODO: Calibrate from known flags
            )
    except Exception as e:
        print(f"Warning: Could not fully parse save: {e}")

    # Fallback to basic context
    return SlotContext(
        slot_index=slot_index,
        character_name=SLOT_CHARACTERS.get(slot_index, f"Slot{slot_index}"),
        ef_offset=None,
        calibrated_tile_base=TILE_BASE_OFFSET,
        calibrated_dungeon_bases=None,
    )


# ============================================================================
# FLAG FORMAT DETECTION
# ============================================================================

def detect_flag_format(flag_id: int) -> str:
    """Determine the flag format from the flag ID."""
    if flag_id is None:
        return "unknown"

    if 1_000_000_000 <= flag_id < 3_000_000_000:
        return "tile"  # 10-digit: 1XXYYZZZZ or 2XXYYZZZZ
    elif 10_000_000 <= flag_id < 100_000_000:
        return "dungeon"  # 8-digit: AASSZZZZ
    elif 60_000 <= flag_id < 100_000:
        return "block"  # 5-6 digit
    else:
        return "unknown"


def extract_map_tile_from_flag(flag_id: int) -> Optional[str]:
    """Extract map tile from a tile-format flag ID."""
    if not (1_000_000_000 <= flag_id < 3_000_000_000):
        return None

    flag_str = str(flag_id)
    if len(flag_str) != 10:
        return None

    # Format: 1XXYYZZZZ or 2XXYYZZZZ
    prefix = flag_str[0]  # 1=base, 2=DLC
    xx = int(flag_str[1:3])
    yy = int(flag_str[3:5])

    area_no = 60 if prefix == "1" else 61  # Approximate
    return f"m{area_no}_{xx}_{yy}"


def extract_area_from_dungeon_flag(flag_id: int) -> Optional[int]:
    """Extract area ID from a dungeon-format flag."""
    if not (10_000_000 <= flag_id < 100_000_000):
        return None

    # Format: AASSZZZZ
    return flag_id // 1_000_000


# ============================================================================
# CAPTURE FUNCTIONS
# ============================================================================

def capture_snapshot(
    phase: str,
    flag_id: Optional[int],
    poi_name: Optional[str],
    slot_index: int,
    category: Optional[str] = None,
    notes: Optional[str] = None,
    auto_chain: bool = True,
) -> Capture:
    """
    Capture a save file snapshot.

    Args:
        phase: "before" or "after"
        flag_id: The POI's storable flag ID (row_id for tiles, NOT getItemFlagId)
        poi_name: Human-readable name of the POI
        slot_index: Character slot index
        category: POI category (e.g., "Item Pickup", "Grace", "Boss")
        notes: Optional notes about the capture
        auto_chain: If True, auto-chain after captures to most recent before

    Returns:
        Capture record
    """
    if not ACTIVE_SAVE.exists():
        raise FileNotFoundError(f"Save file not found: {ACTIVE_SAVE}")

    catalog = load_catalog()

    # Generate IDs
    capture_id = generate_capture_id(catalog)
    capture_index = len(catalog.get("captures", [])) + 1

    # Detect flag format and map tile
    flag_format = detect_flag_format(flag_id) if flag_id else "unknown"
    map_tile = extract_map_tile_from_flag(flag_id) if flag_id else None

    # Extract slot context
    slot_context = extract_slot_context(ACTIVE_SAVE, slot_index)

    # Generate filename
    # Format: ER0000.sl2_capture_{index}_{phase}_{flag_id}_{map_tile}
    parts = [
        f"ER0000.sl2_capture_{capture_index:03d}_{phase}",
    ]
    if flag_id:
        parts.append(str(flag_id))
    if map_tile:
        parts.append(map_tile)

    filename = "_".join(parts)

    # Copy save file
    dest_path = SNAPSHOT_DIR / filename
    shutil.copy2(ACTIVE_SAVE, dest_path)
    print(f"Captured: {filename}")

    # Create capture record
    poi = POIData(
        name=poi_name,
        category=category,
        flag_id=flag_id,
        flag_format=flag_format,
        map_tile=map_tile,
    )

    capture = Capture(
        id=capture_id,
        filename=filename,
        timestamp=datetime.now(timezone.utc).isoformat(),
        phase=phase,
        poi=poi,
        slot_context=slot_context,
        notes=notes,
    )

    # Add to catalog
    catalog["captures"].append(asdict(capture))

    # Handle pairing
    if phase == "after" and auto_chain:
        pair_id = _create_pair(catalog, capture, flag_id)
        if pair_id:
            capture.pair_id = pair_id
            # Update the capture in catalog with pair_id
            catalog["captures"][-1]["pair_id"] = pair_id

    save_catalog(catalog)

    return capture


def _create_pair(catalog: Dict[str, Any], after_capture: Capture, flag_id: Optional[int]) -> Optional[str]:
    """
    Create a capture pair by finding or linking a before capture.

    Returns pair_id if created.
    """
    captures = catalog.get("captures", [])

    # Find matching before capture
    before_capture = None
    auto_chained = False

    # First, look for an exact match (same flag_id, before phase, not yet paired)
    for cap in reversed(captures[:-1]):  # Exclude the just-added after capture
        if cap.get("phase") == "before" and not cap.get("pair_id"):
            cap_flag_id = cap.get("poi", {}).get("flag_id")
            if cap_flag_id == flag_id:
                before_capture = cap
                break

    # If no exact match, auto-chain to most recent unpaired before
    if not before_capture:
        for cap in reversed(captures[:-1]):
            if cap.get("phase") == "before" and not cap.get("pair_id"):
                before_capture = cap
                auto_chained = True
                break

    if not before_capture:
        print("Warning: No matching 'before' capture found for pairing")
        return None

    # Create pair
    pair_id = generate_pair_id(catalog)

    # Determine action type from category
    category = after_capture.poi.category or ""
    action_type = "unknown"
    if "pickup" in category.lower() or "item" in category.lower():
        action_type = "pickup"
    elif "grace" in category.lower():
        action_type = "grace"
    elif "boss" in category.lower():
        action_type = "boss"

    # Generate tags
    tags = []
    if after_capture.poi.flag_format:
        tags.append(f"{after_capture.poi.flag_format}_formula_test")
    if action_type != "unknown":
        tags.append(action_type)

    pair = CapturePair(
        pair_id=pair_id,
        before_capture=before_capture["id"],
        after_capture=after_capture.id,
        flag_id=flag_id,
        action_type=action_type,
        auto_chained=auto_chained,
        tags=tags,
        notes=f"Auto-chained from capture {before_capture['id']}" if auto_chained else None,
    )

    # Update before capture with pair_id
    for cap in catalog["captures"]:
        if cap["id"] == before_capture["id"]:
            cap["pair_id"] = pair_id
            break

    catalog["pairs"].append(asdict(pair))
    print(f"Created pair: {pair_id} ({before_capture['id']} -> {after_capture.id})")

    return pair_id


# ============================================================================
# MIGRATION: PARSE EXISTING SNAPSHOTS
# ============================================================================

def parse_snapshot_filename(filename: str) -> Optional[Dict[str, Any]]:
    """
    Parse an existing snapshot filename to extract metadata.

    Handles various formats:
    - "ER0000.sl2 Confessor - 01 before Missionary Cookbok [4] pickup"
    - "ER0000.sl2 S0 - b1 before pillaging EF-1044367040 rowId-1044360040 mapTile-m60_44_36"
    - "ER0000.sl2 Wretch - 05 Cave of knowledge, rested at Site of grace"
    """
    result = {
        "filename": filename,
        "character": None,
        "slot_index": None,
        "sequence": None,
        "phase": None,
        "action": None,
        "flag_id": None,
        "row_id": None,
        "map_tile": None,
        "ef_id": None,  # The potentially misleading getItemFlagId from filenames
    }

    # Pattern 1: "ER0000.sl2 {Character} - {NN} {phase} {description}"
    match1 = re.match(
        r"ER0000\.sl2\s+(\w+)\s*-\s*(\d+)\s+(before|after)?\s*(.*)",
        filename,
        re.IGNORECASE
    )

    # Pattern 2: "ER0000.sl2 S{N} - b{NN} {phase} {action} EF-{id} rowId-{id} mapTile-{tile}"
    match2 = re.match(
        r"ER0000\.sl2\s+S(\d+)\s*-\s*b(\d+)\s+(before|after)?\s*(.*)",
        filename,
        re.IGNORECASE
    )

    if match2:
        result["slot_index"] = int(match2.group(1))
        result["character"] = SLOT_CHARACTERS.get(result["slot_index"])
        result["sequence"] = int(match2.group(2))
        result["phase"] = match2.group(3).lower() if match2.group(3) else None
        rest = match2.group(4)

        # Extract EF-xxx (this may be getItemFlagId - need to check rowId)
        ef_match = re.search(r"EF-(\d+)", rest)
        if ef_match:
            result["ef_id"] = int(ef_match.group(1))

        # Extract rowId-xxx (this is the correct storable flag)
        row_match = re.search(r"rowId-(\d+)", rest)
        if row_match:
            result["row_id"] = int(row_match.group(1))
            # Use row_id as the flag_id (correct for tile pickups)
            result["flag_id"] = result["row_id"]
        elif result["ef_id"]:
            # Fallback to EF if no rowId (may need correction)
            result["flag_id"] = result["ef_id"]

        # Extract mapTile
        tile_match = re.search(r"mapTile-(m\d+_\d+_\d+)", rest)
        if tile_match:
            result["map_tile"] = tile_match.group(1)

        # Extract action description
        action = re.sub(r"EF-\d+|rowId-\d+|mapTile-m\d+_\d+_\d+", "", rest).strip()
        result["action"] = action if action else None

    elif match1:
        result["character"] = match1.group(1)
        result["sequence"] = int(match1.group(2))
        result["phase"] = match1.group(3).lower() if match1.group(3) else None
        result["action"] = match1.group(4).strip() if match1.group(4) else None

        # Map character to slot
        for slot_idx, char_name in SLOT_CHARACTERS.items():
            if char_name.lower() == result["character"].lower():
                result["slot_index"] = slot_idx
                break

    return result if result["character"] or result["slot_index"] is not None else None


def migrate_existing_snapshots() -> Dict[str, Any]:
    """
    Parse existing snapshot files and add them to the catalog.

    Returns migration statistics.
    """
    catalog = load_catalog()
    existing_filenames = {c["filename"] for c in catalog.get("captures", [])}

    stats = {
        "scanned": 0,
        "added": 0,
        "skipped_existing": 0,
        "skipped_unparseable": 0,
        "pairs_created": 0,
    }

    # Scan slot directories
    for subdir in sorted(SNAPSHOT_DIR.iterdir()):
        if subdir.is_dir() and subdir.name.startswith("slot"):
            for snapshot in sorted(subdir.iterdir()):
                if snapshot.name.startswith("ER0000.sl2"):
                    stats["scanned"] += 1

                    if snapshot.name in existing_filenames:
                        stats["skipped_existing"] += 1
                        continue

                    parsed = parse_snapshot_filename(snapshot.name)
                    if not parsed:
                        stats["skipped_unparseable"] += 1
                        continue

                    # Create capture record
                    capture_id = generate_capture_id(catalog)

                    slot_ctx = SlotContext(
                        slot_index=parsed.get("slot_index", 0),
                        character_name=parsed.get("character"),
                        ef_offset=None,  # Would need to parse save to get this
                        calibrated_tile_base=TILE_BASE_OFFSET,
                    )

                    poi = POIData(
                        name=parsed.get("action"),
                        flag_id=parsed.get("flag_id"),
                        flag_format=detect_flag_format(parsed.get("flag_id")),
                        map_tile=parsed.get("map_tile"),
                    )

                    capture = Capture(
                        id=capture_id,
                        filename=snapshot.name,
                        timestamp=datetime.fromtimestamp(
                            snapshot.stat().st_mtime, tz=timezone.utc
                        ).isoformat(),
                        phase=parsed.get("phase", "unknown"),
                        poi=poi,
                        slot_context=slot_ctx,
                        notes=f"Migrated from existing snapshot. EF-{parsed.get('ef_id')} in filename may be getItemFlagId." if parsed.get("ef_id") and parsed.get("ef_id") != parsed.get("row_id") else None,
                    )

                    catalog["captures"].append(asdict(capture))
                    stats["added"] += 1

    # Create pairs from consecutive before/after captures
    captures = catalog.get("captures", [])
    captures_by_slot = {}
    for cap in captures:
        slot = cap.get("slot_context", {}).get("slot_index", 0)
        if slot not in captures_by_slot:
            captures_by_slot[slot] = []
        captures_by_slot[slot].append(cap)

    for slot, slot_captures in captures_by_slot.items():
        # Sort by sequence number or timestamp
        sorted_caps = sorted(slot_captures, key=lambda c: c.get("timestamp", ""))

        i = 0
        while i < len(sorted_caps) - 1:
            current = sorted_caps[i]
            next_cap = sorted_caps[i + 1]

            if (current.get("phase") == "before" and
                next_cap.get("phase") == "after" and
                not current.get("pair_id") and
                not next_cap.get("pair_id")):

                # Create pair
                pair_id = generate_pair_id(catalog)
                current["pair_id"] = pair_id
                next_cap["pair_id"] = pair_id

                flag_id = next_cap.get("poi", {}).get("flag_id") or current.get("poi", {}).get("flag_id")

                pair = CapturePair(
                    pair_id=pair_id,
                    before_capture=current["id"],
                    after_capture=next_cap["id"],
                    flag_id=flag_id,
                    action_type="unknown",
                    auto_chained=False,
                    tags=[],
                )
                catalog["pairs"].append(asdict(pair))
                stats["pairs_created"] += 1
                i += 2  # Skip the after capture
            else:
                i += 1

    save_catalog(catalog)
    return stats


# ============================================================================
# HTTP SERVER FOR WEBAPP INTEGRATION
# ============================================================================

class CaptureHandler(BaseHTTPRequestHandler):
    """HTTP handler for capture requests from webapp."""

    def _send_json(self, data: Dict[str, Any], status: int = 200):
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Access-Control-Allow-Origin", "*")
        self.end_headers()
        self.wfile.write(json.dumps(data).encode())

    def do_OPTIONS(self):
        """Handle CORS preflight."""
        self.send_response(200)
        self.send_header("Access-Control-Allow-Origin", "*")
        self.send_header("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
        self.send_header("Access-Control-Allow-Headers", "Content-Type")
        self.end_headers()

    def do_GET(self):
        """Handle GET requests."""
        parsed = urllib.parse.urlparse(self.path)

        if parsed.path == "/status":
            catalog = load_catalog()
            self._send_json({
                "status": "running",
                "capture_count": catalog["metadata"].get("capture_count", 0),
                "pair_count": catalog["metadata"].get("pair_count", 0),
                "last_updated": catalog["metadata"].get("last_updated"),
            })
        elif parsed.path == "/catalog":
            catalog = load_catalog()
            self._send_json(catalog)
        else:
            self._send_json({"error": "Not found"}, 404)

    def do_POST(self):
        """Handle POST requests."""
        parsed = urllib.parse.urlparse(self.path)

        if parsed.path == "/capture":
            content_length = int(self.headers.get("Content-Length", 0))
            body = json.loads(self.rfile.read(content_length).decode()) if content_length else {}

            try:
                capture = capture_snapshot(
                    phase=body.get("phase", "before"),
                    flag_id=body.get("flag_id"),
                    poi_name=body.get("poi_name"),
                    slot_index=body.get("slot_index", 0),
                    category=body.get("category"),
                    notes=body.get("notes"),
                )
                self._send_json({"success": True, "capture": asdict(capture)})
            except Exception as e:
                self._send_json({"success": False, "error": str(e)}, 500)
        else:
            self._send_json({"error": "Not found"}, 404)

    def log_message(self, format, *args):
        """Suppress default logging."""
        pass


def run_server(port: int = 8765):
    """Run the HTTP server."""
    server = HTTPServer(("localhost", port), CaptureHandler)
    print(f"Capture agent listening on http://localhost:{port}")
    print("Endpoints:")
    print("  GET  /status  - Server status")
    print("  GET  /catalog - Full capture catalog")
    print("  POST /capture - Capture a snapshot")
    print("\nPress Ctrl+C to stop")
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        print("\nShutting down...")
        server.shutdown()


# ============================================================================
# CLI
# ============================================================================

def show_status():
    """Show catalog status."""
    catalog = load_catalog()
    print(f"\nCapture Catalog Status")
    print(f"=" * 40)
    print(f"Captures: {catalog['metadata'].get('capture_count', 0)}")
    print(f"Pairs: {catalog['metadata'].get('pair_count', 0)}")
    print(f"Last Updated: {catalog['metadata'].get('last_updated', 'N/A')}")

    # Count by slot
    captures = catalog.get("captures", [])
    by_slot = {}
    for cap in captures:
        slot = cap.get("slot_context", {}).get("slot_index", "unknown")
        by_slot[slot] = by_slot.get(slot, 0) + 1

    if by_slot:
        print(f"\nCaptures by Slot:")
        for slot, count in sorted(by_slot.items()):
            char = SLOT_CHARACTERS.get(slot, "Unknown")
            print(f"  Slot {slot} ({char}): {count}")

    # Show recent captures
    recent = captures[-5:] if captures else []
    if recent:
        print(f"\nRecent Captures:")
        for cap in recent:
            phase = cap.get("phase", "?")
            flag = cap.get("poi", {}).get("flag_id", "N/A")
            name = cap.get("poi", {}).get("name", "Unknown")[:30]
            print(f"  [{cap['id']}] {phase}: {name} (flag: {flag})")


def main():
    parser = argparse.ArgumentParser(description="Capture Agent for Snapshot Workflow")
    subparsers = parser.add_subparsers(dest="command", help="Commands")

    # capture command
    cap_parser = subparsers.add_parser("capture", help="Capture a snapshot")
    cap_parser.add_argument("--phase", choices=["before", "after"], required=True)
    cap_parser.add_argument("--flag-id", type=int, help="Storable flag ID (row_id for tiles)")
    cap_parser.add_argument("--poi-name", help="POI name")
    cap_parser.add_argument("--slot", type=int, default=0, help="Character slot index")
    cap_parser.add_argument("--category", help="POI category")
    cap_parser.add_argument("--notes", help="Optional notes")

    # serve command
    serve_parser = subparsers.add_parser("serve", help="Run HTTP server")
    serve_parser.add_argument("--port", type=int, default=8765, help="Port number")

    # migrate command
    subparsers.add_parser("migrate", help="Migrate existing snapshots to catalog")

    # status command
    subparsers.add_parser("status", help="Show catalog status")

    args = parser.parse_args()

    if args.command == "capture":
        capture = capture_snapshot(
            phase=args.phase,
            flag_id=args.flag_id,
            poi_name=args.poi_name,
            slot_index=args.slot,
            category=args.category,
            notes=args.notes,
        )
        print(f"\nCapture complete: {capture.id}")
        print(f"File: {capture.filename}")

    elif args.command == "serve":
        run_server(args.port)

    elif args.command == "migrate":
        print("Migrating existing snapshots...")
        stats = migrate_existing_snapshots()
        print(f"\nMigration complete:")
        print(f"  Scanned: {stats['scanned']}")
        print(f"  Added: {stats['added']}")
        print(f"  Pairs created: {stats['pairs_created']}")
        print(f"  Skipped (existing): {stats['skipped_existing']}")
        print(f"  Skipped (unparseable): {stats['skipped_unparseable']}")

    elif args.command == "status":
        show_status()

    else:
        parser.print_help()


if __name__ == "__main__":
    main()
