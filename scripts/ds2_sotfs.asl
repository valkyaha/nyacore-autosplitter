// Dark Souls II: Scholar of the First Sin - ASL Autosplitter
// Uses kill counter approach - splits when boss kill count goes from 0 to >0
//
// Memory patterns (defined in companion TOML or resolved at runtime):
// game_manager_imp: "48 8b 35 ? ? ? ? 48 8b e9 48 85 f6" (RIP relative, offset 3, len 7)
// Boss counters path: game_manager_imp -> [0x0, 0x70, 0x28, 0x20, 0x8, boss_offset]

state("DarkSoulsII.exe") {
    // Boss kill counters - read as int at specific offsets from boss_counters base
    // The "game_manager_imp" pattern needs to be resolved first, then pointer chain followed

    // Main game bosses
    int lastGiant      : "game_manager_imp", 0x0, 0x70, 0x28, 0x20, 0x8, 0x00;
    int pursuer        : "game_manager_imp", 0x0, 0x70, 0x28, 0x20, 0x8, 0x04;
    int oldDragonslayer: "game_manager_imp", 0x0, 0x70, 0x28, 0x20, 0x8, 0x08;
    int flexileSentry  : "game_manager_imp", 0x0, 0x70, 0x28, 0x20, 0x8, 0x0C;
    int ruinSentinels  : "game_manager_imp", 0x0, 0x70, 0x28, 0x20, 0x8, 0x10;
    int lostSinner     : "game_manager_imp", 0x0, 0x70, 0x28, 0x20, 0x8, 0x14;
    int belfryGargoyles: "game_manager_imp", 0x0, 0x70, 0x28, 0x20, 0x8, 0x18;
    int skeletonLords  : "game_manager_imp", 0x0, 0x70, 0x28, 0x20, 0x8, 0x1C;
    int dragonrider    : "game_manager_imp", 0x0, 0x70, 0x28, 0x20, 0x8, 0x20;
    int executionerChariot: "game_manager_imp", 0x0, 0x70, 0x28, 0x20, 0x8, 0x24;
    int covetousDemon  : "game_manager_imp", 0x0, 0x70, 0x28, 0x20, 0x8, 0x28;
    int mytha          : "game_manager_imp", 0x0, 0x70, 0x28, 0x20, 0x8, 0x2C;
    int smelterDemon   : "game_manager_imp", 0x0, 0x70, 0x28, 0x20, 0x8, 0x30;
    int oldIronKing    : "game_manager_imp", 0x0, 0x70, 0x28, 0x20, 0x8, 0x34;
    int scorpionessNajka: "game_manager_imp", 0x0, 0x70, 0x28, 0x20, 0x8, 0x38;
    int royalRatAuthority: "game_manager_imp", 0x0, 0x70, 0x28, 0x20, 0x8, 0x3C;
    int prowlingMagus  : "game_manager_imp", 0x0, 0x70, 0x28, 0x20, 0x8, 0x40;
    int dukesDearFreja : "game_manager_imp", 0x0, 0x70, 0x28, 0x20, 0x8, 0x44;
    int royalRatVanguard: "game_manager_imp", 0x0, 0x70, 0x28, 0x20, 0x8, 0x48;
    int theRotten      : "game_manager_imp", 0x0, 0x70, 0x28, 0x20, 0x8, 0x4C;
    int dragonriderDuo : "game_manager_imp", 0x0, 0x70, 0x28, 0x20, 0x8, 0x50;
    int lookingGlassKnight: "game_manager_imp", 0x0, 0x70, 0x28, 0x20, 0x8, 0x54;
    int demonOfSong    : "game_manager_imp", 0x0, 0x70, 0x28, 0x20, 0x8, 0x58;
    int velstadt       : "game_manager_imp", 0x0, 0x70, 0x28, 0x20, 0x8, 0x5C;
    int vendrick       : "game_manager_imp", 0x0, 0x70, 0x28, 0x20, 0x8, 0x60;
    int guardianDragon : "game_manager_imp", 0x0, 0x70, 0x28, 0x20, 0x8, 0x64;
    int ancientDragon  : "game_manager_imp", 0x0, 0x70, 0x28, 0x20, 0x8, 0x68;
    int giantLord      : "game_manager_imp", 0x0, 0x70, 0x28, 0x20, 0x8, 0x6C;
    int throneWatcherDefender: "game_manager_imp", 0x0, 0x70, 0x28, 0x20, 0x8, 0x70;
    int nashandra      : "game_manager_imp", 0x0, 0x70, 0x28, 0x20, 0x8, 0x74;
    int aldia          : "game_manager_imp", 0x0, 0x70, 0x28, 0x20, 0x8, 0x78;

    // DLC bosses
    int elana          : "game_manager_imp", 0x0, 0x70, 0x28, 0x20, 0x8, 0x7C;
    int sinh           : "game_manager_imp", 0x0, 0x70, 0x28, 0x20, 0x8, 0x80;
    int afflictedGraverobber: "game_manager_imp", 0x0, 0x70, 0x28, 0x20, 0x8, 0x84;
    int fumeKnight     : "game_manager_imp", 0x0, 0x70, 0x28, 0x20, 0x8, 0x88;
    int sirAlonne      : "game_manager_imp", 0x0, 0x70, 0x28, 0x20, 0x8, 0x8C;
    int blueSmelterDemon: "game_manager_imp", 0x0, 0x70, 0x28, 0x20, 0x8, 0x90;
    int aava           : "game_manager_imp", 0x0, 0x70, 0x28, 0x20, 0x8, 0x94;
    int burntIvoryKing : "game_manager_imp", 0x0, 0x70, 0x28, 0x20, 0x8, 0x98;
    int ludZallen      : "game_manager_imp", 0x0, 0x70, 0x28, 0x20, 0x8, 0x9C;
}

