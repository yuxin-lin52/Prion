#!/usr/bin/env python3
"""Prion cross-language micro-benchmark runner.

Builds the same six benchmarks in up to eight languages, verifies that all
languages print the same result, and reports the median wall time, peak
memory, compile time and artifact size.

Toolchains are discovered from PATH; override with environment variables:
PRIONC, CXX, RUSTC, ZIG, GO, JAVAC, JAVA, DOTNET, SWIFTC.
Languages whose toolchain is missing are skipped.
"""
import ctypes
import json
import os
import shutil
import statistics
import subprocess
import sys
import time

SRC = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(SRC, "out")
BENCH = [("num", 20000000), ("fib", 35), ("list", 2000000),
         ("index", 10000), ("map", 200000), ("concat", 2000000)]
REP = int(os.environ.get("REP", "10"))
if os.environ.get("ONLY"):
    BENCH[:] = [b for b in BENCH if b[0] == os.environ["ONLY"]]


def tool(env_key, *names):
    p = os.environ.get(env_key)
    if p:
        return p
    for n in names:
        p = shutil.which(n)
        if p:
            return p
    return None


TOOLS = {
    "prionc": tool("PRIONC", "prionc"),
    "cxx": tool("CXX", "g++", "clang++"),
    "rustc": tool("RUSTC", "rustc"),
    "zig": tool("ZIG", "zig"),
    "go": tool("GO", "go"),
    "javac": tool("JAVAC", "javac"),
    "java": tool("JAVA", "java"),
    "dotnet": tool("DOTNET", "dotnet"),
    "swiftc": tool("SWIFTC", "swiftc"),
}

ENV = dict(os.environ)
ENV.setdefault("PRION_CC", tool("PRION_CC", "clang", "clang.exe") or "")
if ENV["PRION_CC"]:
    ENV["PRION_CC"] = shutil.which(ENV["PRION_CC"]) or ENV["PRION_CC"]


def run(args, timeout=1800, env=None):
    return subprocess.run(args, cwd=SRC, env=env or ENV, capture_output=True,
                          text=True, encoding="utf-8", errors="replace",
                          timeout=timeout)


class IO_COUNTERS(ctypes.Structure):
    _fields_ = [("ReadOperationCount", ctypes.c_ulonglong),
                ("WriteOperationCount", ctypes.c_ulonglong),
                ("OtherOperationCount", ctypes.c_ulonglong),
                ("ReadTransferCount", ctypes.c_ulonglong),
                ("WriteTransferCount", ctypes.c_ulonglong),
                ("OtherTransferCount", ctypes.c_ulonglong)]


class BASIC_LIMIT(ctypes.Structure):
    _fields_ = [("PerProcessUserTimeLimit", ctypes.c_int64),
                ("PerJobUserTimeLimit", ctypes.c_int64),
                ("LimitFlags", ctypes.c_uint32),
                ("MinimumWorkingSetSize", ctypes.c_size_t),
                ("MaximumWorkingSetSize", ctypes.c_size_t),
                ("ActiveProcessLimit", ctypes.c_uint32),
                ("Affinity", ctypes.c_size_t),
                ("PriorityClass", ctypes.c_uint32),
                ("SchedulingClass", ctypes.c_uint32)]


class EXT_LIMIT(ctypes.Structure):
    _fields_ = [("BasicLimitInformation", BASIC_LIMIT),
                ("IoInfo", IO_COUNTERS),
                ("ProcessMemoryLimit", ctypes.c_size_t),
                ("JobMemoryLimit", ctypes.c_size_t),
                ("PeakProcessMemoryUsed", ctypes.c_size_t),
                ("PeakJobMemoryUsed", ctypes.c_size_t)]


IS_WIN = sys.platform.startswith("win")
if IS_WIN:
    K32 = ctypes.WinDLL("kernel32", use_last_error=True)
    K32.CreateJobObjectW.restype = ctypes.c_void_p
    K32.CreateJobObjectW.argtypes = [ctypes.c_void_p, ctypes.c_wchar_p]
    K32.AssignProcessToJobObject.argtypes = [ctypes.c_void_p, ctypes.c_void_p]
    K32.QueryInformationJobObject.argtypes = [ctypes.c_void_p, ctypes.c_int,
                                              ctypes.c_void_p, ctypes.c_uint32,
                                              ctypes.POINTER(ctypes.c_uint32)]
    K32.SetInformationJobObject.argtypes = [ctypes.c_void_p, ctypes.c_int,
                                            ctypes.c_void_p, ctypes.c_uint32]
    K32.CloseHandle.argtypes = [ctypes.c_void_p]
    JOB_EXTENDED = 9


