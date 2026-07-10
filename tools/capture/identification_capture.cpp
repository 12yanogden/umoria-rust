// Phase 4.5.4.1 — Standalone identification flavor-init golden capture.
//
// Links against unmodified reference object code. Invoke once per magic_seed
// (arrays mutate in place and cannot be reset without a fresh process).

#include <cstdint>
#include <cstdio>
#include <string>

constexpr uint8_t MAX_COLORS = 49;
constexpr uint8_t MAX_MUSHROOMS = 22;
constexpr uint8_t MAX_WOODS = 25;
constexpr uint8_t MAX_METALS = 25;
constexpr uint8_t MAX_ROCKS = 32;
constexpr uint8_t MAX_AMULETS = 11;
constexpr uint8_t MAX_TITLES = 45;

extern const char *colors[MAX_COLORS];
extern const char *mushrooms[MAX_MUSHROOMS];
extern const char *woods[MAX_WOODS];
extern const char *metals[MAX_METALS];
extern const char *rocks[MAX_ROCKS];
extern const char *amulets[MAX_AMULETS];
extern char magic_item_titles[MAX_TITLES][10];

struct Game_t {
    uint32_t magic_seed;
};
extern Game_t game;

uint32_t getRandomSeed();
void setRandomSeed(uint32_t seed);
void magicInitializeItemNames();

namespace {

FILE *openOut(const std::string &path) {
    FILE *fp = fopen(path.c_str(), "w");
    if (fp == nullptr) {
        fprintf(stderr, "identification_capture: cannot open %s\n", path.c_str());
    }
    return fp;
}

void writeStringArray(FILE *fp, const char *label, const char *const *arr, int count) {
    fprintf(fp, "[%s]\n", label);
    for (int i = 0; i < count; i++) {
        fprintf(fp, "%s\n", arr[i]);
    }
}

void writeTitles(FILE *fp) {
    fprintf(fp, "[magic_item_titles]\n");
    for (int i = 0; i < MAX_TITLES; i++) {
        fprintf(fp, "%s\n", magic_item_titles[i]);
    }
}

} // namespace

int main(int argc, char **argv) {
    if (argc < 4) {
        fprintf(stderr, "usage: %s <output_dir> <main_rng_seed> <magic_seed>\n", argv[0]);
        return 1;
    }

    std::string dir = argv[1];
    uint32_t main_seed = static_cast<uint32_t>(std::stoul(argv[2]));
    uint32_t magic_seed = static_cast<uint32_t>(std::stoul(argv[3]));

    setRandomSeed(main_seed);
    uint32_t main_before = getRandomSeed();

    game.magic_seed = magic_seed;
    magicInitializeItemNames();

    uint32_t main_after = getRandomSeed();

    std::string path = dir + "/magic_init_seed" + std::to_string(magic_seed) + ".txt";
    FILE *fp = openOut(path);
    fprintf(fp, "magic_seed=%u\n", magic_seed);
    fprintf(fp, "main_seed_before=%u\n", main_before);
    fprintf(fp, "main_seed_after=%u\n", main_after);
    writeStringArray(fp, "colors", colors, MAX_COLORS);
    writeStringArray(fp, "woods", woods, MAX_WOODS);
    writeStringArray(fp, "metals", metals, MAX_METALS);
    writeStringArray(fp, "rocks", rocks, MAX_ROCKS);
    writeStringArray(fp, "amulets", amulets, MAX_AMULETS);
    writeStringArray(fp, "mushrooms", mushrooms, MAX_MUSHROOMS);
    writeTitles(fp);
    fclose(fp);

    printf("identification_capture: wrote %s\n", path.c_str());
    return 0;
}
