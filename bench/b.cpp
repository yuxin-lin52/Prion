#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <string>
#include <unordered_map>
#include <vector>

static const long long MOD = 1000000007LL;

static long long num(long long n) {
    long long s = 0;
    for (long long i = 1; i <= n; i++) s = (s * 31 + i) % MOD;
    return s;
}

static long long fib(long long n) { return n < 2 ? n : fib(n - 1) + fib(n - 2); }

static long long listb(long long n) {
    std::vector<long long> v;
    v.reserve((size_t)n);
    for (long long i = 0; i < n; i++) v.push_back(i);
    long long s = 0;
    for (long long x : v) s += x;
    return s;
}

static long long indexb(long long n) {
    std::vector<long long> v;
    v.reserve((size_t)n);
    for (long long i = 0; i < n; i++) v.push_back(i);
    long long s = 0;
    for (long long r = 0; r < 3000; r++)
        for (long long x : v) s = (s * 31 + x) % MOD;
    return s;
}

static long long mapb(long long n) {
    std::unordered_map<long long, long long> m;
    m.reserve((size_t)n * 2);
    for (long long i = 0; i < n; i++) m.emplace(i, i * 2);
    long long s = 0;
    for (long long i = 0; i < n; i++) s += m.at(i);
    return s;
}

static long long concatb(long long n) {
    long long k = 0;
    for (long long i = 0; i < n; i++) {
        std::string s;
        s.reserve(6);
        s.append("ab");
        s.append("cd");
        s.append("ef");
        __asm__ __volatile__("" : : "g"(s.data()) : "memory");
        k += (long long)s.size();
    }
    return k;
}

int main(int argc, char** argv) {
    const char* w = argc > 1 ? argv[1] : "num";
    long long n = argc > 2 ? std::atoll(argv[2]) : 0;
    long long r = 0;
    if (!std::strcmp(w, "num")) r = num(n ? n : 200000000LL);
    else if (!std::strcmp(w, "fib")) r = fib(n ? n : 35);
    else if (!std::strcmp(w, "list")) r = listb(n ? n : 2000000LL);
    else if (!std::strcmp(w, "index")) r = indexb(n ? n : 10000LL);
    else if (!std::strcmp(w, "map")) r = mapb(n ? n : 200000LL);
    else if (!std::strcmp(w, "concat")) r = concatb(n ? n : 200000LL);
    std::printf("%lld\n", r);
    return 0;
}