startup {
    // Initialize settings for which bosses to split on
    // Users can enable/disable individual splits
}

init {
    // Called when game process is detected
    // Reset any tracking state
}

split {
    // Split when any boss kill count goes from 0 to > 0
    // This is the core autosplit logic for DS2

    // Main game bosses
    if (current.lastGiant > 0 && old.lastGiant == 0) { return true; }
    if (current.pursuer > 0 && old.pursuer == 0) { return true; }
    if (current.oldDragonslayer > 0 && old.oldDragonslayer == 0) { return true; }
    if (current.flexileSentry > 0 && old.flexileSentry == 0) { return true; }
    if (current.ruinSentinels > 0 && old.ruinSentinels == 0) { return true; }
    if (current.lostSinner > 0 && old.lostSinner == 0) { return true; }
    if (current.belfryGargoyles > 0 && old.belfryGargoyles == 0) { return true; }
    if (current.skeletonLords > 0 && old.skeletonLords == 0) { return true; }
    if (current.dragonrider > 0 && old.dragonrider == 0) { return true; }
    if (current.executionerChariot > 0 && old.executionerChariot == 0) { return true; }
    if (current.covetousDemon > 0 && old.covetousDemon == 0) { return true; }
    if (current.mytha > 0 && old.mytha == 0) { return true; }
    if (current.smelterDemon > 0 && old.smelterDemon == 0) { return true; }
    if (current.oldIronKing > 0 && old.oldIronKing == 0) { return true; }
    if (current.scorpionessNajka > 0 && old.scorpionessNajka == 0) { return true; }
    if (current.royalRatAuthority > 0 && old.royalRatAuthority == 0) { return true; }
    if (current.prowlingMagus > 0 && old.prowlingMagus == 0) { return true; }
    if (current.dukesDearFreja > 0 && old.dukesDearFreja == 0) { return true; }
    if (current.royalRatVanguard > 0 && old.royalRatVanguard == 0) { return true; }
    if (current.theRotten > 0 && old.theRotten == 0) { return true; }
    if (current.dragonriderDuo > 0 && old.dragonriderDuo == 0) { return true; }
    if (current.lookingGlassKnight > 0 && old.lookingGlassKnight == 0) { return true; }
    if (current.demonOfSong > 0 && old.demonOfSong == 0) { return true; }
    if (current.velstadt > 0 && old.velstadt == 0) { return true; }
    if (current.vendrick > 0 && old.vendrick == 0) { return true; }
    if (current.guardianDragon > 0 && old.guardianDragon == 0) { return true; }
    if (current.ancientDragon > 0 && old.ancientDragon == 0) { return true; }
    if (current.giantLord > 0 && old.giantLord == 0) { return true; }
    if (current.throneWatcherDefender > 0 && old.throneWatcherDefender == 0) { return true; }
    if (current.nashandra > 0 && old.nashandra == 0) { return true; }
    if (current.aldia > 0 && old.aldia == 0) { return true; }

    // DLC bosses
    if (current.elana > 0 && old.elana == 0) { return true; }
    if (current.sinh > 0 && old.sinh == 0) { return true; }
    if (current.afflictedGraverobber > 0 && old.afflictedGraverobber == 0) { return true; }
    if (current.fumeKnight > 0 && old.fumeKnight == 0) { return true; }
    if (current.sirAlonne > 0 && old.sirAlonne == 0) { return true; }
    if (current.blueSmelterDemon > 0 && old.blueSmelterDemon == 0) { return true; }
    if (current.aava > 0 && old.aava == 0) { return true; }
    if (current.burntIvoryKing > 0 && old.burntIvoryKing == 0) { return true; }
    if (current.ludZallen > 0 && old.ludZallen == 0) { return true; }

    return false;
}

reset {
    // Optional: Reset when all boss counters are 0 (new game)
    return false;
}

isLoading {
    // DS2 doesn't need loading detection for autosplitter
    return false;
}
