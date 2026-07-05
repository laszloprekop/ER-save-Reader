"""EF detection via the Rust reference implementation (ADR-0005).

Python tooling must NOT re-implement save parsing or anchor detection.
This module is the single sanctioned bridge: it shells out to
`er-save-editor discovery ef-dump`, which runs the fixture-pinned
gaEnd-windowed detection from crates/wasm-event-flags.

The returned `ef_offset` is the GRACE-FAMILY base. Per the 2026-07-05
per-family float finding, it must not be used to position other flag
families (catacombs, tiles, ...) byte-exactly across saves.
"""
from __future__ import annotations

import json
import subprocess
import tempfile
from functools import lru_cache
from pathlib import Path
from typing import Any, Dict, Optional

_REPO_ROOT = Path(__file__).resolve().parents[2]
_BINARY_CANDIDATES = [
    _REPO_ROOT / "target" / "release" / "er-save-editor",
    _REPO_ROOT / "target" / "debug" / "er-save-editor",
]


class EfDumpError(RuntimeError):
    pass


@lru_cache(maxsize=1)
def _binary() -> Path:
    for candidate in _BINARY_CANDIDATES:
        if candidate.exists():
            return candidate
    raise EfDumpError(
        "er-save-editor binary not found. Build it first: cargo build --release. "
        "Python-side EF detection was removed per ADR-0005 "
        "(single reference implementation in crates/wasm-event-flags)."
    )


def _run(args: list[str]) -> Dict[str, Any]:
    proc = subprocess.run(
        [str(_binary()), "discovery", "ef-dump", *args],
        capture_output=True,
        text=True,
    )
    if proc.returncode != 0:
        raise EfDumpError(f"ef-dump failed: {proc.stderr.strip() or proc.stdout.strip()}")
    return json.loads(proc.stdout)


def ef_dump_file(save_path: str | Path, slot: Optional[int] = None) -> Dict[str, Any]:
    """Run ef-dump on a full .sl2 save file. Returns the parsed JSON document."""
    args = [str(save_path)]
    if slot is not None:
        args += ["--slot", str(slot)]
    return _run(args)


def detect_ef_offset_bytes(slot_data: bytes) -> Dict[str, Any]:
    """Detect the grace-family EF base for raw slot bytes.

    Returns the slot entry dict: ef_offset, ga_items_end, confident,
    positive_score, negative_score, ...
    """
    with tempfile.NamedTemporaryFile(suffix=".slot", delete=True) as tmp:
        tmp.write(slot_data)
        tmp.flush()
        doc = _run([tmp.name, "--raw-slot"])
    slots = doc.get("slots") or []
    if not slots:
        raise EfDumpError("ef-dump returned no slots for raw slot input")
    return slots[0]
