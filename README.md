# ER Save Reader

Reads Elden Ring save files and reconstructs the character state they encode — the same
state the game itself loads when you continue playing. Compatible with PC and Playstation
Save Wizard exported saves.

**It is a reader, not an editor** (see [ADR-0009](docs/adr/0009-a-reader-not-an-editor.md)).
It opens your save and shows you what is in it; it never writes one back. The write-back
path from the project's editor days is still in the tree but dormant behind the
`save-writeback` Cargo feature, off by default.

Started as a fork of [ClayAmore/ER-Save-Editor](https://github.com/ClayAmore/ER-Save-Editor)
and has since diverged into a state-reconstruction and event-flag research tool.

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (1.70 or later recommended)
- Cargo (comes with Rust)

## Setup

1. Clone the repository:
   ```bash
   git clone https://github.com/laszloprekop/ER-save-Reader.git
   cd ER-save-Reader
   ```

2. Build the project:
   ```bash
   cargo build --release
   ```

## Running

### Development mode
```bash
cargo run
```

### Release mode (optimized)
```bash
cargo run --release
```

Or run the compiled binary directly:
```bash
./target/release/er-save-reader
```

### Using the Editor

1. Launch the application
2. Click "Open" or drag and drop a save file (`.sl2` for PC, `.txt` for PlayStation Save Wizard exports)
3. Select a character from the left panel
4. Use the section menu to navigate between General, Stats, Equipment, Inventory, Event Flags, and Regions
5. Make your edits
6. Click "Save" to save changes to a file

## Features
- Import characters from other save files
- Export character data to JSON for backup or analysis
- Change PC save file SteamID
- Modify player name
- Change player gender
- Edit player stats
- Modify soul count
- Add items, weapons, armors, ashes of war, and talismans to inventory
- Add items in bulk to speed up build making process
- Browse inventory
- Change player equipment
- Activate/deactivate Sites of Grace, summoning pools, colosseums, etc.
- Revive or kill bosses
- Activate/deactivate invasion regions
- More features will be added in future updates

## Save File Locations

### PC (Steam)
```
%APPDATA%\EldenRing\<SteamID>\ER0000.sl2
```

### PlayStation (via Save Wizard)
Export your save using PlayStation Save Wizard to a `.txt` file.

## Permissions
Feel free to use this save editor for learning or development purposes. However, I do not authorize its use for creating tools or modifications that enable actions online outside the bounds of what the game allows.

## FAQ
Q: Will this ban me?<br/>
A: There's no guarantee that you won't be banned. None of these features have been tested online.

## Reporting Issues
If you encounter any bugs or issues while using the save editor, please report them. When reporting bugs, try to provide reproducible steps so I can debug effectively.

## Credits
<a href="https://github.com/nordgaren/"><img src="https://github.com/ClayAmore/ER-Save-Editor/assets/131625063/710c9ee6-c3df-4665-be6b-d96bce1ebf46"/></a>

## Disclaimer

This project is not affiliated with FromSoftware or Bandai Namco Entertainment. Elden Ring is a trademark of FromSoftware and Bandai Namco Entertainment. All rights reserved.
This code is a fork of the original project by [ClayAmore](https://github.com/ClayAmore/ER-Save-Editor) and is intended for add DLC support to the original project. Meanwhile, 
the original project is updated, so I will try to keep this project updated with the latest DLC features.
