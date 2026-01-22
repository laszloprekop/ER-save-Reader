#!/usr/bin/env python3
"""
EMEVD Event Graph Extraction System

Parses all 589 EMEVD files to build a queryable graph of:
- Flag triggers (what action sets each flag via SetEventFlagID)
- Flag dependencies (prerequisite flags checked via EventFlag)
- Entity-to-flag mappings (chrEntityId, assetEntityId -> flags)
- Progression chains (boss defeats, remembrances, map fragments)

Output: scripts/event_graph.json for Rust loader consumption.
"""

import json
import re
from pathlib import Path
from dataclasses import dataclass, field, asdict
from typing import Dict, List, Optional, Set, Tuple, Any
from collections import defaultdict
from datetime import datetime

# === Configuration ===
GAME_FILES = Path("/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring decompiled game files")
EVENT_DIR = GAME_FILES / "event"
OUTPUT_FILE = Path(__file__).parent / "event_graph.json"

# === Regex Patterns ===

# Event definition: $Event(event_id, restart_type, function(params) { ... })
EVENT_DEF_PATTERN = re.compile(
    r'\$Event\((\d+),\s*(\w+),\s*function\(([^)]*)\)\s*\{',
    re.MULTILINE
)

# SetEventFlagID with literal flag: SetEventFlagID(12345, ON/OFF)
SET_FLAG_LITERAL = re.compile(r'SetEventFlagID\((\d+),\s*(ON|OFF)\)')

# SetEventFlagID with parameter: SetEventFlagID(eventFlagId, ON/OFF)
SET_FLAG_PARAM = re.compile(r'SetEventFlagID\((\w+),\s*(ON|OFF)\)')

# EventFlag condition check: EventFlag(flag_id) or EventFlag(param)
EVENT_FLAG_CONDITION = re.compile(r'EventFlag\((\w+|\d+)\)')

# $InitializeEvent: $InitializeEvent(slot, event_id, arg1, arg2, ...)
INIT_EVENT = re.compile(r'\$InitializeEvent\((\d+),\s*(\d+)(?:,\s*([^)]+))?\)')

# $InitializeCommonEvent: $InitializeCommonEvent(slot, event_id, arg1, arg2, ...)
INIT_COMMON_EVENT = re.compile(r'\$InitializeCommonEvent\((\d+),\s*(\d+)(?:,\s*([^)]+))?\)')

# InitializeEvent (without $): InitializeEvent(slot, event_id, ...)
INIT_EVENT_NO_PREFIX = re.compile(r'(?<!\$)InitializeEvent\((\d+),\s*(\d+)(?:,\s*([^)]+))?\)')

# RegisterBonfire: RegisterBonfire(flag_id, entity_id, ...)
REGISTER_BONFIRE = re.compile(r'RegisterBonfire\((\d+),\s*(\d+)')

# Common entity patterns in event calls
CHR_ENTITY_PATTERN = re.compile(r'chrEntityId|entityId|bossEntityId')
ASSET_ENTITY_PATTERN = re.compile(r'assetEntityId|treasureAssetId')


# === Data Classes ===

@dataclass
class FlagTrigger:
    """What action sets a specific flag"""
    event_id: int
    source_file: str
    action: str  # "ON" or "OFF"
    trigger_context: str  # "boss_defeat", "item_pickup", "grace_discovery", etc.
    entity_id: Optional[int] = None
    line_number: Optional[int] = None

@dataclass
class FlagDependency:
    """Prerequisite relationship between flags"""
    required_flag: int
    condition_type: str  # "EventFlag", "SpEffect", etc.
    source_event: int
    source_file: str

@dataclass
class EntityFlagMapping:
    """Maps entity IDs to their associated flags"""
    entity_type: str  # "chr", "asset", "bonfire"
    map_tile: str
    associated_flags: List[Dict]  # [{flag_id, relationship}]

@dataclass
class ProgressionChain:
    """Known progression chain (boss defeat, remembrance, map fragment)"""
    chain_type: str  # "remembrance", "map_fragment", "grace"
    boss_defeat: Optional[int] = None
    item_lot: Optional[int] = None
    possession_flag: Optional[int] = None
    event_id: int = 0
    params: List[int] = field(default_factory=list)

@dataclass
class EventTemplate:
    """Template from common_func for parameter resolution"""
    event_id: int
    parameters: List[str]
    flag_operations: List[Dict]  # [{param_index, action}]
    flag_conditions: List[Dict]  # [{param_index}]
    source_file: str


