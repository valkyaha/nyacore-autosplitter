// Sekiro: Shadows Die Twice - ASL Autosplitter
// Uses event flag system similar to Dark Souls 3 but with different offsets
//
// Memory patterns (defined in companion TOML or resolved at runtime):
// event_flag_man: "48 8b 0d ? ? ? ? 48 89 5c 24 50 48 89 6c 24 58 48 89 74 24 60" (RIP relative, offset 3, len 7)
// field_area: "48 8b 0d ? ? ? ? 48 85 c9 74 26 44 8b 41 28 48 8d 54 24 40" (RIP relative, offset 3, len 7)
//
// Note: Sekiro uses 0x18 offset (vs DS3's 0x10) and 0xb0 stride (vs DS3's 0x70)

state("sekiro.exe") {
    // Boss defeat event flags
    // These flags are set when the boss is killed/deathblown

    // === Ashina Outskirts (Early) ===
    // General Naomori Kawarada (mini-boss)
    bool kawarada : "event_flag_man", 11105500;

    // Gyoubu Masataka Oniwa
    bool gyoubu : "event_flag_man", 11105520;

    // === Hirata Estate ===
    // Shinobi Hunter Enshin of Misen (mini-boss)
    bool enshin : "event_flag_man", 11005210;

    // Juzou the Drunkard (mini-boss)
    bool juzou : "event_flag_man", 11005200;

    // Lady Butterfly
    bool ladyButterfly : "event_flag_man", 11005900;

    // === Ashina Castle ===
    // General Tenzen Yamauchi (mini-boss)
    bool tenzen : "event_flag_man", 11105250;

    // Blazing Bull
    bool blazingBull : "event_flag_man", 11105530;

    // Genichiro Ashina
    bool genichiro : "event_flag_man", 11105810;

    // === Ashina Depths ===
    // Snake Eyes Shirafuji (mini-boss)
    bool shirafuji : "event_flag_man", 11305200;

    // Snake Eyes Shirahagi (mini-boss)
    bool shirahagi : "event_flag_man", 11705200;

    // === Sunken Valley ===
    // Long-arm Centipede Sen'un (mini-boss)
    bool senun : "event_flag_man", 11505200;

    // Guardian Ape
    bool guardianApe : "event_flag_man", 11505800;

    // === Ashina Depths (continued) ===
    // Guardian Ape (Headless)
    bool guardianApeHeadless : "event_flag_man", 11305800;

    // === Senpou Temple ===
    // Long-arm Centipede Giraffe (mini-boss)
    bool giraffe : "event_flag_man", 12005200;

    // Armored Warrior
    bool armoredWarrior : "event_flag_man", 12005500;

    // Folding Screen Monkeys
    bool foldingScreenMonkeys : "event_flag_man", 12005800;

    // === Fountainhead Palace ===
    // Corrupted Monk (True)
    bool corruptedMonkTrue : "event_flag_man", 15005800;

    // Divine Dragon
    bool divineDragon : "event_flag_man", 15005810;

    // === Ashina Castle (Post-Divine Dragon) ===
    // Owl (Father) - if memory bell chosen
    bool owlFather : "event_flag_man", 11005820;

    // Great Shinobi Owl
    bool owl : "event_flag_man", 11105801;

    // === Ending Bosses ===
    // Emma, the Gentle Blade (Shura ending)
    bool emma : "event_flag_man", 11105812;

    // Isshin Ashina (Shura ending)
    bool isshinAshina : "event_flag_man", 11105813;

    // Isshin, the Sword Saint
    bool isshinSwordSaint : "event_flag_man", 11105850;

    // === Mibu Village ===
    // Corrupted Monk (Illusion)
    bool corruptedMonkIllusion : "event_flag_man", 11705800;

    // O'Rin of the Water (mini-boss)
    bool orin : "event_flag_man", 11705210;

    // === Optional Bosses ===
    // Demon of Hatred
    bool demonOfHatred : "event_flag_man", 11105821;
}

startup {
    // Initialize settings for which bosses to split on
}

init {
    // Called when game process is detected
}

split {
    // Split when any boss defeat flag goes from false to true

    // Main progression bosses
    if (current.gyoubu && !old.gyoubu) { return true; }
    if (current.ladyButterfly && !old.ladyButterfly) { return true; }
    if (current.blazingBull && !old.blazingBull) { return true; }
    if (current.genichiro && !old.genichiro) { return true; }
    if (current.guardianApe && !old.guardianApe) { return true; }
    if (current.guardianApeHeadless && !old.guardianApeHeadless) { return true; }
    if (current.armoredWarrior && !old.armoredWarrior) { return true; }
    if (current.foldingScreenMonkeys && !old.foldingScreenMonkeys) { return true; }
    if (current.corruptedMonkIllusion && !old.corruptedMonkIllusion) { return true; }
    if (current.corruptedMonkTrue && !old.corruptedMonkTrue) { return true; }
    if (current.divineDragon && !old.divineDragon) { return true; }
    if (current.owl && !old.owl) { return true; }
    if (current.owlFather && !old.owlFather) { return true; }
    if (current.isshinSwordSaint && !old.isshinSwordSaint) { return true; }

    // Shura ending bosses
    if (current.emma && !old.emma) { return true; }
    if (current.isshinAshina && !old.isshinAshina) { return true; }

    // Optional bosses
    if (current.demonOfHatred && !old.demonOfHatred) { return true; }

    // Mini-bosses (optional splits)
    if (current.kawarada && !old.kawarada) { return true; }
    if (current.enshin && !old.enshin) { return true; }
    if (current.juzou && !old.juzou) { return true; }
    if (current.tenzen && !old.tenzen) { return true; }
    if (current.shirafuji && !old.shirafuji) { return true; }
    if (current.shirahagi && !old.shirahagi) { return true; }
    if (current.senun && !old.senun) { return true; }
    if (current.giraffe && !old.giraffe) { return true; }
    if (current.orin && !old.orin) { return true; }

    return false;
}

reset {
    return false;
}

isLoading {
    return false;
}
