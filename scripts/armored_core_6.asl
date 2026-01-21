// Armored Core 6: Fires of Rubicon - ASL Autosplitter
// Uses CSEventFlagMan with tree-based structure (same as Elden Ring)
//
// Memory patterns (defined in companion TOML or resolved at runtime):
// cs_event_flag_man: "48 8b 35 ? ? ? ? 83 f8 ff 0f 44 c1" (RIP relative, offset 3, len 7)

state("armoredcore6.exe") {
    // Mission completion / Boss defeat event flags
    // AC6 tracks mission completion rather than individual boss kills

    // === Chapter 1 ===
    // Mission: ILLEGAL ENTRY
    bool illegalEntry : "cs_event_flag_man", 30100100;

    // Mission: DESTROY THE TRANSPORT HELICOPTERS
    bool destroyTransportHelicopters : "cs_event_flag_man", 30100200;

    // Mission: DESTROY THE TESTER AC
    bool destroyTesterAC : "cs_event_flag_man", 30100300;

    // Mission: ATTACK THE DAM COMPLEX
    bool attackDamComplex : "cs_event_flag_man", 30100400;

    // Mission: DESTROY THE WEAPONIZED MINING SHIP
    bool destroyMiningShip : "cs_event_flag_man", 30100500;

    // BOSS: AH12: HC HELICOPTER (Strider)
    bool strider : "cs_event_flag_man", 30100510;

    // Mission: OPERATION WALLCLIMBER
    bool operationWallclimber : "cs_event_flag_man", 30100600;

    // BOSS: JUGGERNAUT
    bool juggernaut : "cs_event_flag_man", 30100610;

    // Mission: RETRIEVE COMBAT LOGS
    bool retrieveCombatLogs : "cs_event_flag_man", 30100700;

    // === Chapter 2 ===
    // Mission: INFILTRATE GRID 086
    bool infiltrateGrid086 : "cs_event_flag_man", 30200100;

    // BOSS: BALTEUS
    bool balteus : "cs_event_flag_man", 30200200;

    // Mission: ATTACK THE WATCHPOINT
    bool attackWatchpoint : "cs_event_flag_man", 30200300;

    // Mission: DESTROY THE SPECIAL FORCES CRAFT
    bool destroySpecialForces : "cs_event_flag_man", 30200400;

    // BOSS: SEA SPIDER
    bool seaSpider : "cs_event_flag_man", 30200500;

    // === Chapter 3 ===
    // Mission: ATTACK THE OLD SPACEPORT
    bool attackOldSpaceport : "cs_event_flag_man", 30300100;

    // BOSS: SMART CLEANER
    bool smartCleaner : "cs_event_flag_man", 30300110;

    // Mission: ELIMINATE THE ENFORCEMENT SQUADS
    bool eliminateEnforcement : "cs_event_flag_man", 30300200;

    // BOSS: EC-0804 SMART CLEANER
    bool smartCleanerBoss : "cs_event_flag_man", 30300300;

    // Mission: SURVEY THE UNINHABITED FLOATING CITY
    bool surveyFloatingCity : "cs_event_flag_man", 30300400;

    // BOSS: IA-02: ICE WORM
    bool iceWorm : "cs_event_flag_man", 30300500;

    // === Chapter 4 ===
    // BOSS: IB-01: CEL 240
    bool cel240 : "cs_event_flag_man", 30400100;

    // Mission: UNKNOWN TERRITORY SURVEY
    bool unknownTerritorySurvey : "cs_event_flag_man", 30400200;

    // BOSS: AYRE
    bool ayre : "cs_event_flag_man", 30400300;

    // === Chapter 5 ===
    // BOSS: IBIS SERIES CEL 240
    bool ibisCel240 : "cs_event_flag_man", 30500100;

    // Mission: BREACH THE KARMAN LINE
    bool breachKarmanLine : "cs_event_flag_man", 30500200;

    // BOSS: IA-13: SEA SPIDER
    bool seaSpiderBoss : "cs_event_flag_man", 30500300;

    // BOSS: HANDLER WALTER
    bool handlerWalter : "cs_event_flag_man", 30500400;

    // === Final Boss (varies by route) ===
    // BOSS: All Mind (Liberator of Rubicon ending)
    bool allMind : "cs_event_flag_man", 30500500;

    // BOSS: Iguazu (Fires of Raven ending)
    bool iguazu : "cs_event_flag_man", 30500510;
}

startup {
    // Initialize settings for which missions/bosses to split on
}

init {
    // Called when game process is detected
}

split {
    // Split when any mission/boss completion flag goes from false to true

    // Chapter 1
    if (current.illegalEntry && !old.illegalEntry) { return true; }
    if (current.destroyTransportHelicopters && !old.destroyTransportHelicopters) { return true; }
    if (current.destroyTesterAC && !old.destroyTesterAC) { return true; }
    if (current.attackDamComplex && !old.attackDamComplex) { return true; }
    if (current.destroyMiningShip && !old.destroyMiningShip) { return true; }
    if (current.strider && !old.strider) { return true; }
    if (current.operationWallclimber && !old.operationWallclimber) { return true; }
    if (current.juggernaut && !old.juggernaut) { return true; }
    if (current.retrieveCombatLogs && !old.retrieveCombatLogs) { return true; }

    // Chapter 2
    if (current.infiltrateGrid086 && !old.infiltrateGrid086) { return true; }
    if (current.balteus && !old.balteus) { return true; }
    if (current.attackWatchpoint && !old.attackWatchpoint) { return true; }
    if (current.destroySpecialForces && !old.destroySpecialForces) { return true; }
    if (current.seaSpider && !old.seaSpider) { return true; }

    // Chapter 3
    if (current.attackOldSpaceport && !old.attackOldSpaceport) { return true; }
    if (current.smartCleaner && !old.smartCleaner) { return true; }
    if (current.eliminateEnforcement && !old.eliminateEnforcement) { return true; }
    if (current.smartCleanerBoss && !old.smartCleanerBoss) { return true; }
    if (current.surveyFloatingCity && !old.surveyFloatingCity) { return true; }
    if (current.iceWorm && !old.iceWorm) { return true; }

    // Chapter 4
    if (current.cel240 && !old.cel240) { return true; }
    if (current.unknownTerritorySurvey && !old.unknownTerritorySurvey) { return true; }
    if (current.ayre && !old.ayre) { return true; }

    // Chapter 5
    if (current.ibisCel240 && !old.ibisCel240) { return true; }
    if (current.breachKarmanLine && !old.breachKarmanLine) { return true; }
    if (current.seaSpiderBoss && !old.seaSpiderBoss) { return true; }
    if (current.handlerWalter && !old.handlerWalter) { return true; }

    // Final bosses
    if (current.allMind && !old.allMind) { return true; }
    if (current.iguazu && !old.iguazu) { return true; }

    return false;
}

reset {
    return false;
}

isLoading {
    return false;
}