class EventGraphExtractor:
    """Main extraction engine for EMEVD event graphs."""

    def __init__(self):
        self.flag_triggers: Dict[int, List[FlagTrigger]] = defaultdict(list)
        self.flag_dependencies: Dict[int, Dict] = defaultdict(lambda: {"depends_on": [], "enables": []})
        self.entity_flag_map: Dict[int, EntityFlagMapping] = {}
        self.progression_chains: Dict[str, ProgressionChain] = {}
        self.event_templates: Dict[int, EventTemplate] = {}

        self.stats = {
            "files_parsed": 0,
            "total_flags_found": set(),
            "total_triggers": 0,
            "total_dependencies": 0,
            "errors": []
        }

    def extract_map_tile(self, filename: str) -> str:
        """Extract map tile ID from filename (e.g., m60_42_37_00.emevd.js -> m60_42_37)"""
        match = re.match(r'(m\d+_\d+_\d+)_\d+\.emevd\.js', filename)
        return match.group(1) if match else "unknown"

    def infer_trigger_context(self, event_id: int, source_file: str, params: List[int] = None) -> str:
        """Infer the trigger context based on event ID patterns and parameters."""
        # Known common_func event patterns
        if 90005880 <= event_id <= 90005889:
            return "boss_defeat"
        if event_id == 90005100:
            return "grace_discovery"
        if 90005600 <= event_id <= 90005699:
            return "asset_interaction"
        if 900005610 <= event_id <= 900005619:
            return "asset_interaction"
        if event_id == 1100:
            return "remembrance"
        if event_id == 1600:
            return "map_fragment"

        # Entity ID patterns in params
        if params:
            for p in params:
                if p >= 1000000000:
                    # 10-digit entity ID
                    suffix = str(p)[-4:]
                    if suffix in ("0800", "0801", "0805"):
                        return "boss_defeat"
                    if suffix == "1950":
                        return "grace"

        return "event_script"

    def parse_event_parameters(self, param_str: str) -> List[str]:
        """Parse function parameter names from string."""
        if not param_str or not param_str.strip():
            return []
        return [p.strip() for p in param_str.split(',') if p.strip()]

    def parse_init_arguments(self, args_str: str) -> List[int]:
        """Parse $InitializeEvent arguments to integer list."""
        if not args_str:
            return []
        args = []
        for arg in args_str.split(','):
            arg = arg.strip()
            # Handle special values
            if arg in ('true', 'false', 'ON', 'OFF'):
                args.append(1 if arg in ('true', 'ON') else 0)
            elif arg.startswith('-'):
                try:
                    args.append(int(arg))
                except ValueError:
                    args.append(0)
            elif arg.isdigit():
                args.append(int(arg))
            elif '.' in arg:
                # Float handling - skip or convert
                try:
                    args.append(int(float(arg)))
                except ValueError:
                    args.append(0)
            else:
                # Enum or variable - skip
                args.append(0)
        return args

    def extract_event_body(self, content: str, start_pos: int) -> Tuple[str, int]:
        """Extract event body by matching braces from start position."""
        brace_count = 0
        in_body = False
        body_start = start_pos

        for i, char in enumerate(content[start_pos:], start_pos):
            if char == '{':
                if not in_body:
                    in_body = True
                    body_start = i + 1
                brace_count += 1
            elif char == '}':
                brace_count -= 1
                if brace_count == 0 and in_body:
                    return content[body_start:i], i

        return "", len(content)

    def parse_common_func(self, filepath: Path) -> Dict[int, EventTemplate]:
        """Parse common_func.emevd.js for event templates."""
        templates = {}

        if not filepath.exists():
            print(f"Warning: {filepath} not found")
            return templates

        content = filepath.read_text(encoding='utf-8', errors='ignore')

        for match in EVENT_DEF_PATTERN.finditer(content):
            event_id = int(match.group(1))
            params = self.parse_event_parameters(match.group(3))

            # Extract event body
            body, _ = self.extract_event_body(content, match.end() - 1)

            # Find flag operations in body
            flag_ops = []
            for set_match in SET_FLAG_PARAM.finditer(body):
                param_name = set_match.group(1)
                action = set_match.group(2)
                if param_name in params:
                    flag_ops.append({
                        "param_index": params.index(param_name),
                        "param_name": param_name,
                        "action": action
                    })

            # Find flag conditions
            flag_conds = []
            for cond_match in EVENT_FLAG_CONDITION.finditer(body):
                cond_val = cond_match.group(1)
                if cond_val in params:
                    flag_conds.append({
                        "param_index": params.index(cond_val),
                        "param_name": cond_val
                    })

            if flag_ops or flag_conds:
                templates[event_id] = EventTemplate(
                    event_id=event_id,
                    parameters=params,
                    flag_operations=flag_ops,
                    flag_conditions=flag_conds,
                    source_file="common_func.emevd.js"
                )

        print(f"  Parsed {len(templates)} event templates from common_func")
        return templates

    def parse_common_emevd(self, filepath: Path):
        """Parse common.emevd.js for known chains and relationships."""
        if not filepath.exists():
            print(f"Warning: {filepath} not found")
            return

        content = filepath.read_text(encoding='utf-8', errors='ignore')

        # Extract Event 1100 (Remembrances)
        remembrance_pattern = re.compile(
            r'\$InitializeEvent\((\d+),\s*1100,\s*(\d+),\s*(\d+),\s*(\d+),\s*(\d+)\)'
        )
        for match in remembrance_pattern.finditer(content):
            slot = int(match.group(1))
            boss_flag = int(match.group(2))
            item_lot = int(match.group(3))
            item_lot2 = int(match.group(4))
            possession_flag = int(match.group(5))

            chain_key = f"remembrance_{boss_flag}"
            self.progression_chains[chain_key] = ProgressionChain(
                chain_type="remembrance",
                boss_defeat=boss_flag,
                item_lot=item_lot,
                possession_flag=possession_flag,
                event_id=1100,
                params=[boss_flag, item_lot, item_lot2, possession_flag]
            )

            # Record trigger
            self.flag_triggers[possession_flag].append(FlagTrigger(
                event_id=1100,
                source_file="common.emevd.js",
                action="ON",
                trigger_context="remembrance"
            ))
            self.stats["total_flags_found"].add(possession_flag)

        # Extract Event 1600 (Map Fragments)
        map_pattern = re.compile(
            r'\$InitializeEvent\((\d+),\s*1600,\s*(\d+),\s*(\d+),\s*(\d+),\s*(\d+)\)'
        )
        for match in map_pattern.finditer(content):
            slot = int(match.group(1))
            discovery_flag = int(match.group(2))
            possession_flag = int(match.group(3))
            entity1 = int(match.group(4))
            entity2 = int(match.group(5))

            chain_key = f"map_fragment_{discovery_flag}"
            self.progression_chains[chain_key] = ProgressionChain(
                chain_type="map_fragment",
                possession_flag=possession_flag,
                event_id=1600,
                params=[discovery_flag, possession_flag, entity1, entity2]
            )

            self.flag_triggers[discovery_flag].append(FlagTrigger(
                event_id=1600,
                source_file="common.emevd.js",
                action="ON",
                trigger_context="map_fragment"
            ))
            self.stats["total_flags_found"].add(discovery_flag)

        # Extract Event 960 (Grace discovery - First Step etc.)
        grace_pattern = re.compile(
            r'\$InitializeEvent\((\d+),\s*960,\s*(\d+)\)'
        )
        for match in grace_pattern.finditer(content):
            slot = int(match.group(1))
            grace_flag = int(match.group(2))

            self.flag_triggers[grace_flag].append(FlagTrigger(
                event_id=960,
                source_file="common.emevd.js",
                action="ON",
                trigger_context="grace_discovery"
            ))
            self.stats["total_flags_found"].add(grace_flag)

        print(f"  Found {len(self.progression_chains)} progression chains in common.emevd.js")

    def parse_map_file(self, filepath: Path):
        """Parse a single map EMEVD file for triggers and dependencies."""
        filename = filepath.name
        map_tile = self.extract_map_tile(filename)

        try:
            content = filepath.read_text(encoding='utf-8', errors='ignore')
        except Exception as e:
            self.stats["errors"].append(f"Error reading {filename}: {e}")
            return

        # Extract literal SetEventFlagID calls
        for match in SET_FLAG_LITERAL.finditer(content):
            flag_id = int(match.group(1))
            action = match.group(2)

            # Find containing event
            event_id = self.find_containing_event(content, match.start())

            self.flag_triggers[flag_id].append(FlagTrigger(
                event_id=event_id,
                source_file=filename,
                action=action,
                trigger_context=self.infer_trigger_context(event_id, filename)
            ))
            self.stats["total_flags_found"].add(flag_id)

        # Extract $InitializeCommonEvent calls
        for match in INIT_COMMON_EVENT.finditer(content):
            slot = int(match.group(1))
            event_id = int(match.group(2))
            args_str = match.group(3)
            args = self.parse_init_arguments(args_str) if args_str else []

            # Resolve template if available
            if event_id in self.event_templates:
                template = self.event_templates[event_id]
                context = self.infer_trigger_context(event_id, filename, args)

                for op in template.flag_operations:
                    idx = op["param_index"]
                    if idx < len(args) and args[idx] != 0:
                        flag_id = args[idx]
                        self.flag_triggers[flag_id].append(FlagTrigger(
                            event_id=event_id,
                            source_file=filename,
                            action=op["action"],
                            trigger_context=context,
                            entity_id=args[0] if args else None
                        ))
                        self.stats["total_flags_found"].add(flag_id)

            # Record entity mappings for boss events
            if 90005880 <= event_id <= 90005889 and len(args) >= 4:
                entity_id = args[0]
                if entity_id >= 1000000000:
                    flags = [a for a in args[1:11] if a >= 1000000]
                    if entity_id not in self.entity_flag_map:
                        self.entity_flag_map[entity_id] = EntityFlagMapping(
                            entity_type="chr",
                            map_tile=map_tile,
                            associated_flags=[]
                        )
                    for flag in flags:
                        self.entity_flag_map[entity_id].associated_flags.append({
                            "flag_id": flag,
                            "relationship": "boss_defeat"
                        })

        # Extract $InitializeEvent calls (local events)
        for match in INIT_EVENT.finditer(content):
            event_id = int(match.group(2))
            args_str = match.group(3)
            args = self.parse_init_arguments(args_str) if args_str else []

            # Record any large flag-like values
            for arg in args:
                if 60000 <= arg < 100000000:
                    self.flag_triggers[arg].append(FlagTrigger(
                        event_id=event_id,
                        source_file=filename,
                        action="ON",
                        trigger_context=self.infer_trigger_context(event_id, filename, args)
                    ))
                    self.stats["total_flags_found"].add(arg)

        # Extract RegisterBonfire calls (grace flags)
        for match in REGISTER_BONFIRE.finditer(content):
            grace_flag = int(match.group(1))
            entity_id = int(match.group(2))

            self.flag_triggers[grace_flag].append(FlagTrigger(
                event_id=0,
                source_file=filename,
                action="ON",
                trigger_context="grace_registration",
                entity_id=entity_id
            ))
            self.stats["total_flags_found"].add(grace_flag)

            # Entity mapping
            if entity_id not in self.entity_flag_map:
                self.entity_flag_map[entity_id] = EntityFlagMapping(
                    entity_type="bonfire",
                    map_tile=map_tile,
                    associated_flags=[]
                )
            self.entity_flag_map[entity_id].associated_flags.append({
                "flag_id": grace_flag,
                "relationship": "grace_flag"
            })

        # Extract EventFlag conditions for dependency tracking
        for event_match in EVENT_DEF_PATTERN.finditer(content):
            event_id = int(event_match.group(1))
            body, _ = self.extract_event_body(content, event_match.end() - 1)

            # Find conditions and flag sets in same event
            conditions = []
            for cond_match in EVENT_FLAG_CONDITION.finditer(body):
                cond_val = cond_match.group(1)
                if cond_val.isdigit():
                    conditions.append(int(cond_val))

            triggers = []
            for set_match in SET_FLAG_LITERAL.finditer(body):
                triggers.append(int(set_match.group(1)))

            # Link conditions to triggers
            for trigger_flag in triggers:
                for cond_flag in conditions:
                    if cond_flag != trigger_flag:
                        self.flag_dependencies[trigger_flag]["depends_on"].append({
                            "required_flag": cond_flag,
                            "condition_type": "EventFlag",
                            "source_event": event_id,
                            "source_file": filename
                        })
                        self.flag_dependencies[cond_flag]["enables"].append({
                            "enabled_flag": trigger_flag,
                            "relationship": "prerequisite"
                        })

        self.stats["files_parsed"] += 1

    def find_containing_event(self, content: str, position: int) -> int:
        """Find the event ID containing a given position."""
        # Search backwards for $Event definition
        search_start = max(0, position - 10000)
        search_region = content[search_start:position]

        matches = list(EVENT_DEF_PATTERN.finditer(search_region))
        if matches:
            return int(matches[-1].group(1))
        return 0

    def run_extraction(self):
        """Run the full extraction pipeline."""
        print("=" * 60)
        print("EMEVD Event Graph Extraction")
        print("=" * 60)

        # Phase 1: Parse common_func for templates
        print("\nPhase 1: Parsing common_func.emevd.js templates...")
        common_func_path = EVENT_DIR / "common_func.emevd.js"
        self.event_templates = self.parse_common_func(common_func_path)

        # Phase 2: Parse common.emevd.js for known chains
        print("\nPhase 2: Parsing common.emevd.js chains...")
        common_path = EVENT_DIR / "common.emevd.js"
        self.parse_common_emevd(common_path)

        # Phase 3: Parse all map files
        print("\nPhase 3: Parsing map EMEVD files...")
        map_files = sorted(EVENT_DIR.glob("m*.emevd.js"))
        total_files = len(map_files)

        for i, filepath in enumerate(map_files):
            if (i + 1) % 100 == 0:
                print(f"  Progress: {i + 1}/{total_files} files...")
            self.parse_map_file(filepath)

        print(f"  Completed: {total_files} map files parsed")

        # Calculate final stats
        self.stats["total_triggers"] = sum(len(t) for t in self.flag_triggers.values())
        self.stats["total_dependencies"] = sum(
            len(d["depends_on"]) for d in self.flag_dependencies.values()
        )

        print("\n" + "=" * 60)
        print("Extraction Summary")
        print("=" * 60)
        print(f"  Files parsed: {self.stats['files_parsed']}")
        print(f"  Unique flags found: {len(self.stats['total_flags_found'])}")
        print(f"  Total triggers: {self.stats['total_triggers']}")
        print(f"  Total dependencies: {self.stats['total_dependencies']}")
        print(f"  Entity mappings: {len(self.entity_flag_map)}")
        print(f"  Progression chains: {len(self.progression_chains)}")
        if self.stats["errors"]:
            print(f"  Errors: {len(self.stats['errors'])}")

    def to_json(self) -> Dict:
        """Convert extraction results to JSON-serializable dict."""
        # Convert flag_triggers
        triggers_dict = {}
        for flag_id, triggers in self.flag_triggers.items():
            triggers_dict[str(flag_id)] = {
                "flag_id": flag_id,
                "triggers": [
                    {
                        "event_id": t.event_id,
                        "source_file": t.source_file,
                        "action": t.action,
                        "trigger_context": t.trigger_context,
                        "entity_id": t.entity_id
                    }
                    for t in triggers
                ]
            }

        # Convert flag_dependencies
        deps_dict = {}
        for flag_id, deps in self.flag_dependencies.items():
            if deps["depends_on"] or deps["enables"]:
                deps_dict[str(flag_id)] = {
                    "flag_id": flag_id,
                    "depends_on": deps["depends_on"],
                    "enables": deps["enables"]
                }

        # Convert entity mappings
        entity_dict = {}
        for entity_id, mapping in self.entity_flag_map.items():
            entity_dict[str(entity_id)] = {
                "entity_type": mapping.entity_type,
                "map_tile": mapping.map_tile,
                "associated_flags": mapping.associated_flags
            }

        # Convert progression chains
        chains_dict = {}
        for chain_key, chain in self.progression_chains.items():
            chains_dict[chain_key] = {
                "chain_type": chain.chain_type,
                "boss_defeat": chain.boss_defeat,
                "item_lot": chain.item_lot,
                "possession_flag": chain.possession_flag,
                "event_id": chain.event_id,
                "params": chain.params
            }

        return {
            "metadata": {
                "extraction_date": datetime.now().isoformat(),
                "emevd_files_parsed": self.stats["files_parsed"],
                "total_unique_flags": len(self.stats["total_flags_found"]),
                "total_triggers": self.stats["total_triggers"],
                "total_dependencies": self.stats["total_dependencies"],
                "entity_mappings": len(self.entity_flag_map),
                "progression_chains": len(self.progression_chains)
            },
            "flag_triggers": triggers_dict,
            "flag_dependencies": deps_dict,
            "entity_flag_map": entity_dict,
            "progression_chains": chains_dict
        }

    def save(self, output_path: Path = None):
        """Save extraction results to JSON file."""
        path = output_path or OUTPUT_FILE
        data = self.to_json()

        with open(path, 'w', encoding='utf-8') as f:
            json.dump(data, f, indent=2)

        print(f"\nSaved to: {path}")
        print(f"File size: {path.stat().st_size / 1024:.1f} KB")


def main():
    extractor = EventGraphExtractor()
    extractor.run_extraction()
    extractor.save()

    # Verification: check known flags
    print("\n" + "=" * 60)
    print("Verification: Known Flag Checks")
    print("=" * 60)

    known_flags = [
        (76100, "First Step grace"),
        (1042370800, "Tree Sentinel boss entity"),
        (9100, "Godrick remembrance boss flag"),
        (62010, "Limgrave West map fragment discovery"),
    ]

    for flag_id, description in known_flags:
        has_trigger = flag_id in extractor.flag_triggers
        trigger_count = len(extractor.flag_triggers.get(flag_id, []))
        status = "FOUND" if has_trigger else "MISSING"
        print(f"  [{status}] {flag_id}: {description} ({trigger_count} triggers)")


if __name__ == "__main__":
    main()
