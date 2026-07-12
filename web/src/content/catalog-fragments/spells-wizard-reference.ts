/**
 * Spells + wizard + reference catalog fragment (phase_3.2.4).
 * Merged by phase_3.4 into docs-catalog.ts — no article prose here.
 */
import type { DocsCatalogEntry } from "../docs-catalog-types.ts";

export const spellsWizardReferenceCatalogFragment: DocsCatalogEntry[] = [
  {
    slug: "spells/system",
    title: "Spell system",
    summary:
      "Who casts what (mage vs priest books), spell attributes (level/mana/fail/XP), book requirements for mages.",
    section: "spells",
    order: 10,
    sources: [
      {
        label: "spells.txt · §1 intro",
        href: "https://beej.us/moria/spells.txt"
      },
      {
        label: "mmspoilers · Spell System",
        href: "https://beej.us/moria/mmspoilers/spells.html#spellsystem"
      }
    ],
    relatedSlugs: ["spells/mana", "spells/failure", "character/classes", "items/books"],
    dependsOnSlugs: ["character/classes"]
  },
  {
    slug: "spells/mana",
    title: "Mana",
    summary: "INT/WIS factor table, mana formula, effective caster level for Rangers/Rogues.",
    section: "spells",
    order: 20,
    sources: [
      {
        label: "spells.txt · mana factor table",
        href: "https://beej.us/moria/spells.txt"
      },
      {
        label: "mmspoilers · Mana",
        href: "https://beej.us/moria/mmspoilers/spells.html#mana"
      }
    ],
    relatedSlugs: ["spells/system", "character/attributes", "spells/mage", "spells/priest", "spells/failure"],
    dependsOnSlugs: ["spells/system"]
  },
  {
    slug: "spells/failure",
    title: "Spell failure",
    summary: "Failure rate formula, insufficient-mana penalty, 5–95% bounds.",
    section: "spells",
    order: 30,
    sources: [
      {
        label: "spells.txt · failure discussion",
        href: "https://beej.us/moria/spells.txt"
      },
      {
        label: "mmspoilers · Failure",
        href: "https://beej.us/moria/mmspoilers/spells.html#failure"
      }
    ],
    relatedSlugs: ["spells/mana", "character/attributes"],
    dependsOnSlugs: ["spells/mana", "spells/system"]
  },
  {
    slug: "spells/mage",
    title: "Mage spells",
    summary: "Full mage/ranger/rogue spell tables, book groupings, recharge math, effect blurbs.",
    section: "spells",
    order: 40,
    sources: [
      {
        label: "spells.txt · §1.2 Mage Spells",
        href: "https://beej.us/moria/spells.txt"
      },
      {
        label: "mmspoilers · Mage Spells",
        href: "https://beej.us/moria/mmspoilers/spells.html#magespells"
      },
      {
        label: "spells.txt · §1.3 Monster saves",
        href: "https://beej.us/moria/spells.txt"
      }
    ],
    relatedSlugs: ["spells/system", "spells/failure", "items/books", "spells/priest", "character/classes"],
    dependsOnSlugs: ["spells/system", "spells/failure", "spells/mana", "character/classes"]
  },
  {
    slug: "spells/priest",
    title: "Priest spells",
    summary: "Full cleric/paladin prayer tables and alphabetical effect summaries.",
    section: "spells",
    order: 50,
    sources: [
      {
        label: "spells.txt · §1.1 Priest Spells",
        href: "https://beej.us/moria/spells.txt"
      },
      {
        label: "mmspoilers · Cleric Spells",
        href: "https://beej.us/moria/mmspoilers/spells.html#clericspells"
      },
      {
        label: "spells.txt · §1.3 Monster saves",
        href: "https://beej.us/moria/spells.txt"
      }
    ],
    relatedSlugs: ["spells/system", "spells/failure", "items/books", "spells/mage", "character/classes"],
    dependsOnSlugs: ["spells/system", "spells/failure", "spells/mana", "character/classes"]
  },
  {
    slug: "wizard/overview",
    title: "Entering Wizard Mode",
    summary: "^W toggle, `-w` CLI, resurrect dead char warning, high-score exclusion.",
    section: "wizard",
    order: 10,
    sources: [
      {
        label: "mmspoilers · Entering Wizard Mode",
        href: "https://beej.us/moria/mmspoilers/wizardmode.html#enteringwizard"
      }
    ],
    relatedSlugs: ["wizard/commands", "getting-started/playing"]
  },
  {
    slug: "wizard/commands",
    title: "Wizard commands",
    summary: "Normal vs rogue keymap table (^A–^W, mapping, teleport, etc.).",
    section: "wizard",
    order: 20,
    sources: [
      {
        label: "mmspoilers · Wizard Commands",
        href: "https://beej.us/moria/mmspoilers/wizardmode.html#wizardcommands"
      }
    ],
    relatedSlugs: ["wizard/overview", "wizard/items"],
    dependsOnSlugs: ["wizard/overview"]
  },
  {
    slug: "wizard/items",
    title: "Wizard items",
    summary: "@-create item prompts: Tval/Subval/flags hex tables for testing items.",
    section: "wizard",
    order: 30,
    sources: [
      {
        label: "mmspoilers · Wizard Items",
        href: "https://beej.us/moria/mmspoilers/wizardmode.html#wizarditems"
      }
    ],
    relatedSlugs: ["items/overview", "items/special-properties"],
    dependsOnSlugs: ["wizard/commands", "wizard/overview", "items/overview"]
  },
  {
    slug: "reference/sources",
    title: "Sources & attribution",
    summary: "Beej HTML conversion, spoiler credits, disclaimer, how to cite this docs site.",
    section: "reference",
    order: 10,
    sources: [
      {
        label: "mmspoilers · Credits",
        href: "https://beej.us/moria/mmspoilers/beginning.html#credits"
      },
      {
        label: "mmspoilers · Using the Spoilers",
        href: "https://beej.us/moria/mmspoilers/beginning.html#usingthespoilers"
      },
      {
        label: "Beej index",
        href: "https://beej.us/moria/mmspoilers/index.html"
      }
    ],
    relatedSlugs: [
      "reference/versions",
      "character/social-class",
      "items/overview",
      "spells/system",
      "items/special-properties"
    ]
  },
  {
    slug: "reference/versions",
    title: "Moria versions & FAQ notes",
    summary: "5.x vs 4.87, spoiler revision policy, FAQ pointer.",
    section: "reference",
    order: 20,
    sources: [
      {
        label: "mmspoilers · Moria Versions",
        href: "https://beej.us/moria/mmspoilers/general.html#moriaversions"
      },
      {
        label: "mmspoilers · FAQ",
        href: "https://beej.us/moria/mmspoilers/general.html#frequentlyaskedquestions"
      },
      {
        label: "mmspoilers · Revision History",
        href: "https://beej.us/moria/mmspoilers/beginning.html#revisionhistory"
      }
    ],
    relatedSlugs: ["getting-started/differences", "reference/sources"]
  }
];