def peak_linux(pid):
    try:
        with open("/proc/%d/status" % pid, encoding="utf-8") as f:
            for line in f:
                if line.startswith("VmHWM:"):
                    return int(line.split()[1]) * 1024
    except OSError:
        pass
    return 0


def measure(argv):
    job = None
    if IS_WIN:
        job = K32.CreateJobObjectW(None, None)
        if job:
            info = EXT_LIMIT()
            K32.SetInformationJobObject(job, JOB_EXTENDED, ctypes.byref(info),
                                        ctypes.sizeof(info))
    t0 = time.perf_counter()
    p = subprocess.Popen(argv, cwd=SRC, env=ENV, stdout=subprocess.PIPE,
                         stderr=subprocess.PIPE)
    if job:
        K32.AssignProcessToJobObject(job, ctypes.c_void_p(int(p._handle)))
    peak = 0
    while p.poll() is None:
        if job:
            q = EXT_LIMIT()
            used = ctypes.c_uint32()
            if K32.QueryInformationJobObject(job, JOB_EXTENDED, ctypes.byref(q),
                                             ctypes.sizeof(q), ctypes.byref(used)):
                peak = max(peak, q.PeakProcessMemoryUsed)
        elif sys.platform.startswith("linux"):
            peak = max(peak, peak_linux(p.pid))
        time.sleep(0.005)
    out, err = p.communicate()
    ms = (time.perf_counter() - t0) * 1000.0
    if job:
        K32.CloseHandle(job)
    text = out.decode("utf-8", "replace").strip()
    if not text:
        text = err.decode("utf-8", "replace").strip()
    return ms, peak, text, p.returncode


def build_one(name, args, artifact, env=None):
    if artifact and os.path.exists(artifact):
        shutil.rmtree(artifact, ignore_errors=True) if os.path.isdir(artifact) else os.remove(artifact)
    t0 = time.perf_counter()
    r = run(args, env=env)
    sec = time.perf_counter() - t0
    if r.returncode:
        print("[skip] %s build failed: %s" % (name, (r.stdout + r.stderr)[-300:]))
        return None
    return sec


def plan():
    steps = {}
    if TOOLS["cxx"]:
        steps["C++"] = ("b_cpp", "b_cpp.exe" if IS_WIN else "b_cpp")
    if TOOLS["rustc"]:
        steps["Rust"] = ("b_rs", "b_rs.exe" if IS_WIN else "b_rs")
    if TOOLS["zig"]:
        steps["Zig"] = None
    if TOOLS["prionc"]:
        steps["Prion"] = None
    if TOOLS["javac"] and TOOLS["java"]:
        steps["Java"] = ("J.class", "J.class")
    if TOOLS["go"]:
        steps["Go"] = ("go", "go.exe" if IS_WIN else "go")
    if TOOLS["dotnet"]:
        steps["C#"] = ("cs", "cs")
    if TOOLS["swiftc"]:
        steps["Swift"] = ("s_swift", "s_swift.exe" if IS_WIN else "s_swift")
    return steps


def build(steps):
    res = {}
    exe = lambda n: os.path.join(OUT, n)
    if "C++" in steps:
        sec = build_one("C++", [TOOLS["cxx"], "-O3", "-o", exe(steps["C++"][1]), "b.cpp"], exe(steps["C++"][1]))
        res["C++"] = (sec, os.path.getsize(exe(steps["C++"][1])) if sec else None)
    if "Rust" in steps:
        sec = build_one("Rust", [TOOLS["rustc"], "-C", "opt-level=3", "-C", "panic=abort",
                                 "-o", exe(steps["Rust"][1]), "b.rs"], exe(steps["Rust"][1]))
        res["Rust"] = (sec, os.path.getsize(exe(steps["Rust"][1])) if sec else None)
    if "Zig" in steps:
        sec, size = 0.0, 0
        for k, _ in BENCH:
            out = exe("z_%s%s" % (k, ".exe" if IS_WIN else ""))
            s = build_one("Zig", [TOOLS["zig"], "build-exe", "-O", "ReleaseFast",
                                  "-femit-bin=" + out, "z_%s.zig" % k], out)
            if s:
                sec += s
                size += os.path.getsize(out)
        res["Zig"] = (sec, size)
    if "Prion" in steps:
        sec, size = 0.0, 0
        for k, _ in BENCH:
            art = os.path.join(SRC, "build", "release", "p_%s%s" % (k, ".exe" if IS_WIN else ""))
            s = build_one("Prion", [TOOLS["prionc"], "build", "p_" + k, "--mode", "release"], art)
            if s:
                shutil.copyfile(art, exe(os.path.basename(art)))
                sec += s
                size += os.path.getsize(art)
        res["Prion"] = (sec, size)
    if "Java" in steps:
        sec = build_one("Java", [TOOLS["javac"], "-d", OUT, "J.java"], exe("J.class"))
        res["Java"] = (sec, os.path.getsize(exe("J.class")) if sec else None)
    if "Go" in steps:
        out = exe(steps["Go"][1])
        sec = build_one("Go", [TOOLS["go"], "build", "-trimpath", "-ldflags=-s -w",
                               "-o", out, "go_bench.go"], out)
        res["Go"] = (sec, os.path.getsize(out) if sec else None)
    if "C#" in steps:
        d = exe("cs")
        sec = build_one("C#", [TOOLS["dotnet"], "build", "Program.csproj", "-c", "Release",
                               "-o", d, "--nologo"], d)
        size = None
        for cand in ("Program.dll", "Program.exe"):
            p = os.path.join(d, cand)
            if sec and os.path.exists(p):
                size = os.path.getsize(p)
                break
        res["C#"] = (sec, size)
    if "Swift" in steps:
        out = exe(steps["Swift"][1])
        sec = build_one("Swift", [TOOLS["swiftc"], "-O", "-o", out, "S.swift"], out)
        res["Swift"] = (sec, os.path.getsize(out) if sec else None)
    return res


