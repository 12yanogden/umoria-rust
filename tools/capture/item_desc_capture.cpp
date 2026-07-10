// Phase 4.5.4.2 — itemDescription golden-capture harness.

#include <cstdint>
#include <cstdio>
#include <cstring>

#include "headers.h"

extern char magic_item_titles[MAX_TITLES][10];

static Inventory_t make_item(uint16_t id, uint8_t sub_category_id) {
    Inventory_t item{};
    inventoryItemCopyTo((int) id, item);
    item.sub_category_id = sub_category_id;
    item.items_count = 1;
    return item;
}

static void emit_desc(const char *name, Inventory_t item, bool add_prefix) {
    obj_desc_t desc{};
    itemDescription(desc, item, add_prefix);
    std::printf("DESC\t%s\t%d\t%s\n", name, add_prefix ? 1 : 0, desc);
}

int main() {
    std::memset(objects_identified, 0, sizeof(objects_identified));

    game.magic_seed = 42;
    magicInitializeItemNames();

    {
        auto item = make_item(332, 64); // Rat Skeleton
        emit_desc("misc", item, true);
        emit_desc("misc_no_prefix", item, false);
    }
    {
        auto item = make_item(81, 64);
        item.damage.dice = 2;
        item.damage.sides = 6;
        emit_desc("arrow_2d6", item, true);
    }
    {
        auto item = make_item(75, 64);
        item.misc_use = 3;
        emit_desc("bow_x3", item, true);
    }
    {
        auto item = make_item(34, 64); // Bastard Sword
        item.damage.dice = 3;
        item.damage.sides = 8;
        item.to_hit = 2;
        item.to_damage = 5;
        item.identification = config::identification::ID_KNOWN2
                              | config::identification::ID_SHOW_HIT_DAM;
        emit_desc("sword_hit_dam", item, true);
    }
    {
        auto item = make_item(34, 64);
        item.damage.dice = 2;
        item.damage.sides = 6;
        item.misc_use = -2;
        item.flags = config::treasure::flags::TR_STR;
        item.identification = config::identification::ID_KNOWN2;
        emit_desc("sword_cursed_str", item, true);
    }
    {
        auto item = make_item(98, 64);
        item.ac = 0;
        item.to_ac = 3;
        item.identification = config::identification::ID_KNOWN2;
        emit_desc("helm_ac_bracket", item, true);
    }
    {
        auto item = make_item(130, 64);
        item.ac = 5;
        item.identification = config::identification::ID_KNOWN2;
        emit_desc("shield_ac5", item, true);
    }
    {
        auto item = make_item(87, 64);
        item.misc_use = 750;
        emit_desc("light_turns", item, true);
    }
    {
        auto item = make_item(243, 64);
        emit_desc("potion_unknown", item, true);
    }
    {
        itemSetAsIdentified(TV_POTION1, 64);
        auto item = make_item(243, 64);
        emit_desc("potion_known", item, true);
    }
    {
        std::memset(objects_identified, 0, sizeof(objects_identified));
        magicInitializeItemNames();
        auto item = make_item(177, 64);
        emit_desc("scroll_unknown", item, true);
    }
    {
        auto item = make_item(293, 64);
        item.misc_use = 5;
        item.identification = config::identification::ID_KNOWN2;
        emit_desc("staff_charges", item, true);
    }
    {
        auto item = make_item(132, 64);
        item.misc_use = 2;
        item.identification = config::identification::ID_KNOWN2;
        emit_desc("ring_plus2", item, true);
    }
    {
        auto item = make_item(163, 64);
        item.misc_use = 1;
        item.identification = config::identification::ID_KNOWN2;
        emit_desc("amulet_plus1", item, true);
    }
    {
        auto item = make_item(243, 64);
        item.category_id = TV_FOOD;
        item.sub_category_id = 64;
        emit_desc("mushroom_unknown", item, true);
    }
    {
        auto item = make_item(243, 64);
        item.category_id = TV_FOOD;
        item.sub_category_id = 64;
        item.items_count = 3;
        emit_desc("mushroom_plural", item, true);
    }
    {
        auto item = make_item(408, 64);
        emit_desc("gold", item, true);
    }
    {
        auto item = make_item(373, 64);
        emit_desc("store_door", item, true);
    }
    {
        auto item = make_item(34, 64);
        item.damage.dice = 2;
        item.damage.sides = 6;
        item.special_name_id = SpecialNameIds::SN_SLAYING;
        item.identification = config::identification::ID_KNOWN2;
        emit_desc("special_name", item, true);
    }
    {
        auto item = make_item(293, 64);
        item.misc_use = 3;
        item.identification = config::identification::ID_KNOWN2
                              | config::identification::ID_MAGIK
                              | config::identification::ID_EMPTY;
        std::strcpy(item.inscription, "abc");
        emit_desc("inscription_flags", item, true);
    }
    {
        auto item = make_item(243, 65);
        int16_t id = objectPositionOffset(TV_POTION1, 65);
        id <<= 6;
        id += (65 & (ITEM_SINGLE_STACK_MIN - 1));
        objects_identified[id] |= config::identification::OD_TRIED;
        emit_desc("tried_potion", item, true);
    }
    {
        auto item = make_item(243, 64);
        item.identification = config::identification::ID_STORE_BOUGHT;
        emit_desc("store_bought_potion", item, true);
    }
    {
        auto item = make_item(88, 64);
        item.damage.dice = 2;
        item.damage.sides = 8;
        item.misc_use = 3;
        item.identification = config::identification::ID_KNOWN2;
        emit_desc("digging_zplusses", item, true);
    }
    {
        auto item = make_item(243, 64);
        item.items_count = 0;
        emit_desc("no_more_potion", item, true);
    }
    {
        auto item = make_item(319, 64);
        emit_desc("magic_book", item, true);
    }
    {
        auto item = make_item(324, 64);
        emit_desc("prayer_book", item, true);
    }

    return 0;
}
