#!/usr/bin/env python3
"""
Comprehensive search for 71008 (Main Gate) across ALL valid EF offsets
and ALL potential bases.
"""

from pathlib import Path

SAVE_DIR = Path("/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files")
BACKUP_FILE = SAVE_DIR / "ER0000-backup-2026-01-11.sl2"

SLOT_0_OFFSET = 0x310
SLOT_SIZE = 0x280010
SEARCH_START = 0x12000

VALIDATION_FLAGS = [
    (71800, 2725, 7, "Cave of Knowledge"),
    (71801, 2725, 6, "Stranded Graveyard"),
    (76100, 3262, 3, "The First Step"),
    (76101, 3262, 2, "Church of Elleh"),
]

NEGATIVE_FLAGS = [
    (76300, 3287, 3, "Zamor Ruins"),
    (76301, 3287, 2, "Ancient Snow Valley"),
    (76350, 3293, 5, "Haligtree Town"),
]

STORMVEIL_GRACES = [
    (71000, "Godrick the Grafted"),
    (71001, "Margit, the Fell Omen"),
    (71002, "Castleward Tunnel"),
    (71003, "Gateside Chamber"),
    (71004, "Stormveil Cliffside"),
    (71005, "Rampart Tower"),
    (71006, "Liftside Chamber"),
    (71007, "Secluded Cell"),
    (71008, "Stormveil Main Gate"),
]

def read_slot_data(save_file, slot_index):
    with open(save_file, 'rb') as f:
        slot_start = SLOT_0_OFFSET + slot_index * SLOT_SIZE
        f.seek(slot_start)
        return f.read(SLOT_SIZE)

def check_flag(data, offset, byte_off, bit_pos):
    abs_pos = offset + byte_off
    if abs_pos < len(data):
        return bool(data[abs_pos] & (1 << bit_pos))
    return False

def main():
    print("=" * 70)
    print("COMPREHENSIVE SEARCH FOR 71008 (STORMVEIL MAIN GATE)")
    print("=" * 70)

    slot0_data = read_slot_data(BACKUP_FILE, 0)

    # Find top EF start candidates
    candidates = []
    max_search = 200_000
    search_end = min(SEARCH_START + max_search, len(slot0_data) - 0x1bf99f)

    for test_offset in range(SEARCH_START, search_end):
        pos_score = 0
        for flag_id, byte_off, bit_pos, name in VALIDATION_FLAGS:
            if check_flag(slot0_data, test_offset, byte_off, bit_pos):
                pos_score += 1

        if pos_score == 4:
            neg_score = 0
            for flag_id, byte_off, bit_pos, name in NEGATIVE_FLAGS:
                if not check_flag(slot0_data, test_offset, byte_off, bit_pos):
                    neg_score += 1
            candidates.append((test_offset, neg_score))

    # Sort by negative score
    candidates.sort(key=lambda x: -x[1])
    top_candidates = candidates[:20]  # Top 20 EF offsets

    print(f"\nChecking {len(top_candidates)} top EF start candidates")

    # For each EF offset, search for a base where 71008 is SET with good coverage
    best_results = []

    for ef_offset, neg_score in top_candidates:
        # Search bases 2500-3000
        for test_base in range(2500, 3000):
            local_71008 = 8
            byte_71008 = test_base + local_71008 // 8
            bit_71008 = 7 - (local_71008 % 8)

            if check_flag(slot0_data, ef_offset, byte_71008, bit_71008):
                # 71008 is SET at this combination
                # Count other graces
                set_count = 0
                flags_set = []
                for flag_id, name in STORMVEIL_GRACES:
                    local = flag_id - 71000
                    bo = test_base + local // 8
                    bp = 7 - (local % 8)
                    if check_flag(slot0_data, ef_offset, bo, bp):
                        set_count += 1
                        flags_set.append(flag_id)

                if set_count >= 5:
                    best_results.append({
                        'ef_offset': ef_offset,
                        'neg_score': neg_score,
                        'base': test_base,
                        'set_count': set_count,
                        'flags_set': flags_set
                    })

    if best_results:
        # Sort by set_count descending, then neg_score descending
        best_results.sort(key=lambda x: (-x['set_count'], -x['neg_score']))

        print(f"\nFound {len(best_results)} combinations where 71008 is SET with 5+ graces")
        print("\nTop 10 results:")
        for r in best_results[:10]:
            print(f"\n  EF=0x{r['ef_offset']:X} (neg={r['neg_score']}), Base={r['base']}: {r['set_count']}/9")
            print(f"    Flags SET: {r['flags_set']}")
    else:
        print("\nNo combination found where 71008 is SET with 5+ other graces")

        # Broader search - look for ANY combination with 71008 SET
        print("\nBroader search - finding any base where 71008 is SET...")

        any_71008_set = []
        for ef_offset, neg_score in top_candidates[:5]:
            for test_base in range(2000, 5000):
                local_71008 = 8
                byte_71008 = test_base + local_71008 // 8
                bit_71008 = 7 - (local_71008 % 8)

                if check_flag(slot0_data, ef_offset, byte_71008, bit_71008):
                    set_count = 0
                    for flag_id, name in STORMVEIL_GRACES:
                        local = flag_id - 71000
                        bo = test_base + local // 8
                        bp = 7 - (local % 8)
                        if check_flag(slot0_data, ef_offset, bo, bp):
                            set_count += 1

                    if set_count >= 3:
                        any_71008_set.append((ef_offset, neg_score, test_base, set_count))

        if any_71008_set:
            print(f"\nFound {len(any_71008_set)} combinations with 71008 SET and 3+ graces:")
            for ef_off, neg, base, count in sorted(any_71008_set, key=lambda x: -x[3])[:15]:
                print(f"  EF=0x{ef_off:X}, Base={base}: {count}/9")
        else:
            print("No combination found even with 3+ graces threshold")

    # Final check: what does 71008 look like at the most promising EF offset?
    print("\n" + "=" * 70)
    print("RAW BYTE ANALYSIS FOR 71008")
    print("=" * 70)

    # Use 0x125BD which showed 6/9 graces
    ef_offset = 0x125BD
    print(f"\nAt EF offset 0x{ef_offset:X}:")

    # Show bytes around where 71008 would be at base 2673
    base = 2673
    byte_71008 = base + 1  # 2674

    print(f"\n71008 would be at byte {byte_71008}, bit 7")
    abs_pos = ef_offset + byte_71008
    byte_val = slot0_data[abs_pos]
    print(f"  Byte {byte_71008} absolute 0x{abs_pos:X}: 0x{byte_val:02X} ({byte_val:08b})")
    print(f"  Bit 7: {bool(byte_val & 0x80)}")

    # Show surrounding bytes
    print(f"\nSurrounding bytes:")
    for i in range(-3, 5):
        pos = abs_pos + i
        if 0 <= pos < len(slot0_data):
            val = slot0_data[pos]
            mark = " <-- 71008" if i == 0 else ""
            print(f"  Byte {byte_71008 + i}: 0x{val:02X} ({val:08b}){mark}")

if __name__ == "__main__":
    main()
