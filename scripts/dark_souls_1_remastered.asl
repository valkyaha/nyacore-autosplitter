// Dark Souls Remastered - ASL Autosplitter
// Uses event flag system - boss flags are 8-digit numbers
//
// Memory patterns (defined in companion TOML or resolved at runtime):
// event_flags: "48 8B 0D ? ? ? ? 99 33 C2 45 33 C0 2B C2 8D 50 F6" (RIP relative, offset 3, len 7)
// Event flags are read using group/area/section/number calculation

state("DarkSoulsRemastered.exe") {
    // Boss defeat event flags
    // Format: Group(1) Area(3) Section(1) Number(3)

    // Asylum Demon
    bool asylumDemon : "event_flags", 11010000;

    // Taurus Demon
    bool taurusDemon : "event_flags", 11010010;

    // Bell Gargoyles
    bool bellGargoyles : "event_flags", 11010020;

    // Capra Demon
    bool capraDemon : "event_flags", 11010050;

    // Gaping Dragon
    bool gapingDragon : "event_flags", 11010060;

    // Quelaag
    bool quelaag : "event_flags", 11410900;

    // Iron Golem
    bool ironGolem : "event_flags", 11510900;

    // Ornstein & Smough
    bool ornsteinSmough : "event_flags", 11510950;

    // Pinwheel
    bool pinwheel : "event_flags", 11310900;

    // Nito
    bool nito : "event_flags", 11310901;

    // Seath the Scaleless
    bool seath : "event_flags", 11700900;

    // Four Kings
    bool fourKings : "event_flags", 11600900;

    // Bed of Chaos
    bool bedOfChaos : "event_flags", 11410410;

    // Sif, the Great Grey Wolf
    bool sif : "event_flags", 11200900;

    // Moonlight Butterfly
    bool moonlightButterfly : "event_flags", 11200901;

    // Sanctuary Guardian
    bool sanctuaryGuardian : "event_flags", 11210900;

    // Knight Artorias
    bool artorias : "event_flags", 11210001;

    // Manus, Father of the Abyss
    bool manus : "event_flags", 11210002;

    // Black Dragon Kalameet
    bool kalameet : "event_flags", 11210003;

    // Priscilla (optional)
    bool priscilla : "event_flags", 11100900;

    // Ceaseless Discharge
    bool ceaselessDischarge : "event_flags", 11410411;

    // Demon Firesage
    bool demonFiresage : "event_flags", 11410890;

    // Centipede Demon
    bool centipedeDemon : "event_flags", 11410850;

    // Gwyn, Lord of Cinder
    bool gwyn : "event_flags", 11800100;

    // Stray Demon
    bool strayDemon : "event_flags", 11810900;

    // Dark Sun Gwyndolin (optional)
    bool gwyndolin : "event_flags", 11510901;
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
    // Split when any boss defeat flag goes from false to true

    // Main bosses
    if (current.asylumDemon && !old.asylumDemon) { return true; }
    if (current.taurusDemon && !old.taurusDemon) { return true; }
    if (current.bellGargoyles && !old.bellGargoyles) { return true; }
    if (current.capraDemon && !old.capraDemon) { return true; }
    if (current.gapingDragon && !old.gapingDragon) { return true; }
    if (current.quelaag && !old.quelaag) { return true; }
    if (current.ironGolem && !old.ironGolem) { return true; }
    if (current.ornsteinSmough && !old.ornsteinSmough) { return true; }
    if (current.pinwheel && !old.pinwheel) { return true; }
    if (current.nito && !old.nito) { return true; }
    if (current.seath && !old.seath) { return true; }
    if (current.fourKings && !old.fourKings) { return true; }
    if (current.bedOfChaos && !old.bedOfChaos) { return true; }
    if (current.sif && !old.sif) { return true; }
    if (current.moonlightButterfly && !old.moonlightButterfly) { return true; }

    // DLC bosses
    if (current.sanctuaryGuardian && !old.sanctuaryGuardian) { return true; }
    if (current.artorias && !old.artorias) { return true; }
    if (current.manus && !old.manus) { return true; }
    if (current.kalameet && !old.kalameet) { return true; }

    // Optional bosses
    if (current.priscilla && !old.priscilla) { return true; }
    if (current.ceaselessDischarge && !old.ceaselessDischarge) { return true; }
    if (current.demonFiresage && !old.demonFiresage) { return true; }
    if (current.centipedeDemon && !old.centipedeDemon) { return true; }
    if (current.gwyn && !old.gwyn) { return true; }
    if (current.strayDemon && !old.strayDemon) { return true; }
    if (current.gwyndolin && !old.gwyndolin) { return true; }

    return false;
}

reset {
    // Optional: Reset when starting a new game
    return false;
}

isLoading {
    return false;
}
