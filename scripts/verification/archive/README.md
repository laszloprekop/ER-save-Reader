# Verification Script Archive

This directory contains superseded or historical verification scripts that are preserved for reference but should not be used for active verification.

## Criteria for Archiving

Scripts should be moved here when:

1. **Superseded by better scripts** - A more comprehensive or accurate script exists
2. **One-time investigation completed** - The investigation is done and results documented
3. **Uses deprecated methods** - Hardcoded offsets that are now in ground_truth
4. **Duplicates functionality** - Now available in shared utils.py

## How to Archive

1. Move the script to this directory
2. Add a note at the top of the script explaining why it was archived
3. Update any documentation that references the script

## Note on flag_formulas.py

`flag_formulas.py` in the parent directory is **deprecated** but NOT archived because:
- `save_parser.py` still imports from it (backward compatibility)
- The deprecation notice in the file explains to use `ground_truth_loader.py` instead

New scripts should use `ground_truth_loader.py` for all offset calculations.
