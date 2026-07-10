// Phase 4.5.3.2 — golden capture for rings/amulets/wands/staffs/chests helpers.
#include "headers.h"
#include <cstdio>

static void print_item(const Inventory_t &item) {
    printf("  misc_use=%d to_hit=%d to_damage=%d to_ac=%d flags=0x%x cost=%d "
           "identification=0x%x special_name_id=%u sub_category_id=%u\n",
           (int)item.misc_use, (int)item.to_hit, (int)item.to_damage, (int)item.to_ac,
           item.flags, (int)item.cost, (unsigned)item.identification,
           (unsigned)item.special_name_id, (unsigned)item.sub_category_id);
}

static void run_ring(int seed, int sub, int level, int base_cost) {
    setRandomSeed(seed);
    Inventory_t item{};
    item.category_id = TV_RING;
    item.sub_category_id = (uint8_t)sub;
    item.cost = base_cost;
    game.treasure.list[1] = item;
    magicTreasureMagicalAbility(1, level);
    printf("RING seed=%d sub=%d level=%d cursed=%d\n", seed, sub, level,
           (10 * (15 + level > 70 ? 70 : 15 + level)) / 13);
    print_item(game.treasure.list[1]);
    printf("  next_rng max=100 val=%d\n", randomNumber(100));
}

static void run_amulet(int seed, int sub, int level, int base_cost) {
    setRandomSeed(seed);
    Inventory_t item{};
    item.category_id = TV_AMULET;
    item.sub_category_id = (uint8_t)sub;
    item.cost = base_cost;
    game.treasure.list[1] = item;
    magicTreasureMagicalAbility(1, level);
    printf("AMULET seed=%d sub=%d level=%d\n", seed, sub, level);
    print_item(game.treasure.list[1]);
    printf("  next_rng max=100 val=%d\n", randomNumber(100));
}

static void run_wand(int seed, int sub) {
    setRandomSeed(seed);
    Inventory_t item{};
    item.category_id = TV_WAND;
    item.sub_category_id = (uint8_t)sub;
    game.treasure.list[1] = item;
    magicTreasureMagicalAbility(1, 10);
    printf("WAND seed=%d sub=%d\n", seed, sub);
    print_item(game.treasure.list[1]);
    printf("  next_rng max=100 val=%d\n", randomNumber(100));
}

static void run_staff(int seed, int sub) {
    setRandomSeed(seed);
    Inventory_t item{};
    item.category_id = TV_STAFF;
    item.sub_category_id = (uint8_t)sub;
    game.treasure.list[1] = item;
    magicTreasureMagicalAbility(1, 10);
    printf("STAFF seed=%d sub=%d\n", seed, sub);
    print_item(game.treasure.list[1]);
    printf("  next_rng max=100 val=%d\n", randomNumber(100));
}

static void run_chest(int seed, int level) {
    setRandomSeed(seed);
    Inventory_t item{};
    item.category_id = TV_CHEST;
    game.treasure.list[1] = item;
    magicTreasureMagicalAbility(1, level);
    printf("CHEST seed=%d level=%d magic_type_roll max=%d\n", seed, level, level + 4);
    print_item(game.treasure.list[1]);
    printf("  next_rng max=100 val=%d\n", randomNumber(100));
}

int main() {
    // Rings: all handled sub_categories at level 10, seed 42
    for (int sub : {0, 1, 2, 3, 4, 5, 19, 20, 21, 24, 25, 26, 27, 28, 29, 30}) {
        run_ring(42, sub, 10, 100);
    }
    // Ring case 4 cursed path — seed 2 tends to hit cursed at cursed=19
    run_ring(2, 4, 10, 100);
    run_ring(100, 0, 10, 100);
    run_ring(42, 0, 50, 100);

    // Amulets
    for (int sub : {0, 1, 2, 8}) {
        run_amulet(42, sub, 10, 200);
    }
    run_amulet(2, 2, 10, 200);

    // Wands: all ids + default
    for (int id = 0; id <= 24; id++) {
        run_wand(42, id);
    }

    // Staffs: all ids + default
    for (int id = 0; id <= 23; id++) {
        run_staff(42, id);
    }

    // Chests: level/seed sweep
    for (int level : {1, 5, 10, 20, 50}) {
        run_chest(42, level);
    }
    run_chest(777, 10);
    run_chest(2, 10);

    return 0;
}
