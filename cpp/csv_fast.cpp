// Fast CSV parser: mmap + zero-copy string_view + from_chars, no allocation in the hot loop.
// Build: clang++ -std=c++20 -O3 -march=native csv_fast.cpp -o csv_fast
#include <charconv>
#include <cstdint>
#include <cstdio>
#include <cstring>
#include <string_view>

#include <fcntl.h>
#include <sys/mman.h>
#include <sys/stat.h>
#include <unistd.h>

namespace {

// Parses an unsigned decimal "123" or "123.45" into hundredths (cents) without any allocation.
// Fields in this file have at most two decimals.
inline bool parse_cents(std::string_view s, int64_t& out) {
    int64_t whole = 0;
    auto [p, ec] = std::from_chars(s.data(), s.data() + s.size(), whole);
    if (ec != std::errc{}) return false;
    int64_t frac = 0;
    if (p != s.data() + s.size()) {
        if (*p != '.') return false;
        ++p;
        size_t digits = static_cast<size_t>(s.data() + s.size() - p);
        if (digits == 0 || digits > 2) return false;
        auto [q, ec2] = std::from_chars(p, s.data() + s.size(), frac);
        if (ec2 != std::errc{} || q != s.data() + s.size()) return false;
        if (digits == 1) frac *= 10;
    }
    out = whole * 100 + frac;
    return true;
}

template <class T>
inline bool parse_int(std::string_view s, T& out) {
    auto [p, ec] = std::from_chars(s.data(), s.data() + s.size(), out);
    return ec == std::errc{} && p == s.data() + s.size();
}

struct Totals {
    uint64_t rows = 0;
    int64_t salary_cents = 0;
    uint64_t age = 0;
    uint64_t active = 0;
    int64_t score_cents = 0;
    uint64_t last_id = 0;
    std::string_view last_name, last_email, last_city;
};

}  // namespace

int main(int argc, char** argv) {
    const char* path = argc > 1 ? argv[1] : "data.csv";

    int fd = open(path, O_RDONLY);
    if (fd < 0) { std::perror("open"); return 1; }
    struct stat st;
    if (fstat(fd, &st) != 0) { std::perror("fstat"); return 1; }
    size_t size = static_cast<size_t>(st.st_size);

    void* map = mmap(nullptr, size, PROT_READ, MAP_PRIVATE, fd, 0);
    if (map == MAP_FAILED) { std::perror("mmap"); return 1; }
    madvise(map, size, MADV_SEQUENTIAL);
    close(fd);

    const char* cur = static_cast<const char*>(map);
    const char* const end = cur + size;

    // skip header
    const char* nl = static_cast<const char*>(memchr(cur, '\n', size));
    cur = nl ? nl + 1 : end;

    Totals t;
    while (cur < end) {
        const char* line_end = static_cast<const char*>(memchr(cur, '\n', end - cur));
        if (!line_end) line_end = end;
        if (line_end > cur && line_end[-1] == '\r') --line_end;  // tolerate CRLF

        // Split the line into 8 fields with memchr (SIMD-accelerated in libc).
        std::string_view f[8];
        const char* p = cur;
        int n = 0;
        for (; n < 7; ++n) {
            const char* c = static_cast<const char*>(memchr(p, ',', line_end - p));
            if (!c) break;
            f[n] = {p, static_cast<size_t>(c - p)};
            p = c + 1;
        }
        if (n != 7) {
            std::fprintf(stderr, "malformed line near offset %zu\n",
                         static_cast<size_t>(cur - static_cast<const char*>(map)));
            return 1;
        }
        f[7] = {p, static_cast<size_t>(line_end - p)};

        uint64_t id; uint32_t age; int64_t salary, score;
        if (!parse_int(f[0], id) || !parse_int(f[3], age) ||
            !parse_cents(f[4], salary) || !parse_cents(f[7], score)) {
            std::fprintf(stderr, "bad number on row %llu\n",
                         static_cast<unsigned long long>(t.rows + 1));
            return 1;
        }

        ++t.rows;
        t.salary_cents += salary;
        t.age += age;
        t.score_cents += score;
        t.active += (f[6] == "true");
        t.last_id = id;
        t.last_name = f[1];
        t.last_email = f[2];
        t.last_city = f[5];

        cur = line_end < end ? line_end + 1 : end;
        if (cur < end && cur[-1] == '\r') ++cur;  // consume '\n' after '\r'
    }

    const double rows = static_cast<double>(t.rows);
    std::printf("rows         : %llu\n", static_cast<unsigned long long>(t.rows));
    std::printf("total salary : %lld.%02lld\n",
                static_cast<long long>(t.salary_cents / 100),
                static_cast<long long>(t.salary_cents % 100));
    std::printf("mean age     : %.4f\n", static_cast<double>(t.age) / rows);
    std::printf("active       : %llu\n", static_cast<unsigned long long>(t.active));
    std::printf("mean score   : %.4f\n", static_cast<double>(t.score_cents) / 100.0 / rows);
    std::printf("last record  : #%llu %.*s <%.*s> %.*s\n",
                static_cast<unsigned long long>(t.last_id),
                static_cast<int>(t.last_name.size()), t.last_name.data(),
                static_cast<int>(t.last_email.size()), t.last_email.data(),
                static_cast<int>(t.last_city.size()), t.last_city.data());

    munmap(map, size);
    return 0;
}
