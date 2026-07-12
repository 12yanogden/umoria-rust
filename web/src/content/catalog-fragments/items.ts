/**
 * Items nav section catalog fragment (phase_3.2.3).
 * Beej items.txt + weapnarm.txt + mmspoilers items.html.
 */
import type { DocsCatalogEntry } from "../docs-catalog-types.ts";

export const itemsCatalogFragment: DocsCatalogEntry[] = [
  {
    slug: "items/overview",
    title: "Items overview",
    summary: "Column legend ($$, Wt, AC, Dam, Feet, etc.), {magik}/damned rules, large vs small items, shop-only note.",
    section: "items",
    order: 10,
    sources: [
      {
        label: "items.txt · intro & column legend",
        href: "https://beej.us/moria/items.txt"
      },
      {
        label: "mmspoilers · Items intro",
        href: "https://beej.us/moria/mmspoilers/items.html"
      }
    ],
    relatedSlugs: ["items/weapons", "items/armor", "dungeon/stores"]
  },
  {
    slug: "items/weapons",
    title: "Weapons & missiles",
    summary: "Swords, hafted, polearms, bows, missiles tables from items.txt; mmspoilers weapons section.",
    section: "items",
    order: 20,
    sources: [
      {
        label: "items.txt · Swords/Hafted/Polearms/Bows/Missiles",
        href: "https://beej.us/moria/items.txt"
      },
      {
        label: "mmspoilers · Weapons",
        href: "https://beej.us/moria/mmspoilers/items.html#weapons"
      }
    ],
    relatedSlugs: ["items/special-properties", "combat/damage", "items/weapon-artifacts"],
    dependsOnSlugs: ["items/overview"]
  },
  {
    slug: "items/armor",
    title: "Armor & worn gear",
    summary: "Soft/hard armor, shields, footwear, headgear, misc armor (cloaks/gloves); mmspoilers armor.",
    section: "items",
    order: 30,
    sources: [
      {
        label: "items.txt · Soft/Hard/Shields/Footwear/Headgear/Misc armor",
        href: "https://beej.us/moria/items.txt"
      },
      {
        label: "mmspoilers · Armor",
        href: "https://beej.us/moria/mmspoilers/items.html#armor"
      }
    ],
    relatedSlugs: ["items/special-properties", "combat/armor-class", "items/armor-artifacts"],
    dependsOnSlugs: ["items/overview"]
  },
  {
    slug: "items/special-properties",
    title: "Weapon & armour specials",
    summary: "Ego prefixes/suffixes: (HA), (DF), slays, resist gear, helm/glove/boot/cloak effects.",
    section: "items",
    order: 40,
    sources: [
      {
        label: "weapnarm.txt (full file)",
        href: "https://beej.us/moria/weapnarm.txt"
      }
    ],
    relatedSlugs: ["items/weapons", "items/armor", "items/rings"],
    dependsOnSlugs: ["items/weapons", "items/armor"]
  },
  {
    slug: "items/weapon-artifacts",
    title: "Weapon artifacts",
    summary: "Named artifact weapons and powers from mmspoilers (not duplicated in items.txt tables).",
    section: "items",
    order: 50,
    sources: [
      {
        label: "mmspoilers · Weapon Artifacts",
        href: "https://beej.us/moria/mmspoilers/items.html#weaponartifacts"
      }
    ],
    relatedSlugs: ["items/weapons", "items/special-properties"],
    dependsOnSlugs: ["items/weapons"]
  },
  {
    slug: "items/armor-artifacts",
    title: "Armor artifacts",
    summary: "Named artifact armor set pieces and powers.",
    section: "items",
    order: 60,
    sources: [
      {
        label: "mmspoilers · Armor Artifacts",
        href: "https://beej.us/moria/mmspoilers/items.html#armorartifacts"
      }
    ],
    relatedSlugs: ["items/armor", "items/special-properties"],
    dependsOnSlugs: ["items/armor"]
  },
  {
    slug: "items/rings",
    title: "Rings",
    summary: "Ring types, base costs, depths; mmspoilers rings + items.txt Rings table.",
    section: "items",
    order: 70,
    sources: [
      {
        label: "items.txt · Rings",
        href: "https://beej.us/moria/items.txt"
      },
      {
        label: "mmspoilers · Rings",
        href: "https://beej.us/moria/mmspoilers/items.html#rings"
      }
    ],
    relatedSlugs: ["items/amulets", "items/special-properties"],
    dependsOnSlugs: ["items/overview"]
  },
  {
    slug: "items/amulets",
    title: "Amulets",
    summary: "Amulet types, DOOM/Magi notes; mmspoilers amulets section.",
    section: "items",
    order: 80,
    sources: [
      {
        label: "items.txt · Amulets",
        href: "https://beej.us/moria/items.txt"
      },
      {
        label: "mmspoilers · Amulets",
        href: "https://beej.us/moria/mmspoilers/items.html#amulets"
      }
    ],
    relatedSlugs: ["items/rings", "character/attributes"],
    dependsOnSlugs: ["items/overview"]
  },
  {
    slug: "items/scrolls",
    title: "Scrolls",
    summary: "Scroll list by depth; identify/enchant/genocide tiers.",
    section: "items",
    order: 90,
    sources: [
      {
        label: "items.txt · Scrolls",
        href: "https://beej.us/moria/items.txt"
      },
      {
        label: "mmspoilers · Scrolls",
        href: "https://beej.us/moria/mmspoilers/items.html#scrolls"
      }
    ],
    relatedSlugs: ["items/overview", "spells/system"],
    dependsOnSlugs: ["items/overview"]
  },
  {
    slug: "items/books",
    title: "Spell books",
    summary: "Eight mage/priest books (cost/depth); ties to spell learning.",
    section: "items",
    order: 100,
    sources: [
      {
        label: "items.txt · Books",
        href: "https://beej.us/moria/items.txt"
      },
      {
        label: "mmspoilers · Spell system (book table)",
        href: "https://beej.us/moria/mmspoilers/spells.html#spellsystem"
      }
    ],
    relatedSlugs: ["spells/system", "spells/mage", "spells/priest"],
    dependsOnSlugs: ["items/overview"]
  },
  {
    slug: "items/wands",
    title: "Wands",
    summary: "Wand types, levels, costs; device use level implications.",
    section: "items",
    order: 110,
    sources: [
      {
        label: "items.txt · Wands",
        href: "https://beej.us/moria/items.txt"
      },
      {
        label: "mmspoilers · Wands",
        href: "https://beej.us/moria/mmspoilers/items.html#wands"
      }
    ],
    relatedSlugs: ["items/staves", "character/attributes"],
    dependsOnSlugs: ["items/overview"]
  },
  {
    slug: "items/staves",
    title: "Staves",
    summary: "Staff types, charges, effects by depth.",
    section: "items",
    order: 120,
    sources: [
      {
        label: "items.txt · Staffs",
        href: "https://beej.us/moria/items.txt"
      },
      {
        label: "mmspoilers · Staves",
        href: "https://beej.us/moria/mmspoilers/items.html#staves"
      }
    ],
    relatedSlugs: ["items/wands", "spells/mage"],
    dependsOnSlugs: ["items/overview"]
  },
  {
    slug: "items/potions",
    title: "Potions",
    summary: "Potion list, restore/gain effects, shop vs dungeon depths.",
    section: "items",
    order: 130,
    sources: [
      {
        label: "items.txt · Potions",
        href: "https://beej.us/moria/items.txt"
      },
      {
        label: "mmspoilers · Potions",
        href: "https://beej.us/moria/mmspoilers/items.html#potions"
      }
    ],
    relatedSlugs: ["items/food", "character/attributes"],
    dependsOnSlugs: ["items/overview"]
  },
  {
    slug: "items/food",
    title: "Food, mushrooms & molds",
    summary: "Rations, waybread, mushrooms/molds with food values and stat effects.",
    section: "items",
    order: 140,
    sources: [
      {
        label: "items.txt · Normal Food + Mushrooms and Molds",
        href: "https://beej.us/moria/items.txt"
      },
      {
        label: "mmspoilers · Food",
        href: "https://beej.us/moria/mmspoilers/items.html#food"
      }
    ],
    relatedSlugs: ["items/potions", "dungeon/stores"],
    dependsOnSlugs: ["items/overview"]
  },
  {
    slug: "items/diggers-and-misc",
    title: "Diggers, light, chests & miscellany",
    summary: "Digging tools, lights, chests, skeletons/flotsam; shop-only consumables table tail.",
    section: "items",
    order: 150,
    sources: [
      {
        label: "items.txt · Miscellaneous + Shop Items",
        href: "https://beej.us/moria/items.txt"
      },
      {
        label: "mmspoilers · Diggers",
        href: "https://beej.us/moria/mmspoilers/items.html#diggers"
      }
    ],
    relatedSlugs: ["items/overview", "dungeon/stores", "wizard/items"],
    dependsOnSlugs: ["items/overview"]
  },
  {
    slug: "items/gems",
    title: "Gems",
    summary: "Gem types, values, selling strategy (mmspoilers-only detail).",
    section: "items",
    order: 160,
    sources: [
      {
        label: "mmspoilers · Gems",
        href: "https://beej.us/moria/mmspoilers/items.html#gems"
      }
    ],
    relatedSlugs: ["dungeon/stores", "items/overview"],
    dependsOnSlugs: ["items/overview"]
  }
];
