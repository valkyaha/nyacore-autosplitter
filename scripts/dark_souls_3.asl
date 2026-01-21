// Dark Souls III - ASL Autosplitter
// Uses area-based event flag system with complex pointer resolution
//
// Memory patterns (defined in companion TOML or resolved at runtime):
// sprj_event_flag_man: "48 c7 05 ? ? ? ? 00 00 00 00 48 8b 7c 24 38 c7 46 54 ff ff ff ff 48 83 c4 20 5e c3"
//   (RIP relative, offset 3, len 11)
// field_area: "4c 8b 3d ? ? ? ? 8b 45 87 83 f8 ff 74 69 48 8d 4d 8f 48 89 4d 9f 89 45 8f 48 8d 55 8f 49 8b 4f 10"
//   (RIP relative, offset 3, len 7)

state("DarkSoulsIII.exe") {
    // Boss defeat event flags
    // These flags are set when the boss is killed

    // Iudex Gundyr
    bool iudexGundyr : "sprj_event_flag_man", 13000050;

    // Vordt of the Boreal Valley
    bool vordt : "sprj_event_flag_man", 13000800;

    // Curse-Rotted Greatwood
    bool curseRottedGreatwood : "sprj_event_flag_man", 13000830;

    // Crystal Sage
    bool crystalSage : "sprj_event_flag_man", 13100800;

    // Deacons of the Deep
    bool deaconsOfTheDeep : "sprj_event_flag_man", 13100850;

    // Abyss Watchers
    bool abyssWatchers : "sprj_event_flag_man", 13300850;

    // High Lord Wolnir
    bool wolnir : "sprj_event_flag_man", 13800800;

    // Old Demon King
    bool oldDemonKing : "sprj_event_flag_man", 13800830;

    // Pontiff Sulyvahn
    bool pontiffSulyvahn : "sprj_event_flag_man", 13700850;

    // Yhorm the Giant
    bool yhorm : "sprj_event_flag_man", 13900800;

    // Aldrich, Devourer of Gods
    bool aldrich : "sprj_event_flag_man", 13700800;

    // Dancer of the Boreal Valley
    bool dancer : "sprj_event_flag_man", 13000890;

    // Dragonslayer Armour
    bool dragonslayerArmour : "sprj_event_flag_man", 13010800;

    // Oceiros, the Consumed King
    bool oceiros : "sprj_event_flag_man", 13000900;

    // Champion Gundyr
    bool championGundyr : "sprj_event_flag_man", 14000800;

    // Lothric, Younger Prince
    bool lothric : "sprj_event_flag_man", 13010850;

    // Ancient Wyvern (optional)
    bool ancientWyvern : "sprj_event_flag_man", 13200800;

    // Nameless King (optional)
    bool namelessKing : "sprj_event_flag_man", 13200850;

    // Soul of Cinder
    bool soulOfCinder : "sprj_event_flag_man", 14100800;

    // === DLC 1: Ashes of Ariandel ===

    // Champion's Gravetender & Gravetender Greatwolf
    bool gravetender : "sprj_event_flag_man", 14500800;

    // Sister Friede
    bool friede : "sprj_event_flag_man", 14500860;

    // === DLC 2: The Ringed City ===

    // Demon Prince
    bool demonPrince : "sprj_event_flag_man", 15000800;

    // Halflight, Spear of the Church
    bool halflight : "sprj_event_flag_man", 15100800;

    // Darkeater Midir
    bool midir : "sprj_event_flag_man", 15100850;

    // Slave Knight Gael
    bool gael : "sprj_event_flag_man", 15110800;
}

startup {
    // Initialize settings for which bosses to split on
}

init {
    // Called when game process is detected
}

split {
    // Split when any boss defeat flag goes from false to true

    // Main game bosses (in typical route order)
    if (current.iudexGundyr && !old.iudexGundyr) { return true; }
    if (current.vordt && !old.vordt) { return true; }
    if (current.curseRottedGreatwood && !old.curseRottedGreatwood) { return true; }
    if (current.crystalSage && !old.crystalSage) { return true; }
    if (current.deaconsOfTheDeep && !old.deaconsOfTheDeep) { return true; }
    if (current.abyssWatchers && !old.abyssWatchers) { return true; }
    if (current.wolnir && !old.wolnir) { return true; }
    if (current.oldDemonKing && !old.oldDemonKing) { return true; }
    if (current.pontiffSulyvahn && !old.pontiffSulyvahn) { return true; }
    if (current.yhorm && !old.yhorm) { return true; }
    if (current.aldrich && !old.aldrich) { return true; }
    if (current.dancer && !old.dancer) { return true; }
    if (current.dragonslayerArmour && !old.dragonslayerArmour) { return true; }
    if (current.oceiros && !old.oceiros) { return true; }
    if (current.championGundyr && !old.championGundyr) { return true; }
    if (current.lothric && !old.lothric) { return true; }
    if (current.ancientWyvern && !old.ancientWyvern) { return true; }
    if (current.namelessKing && !old.namelessKing) { return true; }
    if (current.soulOfCinder && !old.soulOfCinder) { return true; }

    // DLC bosses
    if (current.gravetender && !old.gravetender) { return true; }
    if (current.friede && !old.friede) { return true; }
    if (current.demonPrince && !old.demonPrince) { return true; }
    if (current.halflight && !old.halflight) { return true; }
    if (current.midir && !old.midir) { return true; }
    if (current.gael && !old.gael) { return true; }

    return false;
}

reset {
    return false;
}

isLoading {
    return false;
}
