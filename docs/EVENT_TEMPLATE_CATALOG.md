# Event Template Catalog - common_func.emevd.js

This document catalogs all non-respawning event templates from Elden Ring's `common_func.emevd.js` file. These templates handle permanent progression: boss defeats, item pickups, grace discovery, NPC interactions, and quest states.

> **Epistemic header** (audited 2026-07-20 · BACKLOG step 6)
> **Status: STABLE REFERENCE — game-derived, era-independent.** A catalog of EMEVD event-template semantics (what each template does). This is game knowledge (regulation/EMEVD 1.16.x), not a claim about save-byte positions, so the migration does not affect it.
> - **Claims**: the behavior of `common_func.emevd.js` event templates (boss defeat, pickup, grace, door, quest).
> - **Evidence**: the game's EMEVD (raw corpus `game-raw-1162`).
> - **Methodology**: reading the decompiled/raw event scripts.
> - **Obsolete**: none substantive. The `.emevd.js` decompiles referenced here were not regenerated after the 2026-07-05 reset — the pipeline now parses raw `.emevd` (see `CLAUDE.md`); template semantics are unchanged.

## Legend

- **Event ID**: The template function ID
- **Parameters**: Named parameters with types (flag = event flag, chr = character, asset = object, etc.)
- **Sets Flags**: Whether it calls SetEventFlagID or SetNetworkconnectedEventFlagID
- **Trigger**: What condition causes the flag to be set
- **Purpose**: What this template is used for

---

## Boss & Enemy Defeat Templates

### 90005300 - Enemy Death with Item Drop
**Parameters**: `eventFlagId, chrEntityId, itemLotId, timeSeconds, value`
**Sets Flag**: YES - `SetEventFlagID(eventFlagId, ON)` on death
**Trigger**: `CharacterRatioDead(chrEntityId)`
**Purpose**: Standard enemy death handler. Sets flag when enemy dies, optionally awards item lot after delay.
- `value`: Controls corpse handling (0=normal death, non-zero=treasure corpse)

### 90005301 - Enemy Death with Item Drop (Variant)
**Parameters**: `eventFlagId, chrEntityId, itemLotId, timeSeconds, value`
**Sets Flag**: YES - `SetEventFlagID(eventFlagId, ON)` on death
**Trigger**: `CharacterRatioDead(chrEntityId)`
**Purpose**: Nearly identical to 90005300, uses `SignedAlt()` checks instead of `Signed()`

