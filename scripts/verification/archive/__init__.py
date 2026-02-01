"""
Archive directory for deprecated verification modules.

These modules are kept for backward compatibility but should NOT be used
in new code. Use ground_truth_loader.py instead.

Deprecated modules:
- flag_formulas.py: Contains hardcoded values that are out of sync with
  ground_truth_offsets.json. Use load_block_bases(), get_tile_config(), etc.
"""

from .flag_formulas import FlagFormulas

__all__ = ["FlagFormulas"]