def argv(name, mode, n):
    exe = lambda x: os.path.join(OUT, x)
    suffix = ".exe" if IS_WIN else ""
    if name == "C++":
        return [exe("b_cpp" + suffix), mode, str(n)]
    if name == "Rust":
        return [exe("b_rs" + suffix), mode, str(n)]
    if name == "Zig":
        return [exe("z_%s%s" % (mode, suffix)), str(n)]
    if name == "Prion":
        return [exe("p_%s%s" % (mode, suffix)), str(n)]
    if name == "Java":
        return [TOOLS["java"], "-cp", OUT, "J", mode, str(n)]
    if name == "Go":
        return [exe("go" + suffix), mode, str(n)]
    if name == "C#":
        return [exe(os.path.join("cs", "Program" + suffix)), mode, str(n)]
    return [exe("s_swift" + suffix), mode, str(n)]


WARM = {"Swift": 2, "Java": 2, "C#": 2, "Go": 1}
BASE = ("Swift", "Java", "C#", "Go")


def sample(name, mode, n):
    for _ in range(WARM.get(name, 1)):
        measure(argv(name, mode, n))
    times, peaks, out = [], [], ""
    for _ in range(REP):
        ms, pk, out, rc = measure(argv(name, mode, n))
        if rc == 0:
            times.append(ms)
            peaks.append(pk)
    if not times:
        raise RuntimeError("all runs failed")
    return times, peaks, out


def main():
    os.makedirs(OUT, exist_ok=True)
    steps = plan()
    if not steps:
        print("no toolchain found")
        return 1
    print("languages:", ", ".join(steps))
    builds = build(steps)
    base = {}
    for name in steps:
        if name not in BASE:
            base[name] = 0.0
            continue
        try:
            ts, _, _ = sample(name, "none", 1)
            base[name] = statistics.median(ts)
        except Exception:
            base[name] = 0.0
    result = {"repeat": REP, "bench": {}, "base": base,
              "build": {k: {"sec": v[0], "bytes": v[1]} for k, v in builds.items()}}
    names = list(steps)
    for k, n in BENCH:
        result["bench"][k] = {}
        outs = {}
        for name in names:
            try:
                ts, peaks, out = sample(name, k, n)
                med = statistics.median(ts)
                result["bench"][k][name] = {
                    "median_ms": round(med, 2),
                    "net_ms": round(max(med - base[name], 0.0), 2),
                    "min_ms": round(min(ts), 2),
                    "max_ms": round(max(ts), 2),
                    "peak_mem": max(peaks),
                    "out": out,
                }
                outs[name] = out
            except Exception as e:
                result["bench"][k][name] = {"error": str(e)}
        ref = next(iter(outs.values()), "")
        bad = [nm for nm, v in outs.items() if v != ref]
        if bad:
            result["bench"][k]["mismatch"] = bad
            print("mismatch in", k, ":", bad)
    with open(os.path.join(OUT, "results.json"), "w", encoding="utf-8", newline="\n") as f:
        json.dump(result, f, ensure_ascii=False, indent=2)
    print("results written to", os.path.join(OUT, "results.json"))
    return 0


if __name__ == "__main__":
    sys.exit(main())
