//! Port of src/data_recall.cpp — immutable recall description tables.

pub const RECALL_DESCRIPTION_ATTACK_TYPE: [&str; 25] = [
    "do something undefined",
    "attack",
    "weaken",
    "confuse",
    "terrify",
    "shoot flames",
    "shoot acid",
    "freeze",
    "shoot lightning",
    "corrode",
    "blind",
    "paralyse",
    "steal money",
    "steal things",
    "poison",
    "reduce dexterity",
    "reduce constitution",
    "drain intelligence",
    "drain wisdom",
    "lower experience",
    "call for help",
    "disenchant",
    "eat your food",
    "absorb light",
    "absorb charges",
];

pub const RECALL_DESCRIPTION_ATTACK_METHOD: [&str; 20] = [
    "make an undefined advance",
    "hit",
    "bite",
    "claw",
    "sting",
    "touch",
    "kick",
    "gaze",
    "breathe",
    "spit",
    "wail",
    "embrace",
    "crawl on you",
    "release spores",
    "beg",
    "slime you",
    "crush",
    "trample",
    "drool",
    "insult",
];

pub const RECALL_DESCRIPTION_HOW_MUCH: [&str; 8] = [
    " not at all",
    " a bit",
    "",
    " quite",
    " very",
    " most",
    " highly",
    " extremely",
];

pub const RECALL_DESCRIPTION_MOVE: [&str; 6] = [
    "move invisibly",
    "open doors",
    "pass through walls",
    "kill weaker creatures",
    "pick up objects",
    "breed explosively",
];

pub const RECALL_DESCRIPTION_SPELL: [&str; 15] = [
    "teleport short distances",
    "teleport long distances",
    "teleport its prey",
    "cause light wounds",
    "cause serious wounds",
    "paralyse its prey",
    "induce blindness",
    "confuse",
    "terrify",
    "summon a monster",
    "summon the undead",
    "slow its prey",
    "drain mana",
    "unknown 1",
    "unknown 2",
];

pub const RECALL_DESCRIPTION_BREATH: [&str; 5] =
    ["lightning", "poison gases", "acid", "frost", "fire"];

pub const RECALL_DESCRIPTION_WEAKNESS: [&str; 6] = [
    "frost",
    "fire",
    "poison",
    "acid",
    "bright light",
    "rock remover",
];
