// Elden Ring - ASL Autosplitter
// Uses VirtualMemoryFlag with tree-based structure
//
// Memory patterns (defined in companion TOML or resolved at runtime):
// virtual_memory_flag: "44 89 7c 24 28 4c 8b 25 ? ? ? ? 4d 85 e4" (RIP relative, offset 8, len 7)
// Flags are read using a binary tree traversal algorithm

state("eldenring.exe") {
    // Boss defeat event flags
    // Major remembrance bosses and key progression bosses

    // === Limgrave ===
    // Margit, the Fell Omen
    bool margit : "virtual_memory_flag", 10000800;

    // Godrick the Grafted
    bool godrick : "virtual_memory_flag", 10000850;

    // === Liurnia ===
    // Red Wolf of Radagon
    bool redWolf : "virtual_memory_flag", 14000800;

    // Rennala, Queen of the Full Moon
    bool rennala : "virtual_memory_flag", 14000850;

    // Royal Knight Loretta (Caria Manor)
    bool loretta : "virtual_memory_flag", 11050800;

    // === Caelid ===
    // Starscourge Radahn
    bool radahn : "virtual_memory_flag", 30030800;

    // === Altus Plateau ===
    // Godfrey, First Elden Lord (Golden Shade)
    bool godfreyShade : "virtual_memory_flag", 11000800;

    // === Mt. Gelmir ===
    // Rykard, Lord of Blasphemy
    bool rykard : "virtual_memory_flag", 16000800;

    // === Leyndell, Royal Capital ===
    // Morgott, the Omen King
    bool morgott : "virtual_memory_flag", 11000850;

    // === Mountaintops of the Giants ===
    // Fire Giant
    bool fireGiant : "virtual_memory_flag", 30110800;

    // === Crumbling Farum Azula ===
    // Godskin Duo
    bool godskinDuo : "virtual_memory_flag", 13000800;

    // Maliketh, the Black Blade
    bool maliketh : "virtual_memory_flag", 13000850;

    // Dragonlord Placidusax (optional)
    bool placidusax : "virtual_memory_flag", 13000830;

    // === Leyndell, Ashen Capital ===
    // Godfrey, First Elden Lord / Hoarah Loux
    bool godfrey : "virtual_memory_flag", 11050850;

    // === Elden Throne ===
    // Radagon of the Golden Order / Elden Beast
    bool radagonEldenBeast : "virtual_memory_flag", 19000800;

    // === Optional Major Bosses ===
    // Mohg, Lord of Blood
    bool mohg : "virtual_memory_flag", 12050800;

    // Malenia, Blade of Miquella
    bool malenia : "virtual_memory_flag", 15000800;

    // Lichdragon Fortissax
    bool fortissax : "virtual_memory_flag", 12020800;

    // Dragonkin Soldier of Nokstella
    bool dragonkinNokstella : "virtual_memory_flag", 12010800;

    // Astel, Naturalborn of the Void
    bool astel : "virtual_memory_flag", 12010850;

    // Regal Ancestor Spirit
    bool regalAncestorSpirit : "virtual_memory_flag", 12090800;

    // === DLC: Shadow of the Erdtree ===
    // (Flag IDs TBD when confirmed)
}

startup {
    // Initialize settings for which bosses to split on
}

init {
    // Called when game process is detected
}

split {
    // Split when any boss defeat flag goes from false to true

    // Required progression bosses
    if (current.margit && !old.margit) { return true; }
    if (current.godrick && !old.godrick) { return true; }
    if (current.redWolf && !old.redWolf) { return true; }
    if (current.rennala && !old.rennala) { return true; }
    if (current.loretta && !old.loretta) { return true; }
    if (current.radahn && !old.radahn) { return true; }
    if (current.godfreyShade && !old.godfreyShade) { return true; }
    if (current.rykard && !old.rykard) { return true; }
    if (current.morgott && !old.morgott) { return true; }
    if (current.fireGiant && !old.fireGiant) { return true; }
    if (current.godskinDuo && !old.godskinDuo) { return true; }
    if (current.maliketh && !old.maliketh) { return true; }
    if (current.placidusax && !old.placidusax) { return true; }
    if (current.godfrey && !old.godfrey) { return true; }
    if (current.radagonEldenBeast && !old.radagonEldenBeast) { return true; }

    // Optional major bosses
    if (current.mohg && !old.mohg) { return true; }
    if (current.malenia && !old.malenia) { return true; }
    if (current.fortissax && !old.fortissax) { return true; }
    if (current.dragonkinNokstella && !old.dragonkinNokstella) { return true; }
    if (current.astel && !old.astel) { return true; }
    if (current.regalAncestorSpirit && !old.regalAncestorSpirit) { return true; }

    return false;
}

reset {
    return false;
}

isLoading {
    return false;
}
