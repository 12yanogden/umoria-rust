/**
 * Locations + combat catalog fragment (phase_3.2.2).
 * Merged by phase_3.4 into docs-catalog.ts — no article prose here.
 */
import type { DocsCatalogEntry } from "../docs-catalog-types.ts";

export const dungeonCombatCatalogFragment: DocsCatalogEntry[] = [
  {
    slug: "locations/city",
    title: "The City",
    summary: "Town level: starting area, stores, zero XP for town kills.",
    section: "locations",
    order: 10,
    sources: [
      {
        label: "mmspoilers · Dungeon › City",
        href: "https://beej.us/moria/mmspoilers/dungeon.html#city"
      }
    ],
    relatedSlugs: ["locations/stores", "locations/underground"]
  },
  {
    slug: "locations/stores",
    title: "Stores",
    summary: "Six store types, shopkeeper stats table (Max$, markup, haggle%, race, insults).",
    section: "locations",
    order: 20,
    sources: [
      {
        label: "mmspoilers · Dungeon › Stores",
        href: "https://beej.us/moria/mmspoilers/dungeon.html#stores"
      },
      {
        label: "Umoria shopkeepers (optional cross-check)",
        href: "src/data_store_owners.rs"
      }
    ],
    relatedSlugs: ["locations/haggling", "locations/city", "character/attributes"],
    dependsOnSlugs: ["locations/city"]
  },
  {
    slug: "locations/haggling",
    title: "Haggling",
    summary: "Offer/final price formulas, unidentified item pricing, race/CHA adjustments, insult rules.",
    section: "locations",
    order: 30,
    sources: [
      {
        label: "mmspoilers · Dungeon › Haggling",
        href: "https://beej.us/moria/mmspoilers/dungeon.html#haggling"
      }
    ],
    relatedSlugs: ["locations/stores", "character/attributes"],
    dependsOnSlugs: ["locations/stores"]
  },
  {
    slug: "locations/underground",
    title: "The Underground",
    summary: "Level size, stair one-way generation, no return to prior level.",
    section: "locations",
    order: 40,
    sources: [
      {
        label: "mmspoilers · Dungeon › Underground",
        href: "https://beej.us/moria/mmspoilers/dungeon.html#underground"
      }
    ],
    relatedSlugs: ["locations/traps", "locations/city"],
    dependsOnSlugs: ["locations/city"]
  },
  {
    slug: "locations/traps",
    title: "Traps",
    summary: "Trap types, XP values, effects (pits, gas, runes, dart stat drain, etc.).",
    section: "locations",
    order: 50,
    sources: [
      {
        label: "mmspoilers · Dungeon › Traps",
        href: "https://beej.us/moria/mmspoilers/dungeon.html#traps"
      }
    ],
    relatedSlugs: ["locations/underground", "combat/monsters"],
    dependsOnSlugs: ["locations/underground"]
  },
  {
    slug: "combat/monsters",
    title: "Monster descriptions",
    summary: "Monster flag/ability matrix and stat blocks — large table; stub links to mmspoilers source.",
    section: "combat",
    order: 10,
    sources: [
      {
        label: "mmspoilers · Combat › Monster Descriptions",
        href: "https://beej.us/moria/mmspoilers/combat.html#mdescriptions"
      }
    ],
    relatedSlugs: ["combat/monster-attacks", "combat/damage"]
  },
  {
    slug: "combat/monster-attacks",
    title: "Monster attacks",
    summary: "Breath/spell/special attack types and frequencies from combat spoilers.",
    section: "combat",
    order: 20,
    sources: [
      {
        label: "mmspoilers · Combat › Monster Attacks",
        href: "https://beej.us/moria/mmspoilers/combat.html#mattacks"
      }
    ],
    relatedSlugs: ["combat/monsters", "combat/damage"],
    dependsOnSlugs: ["combat/monsters"]
  },
  {
    slug: "combat/hit-probability",
    title: "Hit probability",
    summary: "To-hit calculation, skill bonuses, visibility and terrain modifiers.",
    section: "combat",
    order: 30,
    sources: [
      {
        label: "mmspoilers · Combat › Hit probability",
        href: "https://beej.us/moria/mmspoilers/combat.html#hitprob"
      }
    ],
    relatedSlugs: ["character/attributes", "combat/armor-class", "combat/damage", "items/weapons"],
    dependsOnSlugs: ["character/attributes"]
  },
  {
    slug: "combat/damage",
    title: "Damage calculation",
    summary: "Player and monster damage formulas, criticals, slays/resists overview.",
    section: "combat",
    order: 40,
    sources: [
      {
        label: "mmspoilers · Combat › Damage calculation",
        href: "https://beej.us/moria/mmspoilers/combat.html#damagecalc"
      }
    ],
    relatedSlugs: ["combat/monster-attacks", "items/weapons"],
    dependsOnSlugs: ["combat/hit-probability"]
  },
  {
    slug: "combat/bashing",
    title: "Bashing",
    summary: "Non-weapon bash damage, weight/strength interaction, store-sold bash weapons note.",
    section: "combat",
    order: 50,
    sources: [
      {
        label: "mmspoilers · Combat › Bashing",
        href: "https://beej.us/moria/mmspoilers/combat.html#bashing"
      }
    ],
    relatedSlugs: ["combat/damage", "character/attributes"],
    dependsOnSlugs: ["combat/damage"]
  },
  {
    slug: "combat/armor-class",
    title: "AC calculation",
    summary: "AC sources, armor vs dex, shield and magical AC stacking rules.",
    section: "combat",
    order: 60,
    sources: [
      {
        label: "mmspoilers · Combat › AC calculation",
        href: "https://beej.us/moria/mmspoilers/combat.html#accalc"
      }
    ],
    relatedSlugs: ["items/armor", "character/attributes", "combat/hit-probability"],
    dependsOnSlugs: ["character/attributes", "combat/damage"]
  }
];
