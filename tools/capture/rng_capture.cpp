// Phase 1.4.2 — Standalone RNG golden-capture harness.
//
// Links against the UNMODIFIED reference object code (every src/*.cpp EXCEPT
// main.cpp; never with TEST_RNG defined) and emits deterministic golden RNG
// artifacts that the Rust port is later diffed against.
//
// The reference symbols are forward-declared here (matching their C++
// signatures) rather than pulling in headers.h, so this file stays a tiny,
// dependency-light harness. It NEVER initializes curses.
//
// Usage: rng_capture <output_dir>   (defaults to tests/golden/rng)

#include <cstdint>
#include <cstdio>
#include <string>

// --- from src/rng.cpp ---
uint32_t getRandomSeed();
void setRandomSeed(uint32_t seed);
int32_t rnd();

// --- from src/game.cpp ---
int randomNumber(int max);
int randomNumberNormalDistribution(int mean, int standard);

// --- from src/game.h (defined in the reference build) ---
constexpr int NORMAL_TABLE_SIZE = 256;
extern uint16_t normal_table[NORMAL_TABLE_SIZE];

namespace {

const uint32_t SEEDS[] = {1u, 42u, 12345u, 2147483647u};

// randomNumber(max) sample: the `max` values we probe.
const int RN_MAXES[] = {2, 6, 10, 100, 32767};

// randomNumberNormalDistribution(mean, standard) sample pairs.
struct MeanSd { int mean; int sd; };
const MeanSd ND_PARAMS[] = {{10, 4}, {50, 10}, {100, 25}};

FILE *openOut(const std::string &dir, const std::string &name) {
    std::string path = dir + "/" + name;
    FILE *fp = fopen(path.c_str(), "w");
    if (fp == nullptr) {
        fprintf(stderr, "rng_capture: cannot open %s\n", path.c_str());
    }
    return fp;
}

// z[10001] invariant: setRandomSeed(0) -> effective internal seed 1, then the
// 10000th rnd() must equal 1043618065 (Park-Miller canonical anchor).
void captureInvariant(const std::string &dir) {
    setRandomSeed(0);
    int32_t value = 0;
    for (int i = 0; i < 10000; i++) {
        value = rnd();
    }
    FILE *fp = openOut(dir, "z10001.txt");
    fprintf(fp, "%d\n", value);
    fclose(fp);
}

// Raw rnd() sequence per seed. Seed 1 gets >= 10001 values; others get 1000.
void captureRndSequences(const std::string &dir) {
    for (uint32_t seed : SEEDS) {
        setRandomSeed(seed);
        int count = (seed == 1u) ? 10001 : 1000;
        FILE *fp = openOut(dir, "rnd_seed" + std::to_string(seed) + ".txt");
        for (int i = 0; i < count; i++) {
            fprintf(fp, "%d\n", rnd());
        }
        fclose(fp);
    }
}

void captureRandomNumber(const std::string &dir) {
    for (uint32_t seed : SEEDS) {
        setRandomSeed(seed);
        FILE *fp = openOut(dir, "randomNumber_seed" + std::to_string(seed) + ".txt");
        for (int max : RN_MAXES) {
            for (int i = 0; i < 20; i++) {
                fprintf(fp, "%d %d\n", max, randomNumber(max));
            }
        }
        fclose(fp);
    }
}

void captureNormalDist(const std::string &dir) {
    for (uint32_t seed : SEEDS) {
        setRandomSeed(seed);
        FILE *fp = openOut(dir, "normalDist_seed" + std::to_string(seed) + ".txt");
        for (const MeanSd &p : ND_PARAMS) {
            for (int i = 0; i < 20; i++) {
                fprintf(fp, "%d %d %d\n", p.mean, p.sd, randomNumberNormalDistribution(p.mean, p.sd));
            }
        }
        fclose(fp);
    }
}

void captureNormalTable(const std::string &dir) {
    FILE *fp = openOut(dir, "normal_table.txt");
    for (int i = 0; i < NORMAL_TABLE_SIZE; i++) {
        fprintf(fp, "%u\n", (unsigned) normal_table[i]);
    }
    fclose(fp);
}

} // namespace

int main(int argc, char **argv) {
    std::string dir = (argc > 1) ? argv[1] : "tests/golden/rng";
    captureInvariant(dir);
    captureRndSequences(dir);
    captureRandomNumber(dir);
    captureNormalDist(dir);
    captureNormalTable(dir);
    printf("rng_capture: wrote golden files to %s\n", dir.c_str());
    return 0;
}
