/**
 * Character + social class catalog fragment (phase_3.2.1).
 * Merged by phase_3.4 into docs-catalog.ts — no article prose here.
 */
import type { DocsCatalogEntry } from "../docs-catalog-types.ts";

export const characterCatalogFragment: DocsCatalogEntry[] = [
  {
    slug: "character/attributes",
    title: "Attributes",
    summary: "STR/DEX/INT/WIS/CON/CHA tables, carry capacity, attacks/round, device use from INT.",
    section: "character",
    order: 10,
    sources: [
      {
        label: "mmspoilers · Character › Attributes",
        href: "https://beej.us/moria/mmspoilers/character.html#attributes"
      }
    ],
    relatedSlugs: ["character/classes", "combat/hit-probability", "combat/armor-class", "spells/mana"]
  },
  {
    slug: "character/races",
    title: "Races",
    summary: "Eight races: stat mods, class bitmap, skills, age/height/weight, infravision, hit die.",
    section: "character",
    order: 20,
    sources: [
      {
        label: "mmspoilers · Character › Races",
        href: "https://beej.us/moria/mmspoilers/character.html#races"
      },
      {
        label: "Umoria data (races)",
        href: "src/data_player.rs"
      }
    ],
    relatedSlugs: ["character/classes", "character/experience"]
  },
  {
    slug: "character/classes",
    title: "Classes",
    summary: "Six classes: stat mods, skills, per-level skill growth, spell access overview.",
    section: "character",
    order: 30,
    sources: [
      {
        label: "mmspoilers · Character › Classes",
        href: "https://beej.us/moria/mmspoilers/character.html#classes"
      },
      {
        label: "Umoria data (classes)",
        href: "src/data_player.rs"
      }
    ],
    relatedSlugs: ["character/races", "character/experience", "spells/system"],
    dependsOnSlugs: ["character/races"]
  },
  {
    slug: "character/experience",
    title: "Experience & titles",
    summary: "Level XP table, race/class penalties, XP gain sources, level titles by class.",
    section: "character",
    order: 40,
    sources: [
      {
        label: "mmspoilers · Character › Experience",
        href: "https://beej.us/moria/mmspoilers/character.html#experience"
      }
    ],
    relatedSlugs: ["character/classes", "character/races"],
    dependsOnSlugs: ["character/classes"]
  },
  {
    slug: "character/social-class",
    title: "Social class overview",
    summary: "What social class affects (starting gold, flavor); pointer to race-group tables.",
    section: "character",
    order: 50,
    sources: [
      {
        label: "classes.txt · Introduction",
        href: "https://beej.us/moria/classes.txt"
      }
    ],
    relatedSlugs: [
      "character/social-class-humanoids",
      "character/social-class-elves",
      "character/social-class-smallfolk",
      "character/social-class-dwarves-trolls"
    ]
  },
  {
    slug: "character/social-class-humanoids",
    title: "Social class — Human, Half-Elf, Half-Orc",
    summary: "Parent occupation bases, birth/family adjustments, half-elf elvish parent mods.",
    section: "character",
    order: 60,
    sources: [
      {
        label: "classes.txt · §1 Humans, Half-Elves, Half-Orcs",
        href: "https://beej.us/moria/classes.txt"
      }
    ],
    relatedSlugs: ["character/races", "character/social-class"],
    dependsOnSlugs: ["character/social-class"]
  },
  {
    slug: "character/social-class-elves",
    title: "Social class — Elves",
    summary: "Elf parent occupation and birth circumstance tables.",
    section: "character",
    order: 70,
    sources: [
      {
        label: "classes.txt · §2 Elves",
        href: "https://beej.us/moria/classes.txt"
      }
    ],
    relatedSlugs: ["character/races", "character/social-class"],
    dependsOnSlugs: ["character/social-class"]
  },
  {
    slug: "character/social-class-smallfolk",
    title: "Social class — Halflings & Gnomes",
    summary: "Halfling and gnome social-class tables (§3–4).",
    section: "character",
    order: 80,
    sources: [
      {
        label: "classes.txt · §3 Halflings, §4 Gnomes",
        href: "https://beej.us/moria/classes.txt"
      }
    ],
    relatedSlugs: ["character/races", "character/social-class"],
    dependsOnSlugs: ["character/social-class"]
  },
  {
    slug: "character/social-class-dwarves-trolls",
    title: "Social class — Dwarves & Half-Trolls",
    summary: "Dwarf and half-troll occupation tables and troll parent race mods.",
    section: "character",
    order: 90,
    sources: [
      {
        label: "classes.txt · §5 Dwarves, §6 Half-Trolls",
        href: "https://beej.us/moria/classes.txt"
      }
    ],
    relatedSlugs: ["character/races", "character/social-class"],
    dependsOnSlugs: ["character/social-class"]
  }
];