### 90005360 - Generic Event Flag Wait
**Parameters**: `eventFlagId, chrEntityId, itemLotId`
**Sets Flag**: NO (waits for flag, doesn't set it)
**Trigger**: `EventFlag(eventFlagId)`
**Purpose**: Waits for a flag to be set, then disables character and awards items. Used for triggered events.

### 90005390 - Death with Transformation SFX
**Parameters**: `eventFlagId, eventFlagId2, entityId, chrEntityId, value, itemLotId`
**Sets Flag**: YES - `SetNetworkconnectedEventFlagID(eventFlagId, ON)`
**Trigger**: `EventFlag(eventFlagId2) && CharacterDead(chrEntityId)`
**Purpose**: Enemy death that spawns special effect (death animation), then awards items. Used for special enemies.

### 90005391 - Death with Character Swap
**Parameters**: `eventFlagId, eventFlagId2, chrEntityId, chrEntityId2, value`
**Sets Flag**: YES - `SetNetworkconnectedEventFlagID(eventFlagId2, ON)` (phase change flag)
**Trigger**: `CharacterDead(chrEntityId)`
**Purpose**: When character 1 dies, swap to character 2 with SFX. Used for phase transitions (e.g., NPC invasions).

### 9005840 - Boss Death (Demigod Banner)
**Parameters**: `eventFlagId, eventFlagId2, chrEntityId`
**Sets Flag**: YES - `SetEventFlagID(eventFlagId, ON)` and optionally `eventFlagId2`
**Trigger**: `CharacterHPValue(chrEntityId) <= 0` then `CharacterDead(chrEntityId)`
**Purpose**: Main boss defeat handler. Shows "DEMIGOD FELLED" banner, sets boss flag. Used for major bosses.

### 9005845 - Elden Beast Special (Example)
**Parameters**: `eventFlagId, chrEntityId`
**Sets Flag**: YES - `SetNetworkconnectedEventFlagID(11000801, ON)` (hardcoded)
**Trigger**: Player proximity or damage
**Purpose**: Special boss activation logic for Elden Beast (event 11000800). Not a general template.

### 90005860 - Boss Defeat with Item Reward
**Parameters**: `eventFlagId, eventFlagId2, chrEntityId, value, itemLotId, timeSeconds`
**Sets Flag**: YES - `SetEventFlagID(eventFlagId, ON)` and optionally `eventFlagId2`
**Trigger**: `CharacterHPValue(chrEntityId) <= 0` then `CharacterDead(chrEntityId)`
**Purpose**: Boss death with banner (type based on `value`: 0=Enemy Felled, 1=Great Enemy, 2=Great Enemy, 3=Demigod), awards item lot after delay.
- `value`: 0=EnemyFelled, 1=GreatEnemyFelled, 2=GreatEnemyFelled, 3=DemigodFelled

### 90005861 - Boss Defeat with Item and Message
**Parameters**: `eventFlagId, eventFlagId2, chrEntityId, value, itemLotId, messageId, timeSeconds`
**Sets Flag**: YES - `SetEventFlagID(eventFlagId, ON)` and optionally `eventFlagId2`
**Trigger**: `CharacterHPValue(chrEntityId) <= 0` then `CharacterDead(chrEntityId)`
**Purpose**: Like 90005860 but with custom message display. Used for special bosses.

### 90005702 - NPC Death Flag Batch Reset
**Parameters**: `chrEntityId, eventFlagId, eventFlagId2, eventFlagId3`
**Sets Flag**: YES - `BatchSetNetworkconnectedEventFlags(eventFlagId2, eventFlagId3, OFF)` then `SetNetworkconnectedEventFlagID(eventFlagId, ON)`
**Trigger**: `!EventFlag(eventFlagId) && CharacterDead(chrEntityId)`
**Purpose**: When NPC dies, clear a range of flags and set the death flag. Used for quest state transitions.

### 90005703 - Boss Phase Transition (HP Threshold)
**Parameters**: `chrEntityId, eventFlagId, eventFlagId2, eventFlagId3, eventFlagId4, eventFlagId5, eventFlagId6, value`
**Sets Flag**: YES - `BatchSetNetworkconnectedEventFlags()` then sets specific flag
**Trigger**: HP threshold or death
**Purpose**: Multi-phase boss logic. Clears phase flag range, sets current phase flag.

### 90005704 - Boss Phase with Animation
**Parameters**: `entityId, eventFlagId, eventFlagId2, eventFlagId3, value`
**Sets Flag**: YES - `BatchSetNetworkconnectedEventFlags()` then sets phase flag
**Trigger**: HP threshold on boss entity
**Purpose**: Phase transition with animation playback.

### 90005707 - Boss Phase Transition Complex
**Parameters**: `chrEntityId, eventFlagId, eventFlagId2, eventFlagId3, eventFlagId4, eventFlagId5, eventFlagId6, value, animationId, eventFlagId7, eventFlagId8`
**Sets Flag**: YES - Multiple phase flags with batch clear
**Trigger**: HP thresholds with animation
**Purpose**: Complex multi-phase boss with animations and multiple phase flags.

---

## Boss Room Entry & Fog Gates

### 9005800 - Boss Fog Gate Entry (Host)
**Parameters**: `eventFlagId, entityId, areaEntityId, eventFlagId2, chrEntityId, actionButtonParameterId, eventFlagId3, areaEntityId2`
**Sets Flag**: YES - `SetNetworkconnectedEventFlagID(eventFlagId2, ON)` (boss start flag)
**Trigger**: Player enters fog gate area and interacts
**Purpose**: Main boss fog gate entry for host. Handles multiplayer notification, boss activation.
- `eventFlagId`: Boss defeated flag
- `eventFlagId2`: Boss battle started flag
- `eventFlagId3`: Optional unlock flag

### 9005801 - Boss Fog Gate Entry (Phantom)
**Parameters**: `eventFlagId, entityId, areaEntityId, eventFlagId2, eventFlagId3, actionButtonParameterId`
**Sets Flag**: YES - `SetEventFlagID(eventFlagId3, ON)` (phantom entered flag)
**Trigger**: Phantom enters fog gate area
**Purpose**: Handles phantom entry to boss room. Disables network sync (client-side).

### 90005830 - Boss Room Speffect Area
**Parameters**: `eventFlagId, areaEntityId`
**Sets Flag**: NO (applies spEffect, no permanent flag)
**Trigger**: Player enters area before boss defeated
**Purpose**: Applies spEffect 4250 when entering boss area pre-defeat. Temporary buff/debuff.

---

## Grace (Bonfire) Templates

### 90005600 - Grace Registration (Simple)
**Parameters**: `eventFlagId, assetEntityId, enemyDeactivationDistance, chrEntityId`
**Sets Flag**: NO (RegisterBonfire doesn't set the flag, player interaction does)
**Trigger**: `RegisterBonfire()` called with eventFlagId
**Purpose**: Registers a simple grace. The grace flag itself is set by the game when player interacts.
- `eventFlagId`: Grace discovered flag
- `enemyDeactivationDistance`: Radius to disable enemies

### 9005810 - Grace Spawn with Boss Trigger
**Parameters**: `eventFlagId, eventFlagId2, chrEntityId, assetEntityId, enemyDeactivationDistance`
**Sets Flag**: NO (RegisterBonfire internal)
**Trigger**: `EventFlag(eventFlagId)` (boss defeated flag)
**Purpose**: Grace appears after boss is defeated. Spawns grace object with SFX when boss flag is set.
- `eventFlagId`: Boss defeated flag (trigger)
- `eventFlagId2`: Grace flag (for RegisterBonfire)

### 9005811 - Grace Fog (Multiplayer)
**Parameters**: `eventFlagId, assetEntityId, sfxId, eventFlagId2`
**Sets Flag**: NO
**Trigger**: Multiplayer state or grace not discovered
**Purpose**: Shows/hides grace fog for multiplayer. Visual only, doesn't set flags.

### 9005812 - Grace Fog (Multiplayer + Invasion)
**Parameters**: `eventFlagId, assetEntityId, sfxId, eventFlagId2, sfxId2`
**Sets Flag**: NO
**Trigger**: Multiplayer/invasion state
**Purpose**: Like 9005811 but with separate invasion SFX.

### 9005813 - Grace Fog (Multiplayer Extended)
**Parameters**: `eventFlagId, assetEntityId, sfxId, eventFlagId2, sfxId2`
**Sets Flag**: NO
**Trigger**: Multiplayer pending states
**Purpose**: Extended multiplayer fog logic with pending states.

### 90005605 - Grace Discovery with Fast Travel Prompt
**Parameters**: `assetEntityId, areaId, blockId, regionId, indexId, initialAreaEntityId, subareaNamePopupMessageId, eventFlagId, eventFlagId2, eventFlagId3, eventFlagId4, messageId, timeSeconds, timeSeconds2`
**Sets Flag**: YES - `SetEventFlagID(eventFlagId, ON)` (internal visual flag), `SetEventFlagID(eventFlagId2, ON)` (dialog result)
**Trigger**: Player interacts with grace action button
**Purpose**: Grace discovery with "Travel to grace?" dialog. Handles warp on acceptance.
- `eventFlagId`: Grace visual SFX flag (not the discovered flag)
- `eventFlagId2/3`: Dialog choice flags (Yes/No)
- `eventFlagId4`: Optional unlock requirement

---

## Item Pickup & Treasure Templates

### 90005555 - Special Treasure Pickup (Gesture)
**Parameters**: `eventFlagId, itemLotId, assetEntityId`
**Sets Flag**: NO (flag set elsewhere after item awarded)
**Trigger**: `ActionButtonInArea(209200, assetEntityId)` (interact with treasure)
**Purpose**: Special treasure pickup with animation (61040) and sound. Awards item lot. Used for special items.

### 90005556 - Time-Gated Treasure (Night Only)
**Parameters**: `assetEntityId, eventFlagId`
**Sets Flag**: NO
**Trigger**: Time of day 20:00-5:59
**Purpose**: Treasure only appears at night. Enables/disables asset treasure based on time.

### 90005560 - Destructible Object Treasure
**Parameters**: `eventFlagId, assetEntityId, value`
**Sets Flag**: YES - `SetNetworkconnectedEventFlagID(eventFlagId, ON)` when destroyed
**Trigger**: `AssetDestroyed(assetEntityId)`
**Purpose**: Breakable pots, crates, etc. Sets flag when destroyed, enables treasure pickup.
- `value`: If 0, adds glowing SFX before destruction

### 90005750 - Treasure Beacon (Conditional)
**Parameters**: `assetEntityId, actionButtonParameterId, itemLotId, eventFlagId, eventFlagId2, eventFlagId3, sfxId`
**Sets Flag**: NO (flag set elsewhere)
**Trigger**: Action button interact, batch flags not all set
**Purpose**: Shows glowing beacon when quest conditions met. Awards item when player interacts.
- Uses `AllBatchEventFlags(eventFlagId, eventFlagId2)` to check range

### 90005632 - Map Fragment Treasure
**Parameters**: `eventFlagId, assetEntityId, itemLotId`
**Sets Flag**: NO (flag set by treasure pickup system)
**Trigger**: Treasure pickup
**Purpose**: Simple treasure that awards item lot. Used for map fragments and similar.

---

## Door & Mechanism Templates

### 90005500 - Lever Door (Two-State)
**Parameters**: `eventFlagId, eventFlagId2, value, assetEntityId, assetEntityId2, objactEventFlag, assetEntityId3, objactEventFlag2, areaEntityId, areaEntityId2, eventFlagId3, eventFlagId4, eventFlagId5`
**Sets Flag**: YES - `SetNetworkconnectedEventFlagID(eventFlagId, ON/OFF)` and `SetNetworkconnectedEventFlagID(eventFlagId3, ON/OFF)`
**Trigger**: `ObjActEventFlag(objactEventFlag)` (lever interaction)
**Purpose**: Complex lever-operated door system. Two assets (open/closed), handles animations.
- `eventFlagId`: Door open flag
- `eventFlagId2`: Door state toggle flag (internal)
- `eventFlagId3`: Animation in progress flag
- `value`: Animation variation (0-10)

### 90005501 - Door State Handler
**Parameters**: `eventFlagId, eventFlagId2, value, assetEntityId, assetEntityId2, assetEntityId3, eventFlagId3`
**Sets Flag**: YES - `SetEventFlagID(eventFlagId2, ON)` and `SetEventFlagID(eventFlagId3, OFF)`
**Trigger**: Asset backread (door in range)
**Purpose**: Handles door animation states based on flag. Plays correct animation when door loads.

### 90005502 - Simple Unlock Door
**Parameters**: `eventFlagId, assetEntityId, areaEntityId`
**Sets Flag**: YES - `SetNetworkconnectedEventFlagID(eventFlagId, ON)`
**Trigger**: Player in area
**Purpose**: Simple door unlock when player enters area. One-way progression gates.

### 90015502 - Simple Unlock Door (Variant)
**Parameters**: `eventFlagId, assetEntityId, areaEntityId`
**Sets Flag**: YES - `SetNetworkconnectedEventFlagID(eventFlagId, ON)`
**Trigger**: Player in area
**Purpose**: Identical to 90005502, different event ID for organization.

### 90005510 - ObjAct Unlock
**Parameters**: `eventFlagId, assetEntityId, objactEventFlag, objactParamId, messageId, value`
**Sets Flag**: YES - `SetNetworkconnectedEventFlagID(eventFlagId, ON)`
**Trigger**: `ObjActEventFlag(objactEventFlag)` (interact)
**Purpose**: Interactable object that sets flag when used. Generic unlock mechanism.

### 90005511 - ObjAct Unlock (No Message)
**Parameters**: `eventFlagId, assetEntityId, objactEventFlag, objactParamId, value`
**Sets Flag**: YES - `SetNetworkconnectedEventFlagID(eventFlagId, ON)`
**Trigger**: `ObjActEventFlag(objactEventFlag)`
**Purpose**: Like 90005510 but without message display.

### 90005512 - Area Unlock
**Parameters**: `eventFlagId, areaEntityId, areaEntityId2`
**Sets Flag**: YES - `SetEventFlagID(eventFlagId, ON)`
**Trigger**: Player enters either area
**Purpose**: Sets flag when player enters one of two areas. Used for progression tracking.

### 90005513 - ObjAct with Animation
**Parameters**: `eventFlagId, assetEntityId, assetEntityId2, objactEventFlag, objactParamId, animationId, animationId2`
**Sets Flag**: YES - `SetNetworkconnectedEventFlagID(eventFlagId, ON)`
**Trigger**: `ObjActEventFlag(objactEventFlag)`
**Purpose**: Interactable that plays animation and sets flag. Used for levers, switches.

### 90005520 - Door Toggle Flag
**Parameters**: `eventFlagId, assetEntityId, eventFlagId2, eventFlagId3`
**Sets Flag**: YES - `SetEventFlagID(eventFlagId2, ON/OFF)` toggles
**Trigger**: Flag changes
**Purpose**: Door state synchronization. When flag changes, updates door open/close state.

### 90005525 - Asset Activation
**Parameters**: `eventFlagId, assetEntityId`
**Sets Flag**: NO
**Trigger**: EventFlag state
**Purpose**: Enables/disables asset based on flag. Used for conditional objects.

### 90005540 - ObjAct Lever with Animation
**Parameters**: `eventFlagId, assetEntityId, assetEntityId2, objactEventFlag, objactParamId, animationId, animationId2`
**Sets Flag**: YES - `SetNetworkconnectedEventFlagID(eventFlagId, ON)`
**Trigger**: `ObjActEventFlag(objactEventFlag)`
**Purpose**: Lever that plays animation on both lever and target asset.

### 90005550 - ObjAct Flag Set
**Parameters**: `eventFlagId, assetEntityId, objactEventFlag`
**Sets Flag**: YES - `SetNetworkconnectedEventFlagID(eventFlagId, ON)`
**Trigger**: `ObjActEventFlag(objactEventFlag)`
**Purpose**: Simple interactable that sets flag. Generic unlock.

### 900055278 - Stake of Marika
**Parameters**: `eventFlagId, assetEntityId, sfxId, actionButtonParameterId, messageId, value, value2, value3`
**Sets Flag**: NO (sets respawn point)
**Trigger**: Action button interact
**Purpose**: Stake of Marika interaction. Sets respawn point, doesn't set permanent flag.

### 90005920 - ObjAct Flag Set (Alt)
**Parameters**: `eventFlagId, assetEntityId, objactEventFlag`
**Sets Flag**: YES - `SetNetworkconnectedEventFlagID(eventFlagId, ON)`
**Trigger**: `ObjActEventFlag(objactEventFlag)`
**Purpose**: Identical to 90005550, different event ID range.

---

## Gesture & Key Item Unlock Templates

### 90005570 - Gesture Unlock (Asset Interaction)
**Parameters**: `eventFlagId, gestureParamId, assetEntityId, value, value2, value3`
**Sets Flag**: YES - `SetEventFlagID(eventFlagId, ON)` after gesture awarded
**Trigger**: Action button interact with asset
**Purpose**: Interacting with asset awards gesture (e.g., Prattling Pate stones).
- `value`: Action button type (0=4200, 1=4300, 2=4250)
- `value2`: SFX type (0-3 for different glow effects)

### 900005571 - Gesture Unlock (Flag Triggered)
**Parameters**: `eventFlagId, gestureParamId, eventFlagId2, value`
**Sets Flag**: YES - `SetEventFlagID(eventFlagId, ON)` after gesture awarded
**Trigger**: `EventFlag(eventFlagId2)`
**Purpose**: Awards gesture when another flag is set. Used for quest rewards.

### 900005580 - Asset Event Flag Sync
**Parameters**: `eventFlagId, assetEntityId, eventFlagId2`
**Sets Flag**: YES - `SetEventFlagID(eventFlagId2, ON)`
**Trigger**: `EventFlag(eventFlagId)`
**Purpose**: When flag 1 sets, set flag 2 and play asset animation. Used for cascading events.

---

## Boss BGM Templates

### 9005822 - Boss BGM (Two-Phase)
**Parameters**: `eventFlagId, bgmBossConvParamId, eventFlagId2, eventFlagId3, eventFlagId4, eventFlagId5, value, value2`
**Sets Flag**: NO (controls BGM, doesn't set flags)
**Trigger**: Boss start flag
**Purpose**: Boss music with phase transition. Start -> HeatUp -> Stop.
- `eventFlagId`: Boss defeated flag (trigger to stop)
- `eventFlagId2`: Boss started flag (trigger to start)
- `eventFlagId5`: Phase 2 flag

### 9005823 - Boss BGM (Three-Phase)
**Parameters**: `eventFlagId, bgmBossConvParamId, eventFlagId2, eventFlagId3, eventFlagId4, eventFlagId5, eventFlagId6, value, value2`
**Sets Flag**: NO
**Trigger**: Boss start and phase flags
**Purpose**: Three-phase boss music. Start -> HeatUp -> HeatUp2 -> Stop.

### 9005824 - Boss BGM (Three-Phase Alt)
**Parameters**: `eventFlagId, bgmBossConvParamId, eventFlagId2, eventFlagId3, eventFlagId4, eventFlagId5, eventFlagId6, value, value2`
**Sets Flag**: NO
**Trigger**: Boss start and phase flags
**Purpose**: Like 9005823 but with slightly different phase logic.

### 90005885 - Boss BGM (Two-Phase Simple)
**Parameters**: `eventFlagId, bgmBossConvParamId, eventFlagId2, eventFlagId3, value, value2`
**Sets Flag**: NO
**Trigger**: Boss flags
**Purpose**: Simpler two-phase music control.

---

## NPC Summon Sign Templates

### 90005780 - Place NPC Summon Sign
**Parameters**: `eventFlagId, eventFlagId2, eventFlagId3, npcEntityId, signType, areaEntityId, eventFlagId4, shouldMultiplayerRestrictionsApply, value`
**Sets Flag**: NO (PlaceNPCSummonSign internal)
**Trigger**: Player near NPC location, conditions met
**Purpose**: Places summon sign for NPC. Used for boss helper NPCs.
- `eventFlagId`: Boss defeated flag (don't place if defeated)
- `eventFlagId2`: Sign active flag (set by PlaceNPCSummonSign)
- `eventFlagId3`: NPC summoned flag
- `eventFlagId4`: Optional requirement flag

### 90005781 - NPC Summon Spawning
**Parameters**: `eventFlagId, eventFlagId2, eventFlagId3, chrEntityId`
**Sets Flag**: NO
**Trigger**: `EventFlag(eventFlagId2)` (summon sign used)
**Purpose**: Spawns NPC when summon sign is used. Enables character and AI.

### 90005782 - NPC Post-Summon Navigation
**Parameters**: `eventFlagId, eventFlagId2, chrEntityId, entityId, areaEntityId, animationId`
**Sets Flag**: NO
**Trigger**: Both flags set (summoned + boss started)
**Purpose**: Directs NPC to navigate to boss fog gate after summoning.

### 90005783 - Send NPC Summon Home (Timed)
**Parameters**: `eventFlagId, eventFlagId2, eventFlagId3, npcEntityId, entityId, areaEntityId, value`
**Sets Flag**: NO
**Trigger**: Player leaves area or NPC out of combat too long
**Purpose**: Despawns NPC summon if conditions met. Prevents NPC from following forever.
- `value`: Distance threshold (1=360m, 2=720m, default=180m)

### 90005785 - Send NPC Summon Home (Distance)
**Parameters**: `eventFlagId, eventFlagId2, eventFlagId3, npcEntityId, entityId, areaEntityId, targetDistance`
**Sets Flag**: NO
**Trigger**: Player too far from NPC or area
**Purpose**: Like 90005783 but with custom distance parameter.

### 90005790 - Place NPC Summon Sign (Timed)
**Parameters**: `eventFlagId, eventFlagId2, eventFlagId3, eventFlagId4, npcEntityId, signType, areaEntityId, areaEntityId2, timeSeconds, eventFlagId5, shouldMultiplayerRestrictionsApply, value`
**Sets Flag**: NO
**Trigger**: Conditions met for time duration
**Purpose**: Places summon sign that requires conditions to be met for a duration before appearing.

### 90005791 - NPC Summon with Character State
**Parameters**: `eventFlagId, eventFlagId2, eventFlagId3, chrEntityId`
**Sets Flag**: NO
**Trigger**: Summon flag
**Purpose**: Spawns NPC with specific character state setup.

### 90005792 - NPC Summon with Item Drop
**Parameters**: `eventFlagId, eventFlagId2, eventFlagId3, chrEntityId, itemLotId, timeSeconds`
**Sets Flag**: NO
**Trigger**: Summon flag
**Purpose**: NPC summon that drops items when boss defeated.

### 90005793 - NPC Summon with Area Check
**Parameters**: `eventFlagId, eventFlagId2, eventFlagId3, npcEntityId, entityId, areaEntityId, value`
**Sets Flag**: NO
**Trigger**: Conditions met
**Purpose**: NPC summon with additional area validation.

---

## Hostile NPC Invasion Templates

### 90005796 - Hostile NPC Invader (Simple)
**Parameters**: `eventFlagId, chrEntityId, bannerType, areaEntityId`
**Sets Flag**: NO
**Trigger**: Player enters area, flag not set
**Purpose**: Spawns hostile NPC invader with banner when player enters area.
- `bannerType`: Invasion banner type (e.g., "Recusant X Invaded!")

### 90005797 - Hostile NPC Invader (SpEffect)
**Parameters**: `eventFlagId, chrEntityId, bannerType, areaEntityId, spEffectId`
**Sets Flag**: NO
**Trigger**: Player enters area
**Purpose**: Like 90005796 but applies spEffect to player on invasion.

### 90005798 - Hostile NPC Invader (Conditional Flag)
**Parameters**: `eventFlagId, chrEntityId, bannerType, areaEntityId, eventFlagId2, spEffectId`
**Sets Flag**: NO
**Trigger**: Player enters area and eventFlagId2 is set
**Purpose**: Invader only spawns if additional flag condition met.

### 90005799 - Hostile NPC Invader (Two Characters)
**Parameters**: `eventFlagId, chrEntityId, bannerType, areaEntityId, eventFlagId2, chrEntityId2, spEffectId`
**Sets Flag**: NO
**Trigger**: Player enters area
**Purpose**: Two-character invasion (e.g., duo boss invaders).

### 90005795 - Invasion Message Prompt
**Parameters**: `eventFlagId, eventFlagId2, eventFlagId3, eventFlagId4, eventFlagId5, messageId, actionButtonParameterId, assetEntityId, sfxId`
**Sets Flag**: NO
**Trigger**: Action button interaction
**Purpose**: Shows message before invasion starts. Used for lore notes before hostile NPC.

---

## World State & Progression Templates

### 90005100 - Bell Bearing Collection (Large Set)
**Parameters**: `eventFlagId, eventFlagId2, assetEntityId, eventFlagId3, thresholdValue, eventFlagId4-eventFlagId13`
**Sets Flag**: Complex - Multiple network flags based on bell bearing collection
**Trigger**: `ObjActEventFlag()` on asset
**Purpose**: Handles bell bearing turn-in. Updates shop inventory flags based on bells collected.
- Checks 10 bell bearing flags (eventFlagId4-13)
- Sets progressive unlock flags

### 90005101 - Bell Bearing Collection (Variant)
**Parameters**: Same as 90005100
**Sets Flag**: Complex - Multiple flags
**Trigger**: ObjAct
**Purpose**: Variant bell bearing handler with different logic flow.

### 90005102 - Bell Bearing Collection (Extended)
**Parameters**: Same as 90005100 + `eventFlagId14`
**Sets Flag**: Complex - Multiple flags
**Trigger**: ObjAct
**Purpose**: Extended bell bearing handler with 11 bell slots.

### 90005110 - Golden Seed Offering
**Parameters**: `eventFlagId, eventFlagId2, assetEntityId, itemLotId, itemId, sfxId, actionButtonParameterId, animationId, value`
**Sets Flag**: YES - Sets upgrade level flags
**Trigger**: Action button interaction with required item
**Purpose**: Flask upgrade at Sites of Grace. Consumes Golden Seeds/Sacred Tears.

### 90005730 - Timed Event Trigger
**Parameters**: `eventFlagId, targetTimeSeconds, eventFlagId2, eventFlagId3, eventFlagId4, eventFlagId5`
**Sets Flag**: YES - `SetNetworkconnectedEventFlagID(eventFlagId, ON)`
**Trigger**: Timer expires or flags change
**Purpose**: Sets flag after time delay. Used for timed world events.

### 90005732 - Area Trigger (Two Areas OR)
**Parameters**: `eventFlagId, areaEntityId, areaEntityId2`
**Sets Flag**: YES - `SetNetworkconnectedEventFlagID(eventFlagId, ON)`
**Trigger**: Player enters either area
**Purpose**: Sets flag when player enters one of two areas.

### 90005733 - Immediate Flag Set
**Parameters**: `eventFlagId`
**Sets Flag**: YES - `SetNetworkconnectedEventFlagID(eventFlagId, ON)`
**Trigger**: Immediate
**Purpose**: Sets flag immediately when event runs. Used for initialization.

### 90005734 - Area Trigger with Counter
**Parameters**: `eventFlagId, eventFlagId2, areaEntityId, areaEntityId2, eventFlagId3, value`
**Sets Flag**: YES - Multiple flags
**Trigger**: Player enters area, increments counter
**Purpose**: Counts area entries, sets flags based on count.

---

## Quest & NPC State Templates

### 90005767 - Quest Completion (Two NPCs)
**Parameters**: `eventFlagId, eventFlagId2, eventFlagId3, chrEntityId, eventFlagId4, chrEntityId2, eventFlagId5`
**Sets Flag**: YES - `SetNetworkconnectedEventFlagID(eventFlagId, ON)`
**Trigger**: Complex flag conditions and character states
**Purpose**: Quest completion when two NPCs reach certain states.

### 90005768 - Quest Item Award (Conditional)
**Parameters**: `eventFlagId, itemLotId, eventFlagId2, itemLotId2, eventFlagId3, eventFlagId4`
**Sets Flag**: NO (awards items based on flags)
**Trigger**: Flag changes
**Purpose**: Awards different item lots based on which flags are set.

### 90005769 - NPC Death Quest Update
**Parameters**: `chrEntityId, eventFlagId, chrEntityId2, eventFlagId2, eventFlagId3, eventFlagId4, eventFlagId5`
**Sets Flag**: YES - Multiple quest flags
**Trigger**: NPC death
**Purpose**: Updates quest flags when NPC dies. Used for quest branching.

### 90005773 - Immediate Flag Set (Default Mode)
**Parameters**: `eventFlagId`
**Sets Flag**: YES - `SetNetworkconnectedEventFlagID(eventFlagId, ON)`
**Trigger**: Immediate
**Purpose**: Sets flag immediately, Default mode (runs every load).

### 90005774 - Item Lot Flag Award
**Parameters**: `eventFlagId, itemLotId, eventFlagId2`
**Sets Flag**: NO
**Trigger**: `EventFlag(eventFlagId2)`
**Purpose**: Awards item lot when flag is set. Used for quest rewards.

### 90005776 - Quest Item Award (Two Flags)
**Parameters**: `eventFlagId, eventFlagId2, itemLotId`
**Sets Flag**: NO
**Trigger**: Both flags set
**Purpose**: Awards item when two conditions met.

### 90005777 - NPC Quest Flag Update
**Parameters**: `chrEntityId, eventFlagId, eventFlagId2, eventFlagId3`
**Sets Flag**: YES - `BatchSetNetworkconnectedEventFlags()` then specific flag
**Trigger**: Character state change
**Purpose**: Updates NPC quest state flags.

### 90005778 - Multi-Flag Quest Update
**Parameters**: `eventFlagId, eventFlagId2, eventFlagId3, entityId`
**Sets Flag**: YES - `BatchSetNetworkconnectedEventFlags()` then specific flag
**Trigger**: Flag conditions met
**Purpose**: Quest state machine with flag range clearing.

### 90005779 - NPC Animation Quest Trigger
**Parameters**: `chrEntityId, eventFlagId, animationId, eventFlagId2, eventFlagId3`
**Sets Flag**: YES - `SetEventFlagID(eventFlagId, ON)`
**Trigger**: Character animation state
**Purpose**: Sets quest flag when NPC plays specific animation.

---

## Special Mechanics Templates

### 90005605 - (Detailed Above) Grace Discovery
### 90005600 - (Detailed Above) Grace Registration
### 90005555 - (Detailed Above) Special Treasure

### 90005645 - Map Discovery
**Parameters**: `eventFlagId, eventFlagId2, eventFlagId3, assetEntityId, initialAreaEntityId, areaId, blockId, regionId, indexId`
**Sets Flag**: YES - `SetEventFlagID(eventFlagId, ON)`
**Trigger**: Player approaches map asset
**Purpose**: Unlocks map region when player discovers map asset.

### 90005646 - Map Discovery (Default)
**Parameters**: Same as 90005645
**Sets Flag**: YES - `SetEventFlagID(eventFlagId, ON)`
**Trigger**: Player approaches map asset
**Purpose**: Like 90005645 but Default mode (persistent).

### 90005650 - Prayer Book / Spell Unlock
**Parameters**: `eventFlagId, assetEntityId, assetEntityId2, objactEventFlag, objactParamId`
**Sets Flag**: YES - `SetNetworkconnectedEventFlagID(eventFlagId, ON)`
**Trigger**: ObjAct interaction
**Purpose**: Unlocks spells/incantations at asset (e.g., prayer book turn-in).

### 90005651 - Spell Unlock (Simple)
**Parameters**: `eventFlagId, entityId`
**Sets Flag**: YES - `SetNetworkconnectedEventFlagID(eventFlagId, ON)`
**Trigger**: Entity interaction
**Purpose**: Simple spell unlock.

### 90005652 - Conditional Unlock
**Parameters**: `eventFlagId, assetEntityId, eventFlagId2`
**Sets Flag**: YES - `SetNetworkconnectedEventFlagID(eventFlagId, ON)`
**Trigger**: `EventFlag(eventFlagId2)` changes
**Purpose**: Unlocks when another flag changes state.

### 90005775 - Map Point Discovery
**Parameters**: `worldMapPointParamId, eventFlagId, distance`
**Sets Flag**: YES - `SetEventFlagID(eventFlagId, ON)`
**Trigger**: Player within distance of map point
**Purpose**: Unlocks map marker when player approaches location.

### 90005638 - Boss Arena Door
**Parameters**: `eventFlagId, assetEntityId, assetEntityId2`
**Sets Flag**: YES - `SetNetworkconnectedEventFlagID(eventFlagId, ON)`
**Trigger**: Asset activated
**Purpose**: Opens boss arena door when activated.

### 90005639 - Timed Door Unlock
**Parameters**: `eventFlagId, assetEntityId, assetEntityId2, assetEntityId3, timeSeconds`
**Sets Flag**: YES - `SetNetworkconnectedEventFlagID(eventFlagId, ON)`
**Trigger**: Timer after asset activation
**Purpose**: Door opens after delay. Used for dramatic entrances.

### 90005640 - Boss Door Post-Defeat
**Parameters**: `eventFlagId, assetEntityId`
**Sets Flag**: NO
**Trigger**: Boss defeated flag
**Purpose**: Opens arena exit door after boss is defeated.

---

## Character Behavior Templates (Non-Flag Setting)

Most of these don't set permanent flags but control NPC AI, animations, and behavior:

- **90005200-90005271**: NPC idle animations and patrol behaviors
- **90005400-90005491**: NPC special effects, transformations, behavior triggers
- **90005660-90005695**: Character AI commands, patrol routes, behavior triggers
- **90005700-90005729**: Boss phase transitions, HP triggers, AI commands
- **90005735-90005779**: Complex NPC behavior, combat triggers, quest interactions
- **90005870-90005882**: NPC invasions, summons, and display names

These are important for understanding NPC behavior but don't directly create permanent progression flags.

---

## Flag Arithmetic Patterns

### X0_4 + Offset Pattern
Many events use parameters like `X0_4 + 0`, `X0_4 + 1`, etc. to calculate flag IDs:
- Used in batch flag operations
- Allows single event to manage flag ranges
- Common in boss phase transitions and quest states

Example from 90005700:
```javascript
BatchSetNetworkconnectedEventFlags(eventFlagId4, eventFlagId5, OFF);
SetNetworkconnectedEventFlagID(eventFlagId, ON);
```

This clears a range `eventFlagId4` to `eventFlagId5`, then sets `eventFlagId`.

### Batch Flag Operations
- `BatchSetEventFlags(startFlag, endFlag, ON/OFF)` - Sets range of flags
- `BatchSetNetworkconnectedEventFlags(startFlag, endFlag, ON/OFF)` - Network-synced version
- `AllBatchEventFlags(startFlag, endFlag)` - Checks if all flags in range are set

Used for:
- Quest state machines (clear old states, set new state)
- Boss phase management
- Bell bearing collection tracking

---

## Key Observations

### Flag Setting Functions
1. **SetEventFlagID(flag, ON/OFF)** - Local flag, not network synced
2. **SetNetworkconnectedEventFlagID(flag, ON/OFF)** - Synced across multiplayer
3. **BatchSetEventFlags(start, end, ON/OFF)** - Set range of local flags
4. **BatchSetNetworkconnectedEventFlags(start, end, ON/OFF)** - Set range of synced flags

### Permanent vs Temporary
**Permanent** (sets flags we care about):
- Boss defeats (90005860, 90005861, 9005840)
- Item pickups (90005555, 90005560, treasure systems)
- Grace discovery (90005600, 9005810, 90005605)
- Door unlocks (90005510, 90005511, 90005550)
- NPC deaths (90005300, 90005301)
- Quest progression (90005767, 90005768, 90005769)

**Temporary** (transient state):
- BGM control (9005822, 9005823, 9005824)
- Fog gates (9005811, 9005812, 9005813)
- Animation triggers (90005200-271 series)
- NPC summon signs (90005780, 90005781)

### Event Flag ID Ranges
Based on observed usage:
- Boss defeated flags: Often in x00 format (e.g., 1000800 for Margit)
- Boss start flags: Often x01 (e.g., 1000801)
- Grace flags: Often in x0000 format
- Quest state flags: Often sequential ranges (x00, x01, x02, etc.)
- Item pickup flags: Often match itemLot IDs

---

## Usage for Auto-Detection

For the save editor's auto-detection system, focus on these high-confidence templates:

### Tier 1 - Direct Flag Setters (Highest Confidence)
- **90005860/90005861**: Boss defeats - flag set on death + banner
- **9005840**: Boss defeats - Demigod banner variant
- **90005300/90005301**: Enemy deaths - flag set on death
- **SetNetworkconnectedEventFlagID** calls - Always permanent

### Tier 2 - Corroborated Events
- **90005510/90005511/90005550**: Door unlocks - flag + ObjAct
- **90005560**: Destructible treasure - flag + AssetDestroyed
- **RegisterBonfire calls**: Grace discovery - flag registered
- **90005605**: Grace discovery - complex but detectable

### Tier 3 - Context-Dependent
- **90005500**: Lever doors - multiple flags, complex
- **90005780**: NPC summons - flag checked, not always set
- **90005767/768/769**: Quest events - batch flag operations

### What to Index
For each event instance in map files:
1. Event template ID (90005860, etc.)
2. All eventFlagId parameters
3. Related entity IDs (chr, asset)
4. Flag setting pattern (SetEventFlagID vs SetNetworkconnectedEventFlagID)

This allows correlation:
- If flag X is set AND corresponds to event using template 90005860
- And event has chrEntityId matching known boss
- Then flag X = boss defeat flag (HIGH confidence)

---

## Template Signatures for Pattern Matching

When parsing map event files, look for these signatures:

### Boss Defeat Pattern
```
InitializeEvent(slot, 90005860, flagX, flagY, bossEntityId, bannerType, itemLotId, delay)
+ flagX not set in save
+ bossEntityId is known boss character
= flagX is boss defeat flag
```

### Grace Discovery Pattern
```
InitializeEvent(slot, 9005810, bossFlagX, graceFlagY, bossEntityId, graceAsset, distance)
+ bossFlagX is boss defeat flag (from above)
+ graceFlagY not set in save
= graceFlagY is grace discovery flag (appears after boss)
```

### Item Pickup Pattern
```
InitializeEvent(slot, 90005560, flagX, assetEntityId, 0)
+ flagX not set in save
+ assetEntityId is destructible object
= flagX is item pickup flag
```

This catalog provides the foundation for building a comprehensive event flag database by parsing actual map files and correlating them with these templates.
