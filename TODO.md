# P0 — Minimum Viable Path (MVP: Single Literal, Byte-based)

## Scalar Baseline
* [X] **S0. Implement `memmem` fallback path**
  DoD: `find(h, n)` passes all empty pattern/present/absent/boundary(overlap) cases. Single-threaded, supports file/STDIN input.
* [ ] **S1. Implement chunk+overlap loader (Chunker)**
  DoD: Read with `chunk_bytes` and maintain `needle.len()-1` overlap. Handle end-of-file/empty file/small file (shorter than needle).
* [ ] **S2. Implement line indexer (scalar) v1**
  DoD: Generate line start offset vector by `\n` scanning. Accurate line_no conversion even for large lines (50MiB).

## SIMD Prefilter (Single Literal)

* [ ] **V0. Define candidate generation interface** (`Prefilter::candidates(h, needle) -> iter`)
  DoD: Works with scalar implementation. Ensure candidate positions are superset of actual matches (false positives OK, false negatives NO).
* [ ] **V1. AVX2: Single byte anchor (eq_byte) + mask scan**
  DoD: Generate candidate bit sequence with `_mm256_cmpeq_epi8`→`movemask`. Tail falls back to scalar. No UB.
* [ ] **V2. AVX2: (First/last) 2-anchor intersection mask**
  DoD: Reduce candidates with intersection mask using `v` and `shift(v)`. No false negatives.
* [ ] **V3. Candidate verifier v1**
  DoD: Final verification at candidate positions with length-based u128/byte comparison. Prohibit padding/overrides.
* [ ] **V4. Runtime ISA dispatcher (Auto: AVX2→SSE2→Scalar)**
  DoD: Multi-path selection based on `is_x86_feature_detected!` for x86 family. Safe fallback to scalar in unsupported environments.

## UTF-8 Boundary Option (Lightweight)

* [ ] **U0. Boundary bit check function** `is_char_start/end(h, pos)`
  DoD: O(1) check with `(b & 0xC0)!=0x80` rule, prevent out-of-bounds access.
* [ ] **U1. Integrate "boundary check" between prefilter→verification**
  DoD: Apply only when `UTF8_BOUNDARY` flag is active. Match expected results in ASCII/Korean/mixed text.

## Input/Output/CLI

* [ ] **I0. `GrepEngine::search` pipeline skeleton**
  DoD: Ensure Chunker→Prefilter→Verifier→LineMapper→Sink call order, clean error propagation.
* [ ] **I1. Minimal CLI options** (`-n`, `-H`, `--count`, `--utf8-boundary`)
  DoD: Determine standard output format (offset/line/filename). Internal consistency for `--count` (accumulation/no duplicates).

## Testing (Basic Accuracy)

* [ ] **T0. Functional test set**
  DoD: Empty pattern, 1/2/3/4/8/16/32/64B pattern hit/nohit/overlap, file/STDIN, chunk boundary hit all pass.
* [ ] **T1. UTF-8 boundary tests**
  DoD: Verify "Korea/Ko/rea" boundary cases, emoji, mixed text, NFC vs NFD (mismatch).
* [ ] **T2. Adversarial input tests**
  DoD: Maintain accuracy with single byte repetition, near matches (last 1 byte different), very long lines, etc.




# P1 — Performance Optimization (AVX2/NEON, Line Indexer SIMD)

## SIMD Kernel Improvements

* [ ] **V5. AVX2: Partial vector comparison (16/32B fast-path)**
  DoD: Primary verification with u128/ymm comparison from candidates, then final confirmation with byte comparison. No false negatives.
* [ ] **V6. Implement SSE2 path** (for AVX2 unsupported environments)
  DoD: 16B stride scan and mask scan. Include performance regression tests.
* [ ] **A0. NEON: Single byte anchor + candidate scan**
  DoD: 16B step scan on aarch64, same tail handling, equivalent accuracy.
* [ ] **A1. NEON: 2-anchor intersection mask**
  DoD: Candidate reduction effect equivalent to AVX2, record performance benchmarks.

## Line Indexer SIMD

* [ ] **L0. AVX2 `\n` scanner**
  DoD: 32B load→`== '\n'`→mask→popcnt accumulation. Line offset vector generation speed >1 GiB/s.
* [ ] **L1. NEON `\n` scanner**
  DoD: Same accuracy/interface. No performance regression even with large files.

## Benchmarking/Profiling

* [ ] **B0. Criterion benchmark grid** (length×hit rate)
  DoD: Pattern length {1,3,8,16,32,64}, hit rate {0,0.1,1,10%}, corpus {ASCII, Korean, binary} fixed suite.
* [ ] **B1. Auto comparison report by ISA**
  DoD: Output Scalar/SSE2/AVX2/NEON GB/s table in `--bench` results.




# P2 — Multi-literal, Threading, I/O Tuning

## Multi-literal (Few Patterns)

* [ ] **M0. Implement 2B key index table**
  DoD: Index each pattern→2B suffix key→pattern in bucket. Maintain superset with candidate→bucket verification.
* [ ] **M1. Fast filter within bucket** (nibble LUT/`vpshufb`-like)
  DoD: Additional candidate reduction in bucket with AVX2, no false negatives.

## Threading

* [ ] **R0. Parallel chunk processing (file range partitioning)**
  DoD: Chunk assignment per worker + ensure `needle.len()-1` overlap. Prevent duplicates when merging results.
* [ ] **R1. Result reporting merger**
  DoD: Maintain offset sorting, preserve `--count`/`-n` mode accuracy.

## I/O Tuning

* [ ] **IO0. Add mmap source**
  DoD: Auto-select mmap/read based on file size/OS. Fallback on memory shortage/mapping failure.
* [ ] **IO1. Auto chunk size tuner**
  DoD: Measure 2-3 candidate sizes during warmup, then select optimal value (min/max limits).



# P3 — Backend Extensions, Options, Platforms

## Large-scale Patterns/Regex

* [ ] **AC0. Connect Aho-Corasick backend (include option switching criteria)**
  DoD: Auto-switch above pattern count threshold. Accuracy/performance tests.
* [ ] **RX0. Integrate regex-automata frontend**
  DoD: Force prepass application when literal extraction possible, otherwise go directly to main body.

## Advanced SIMD/Platforms

* [ ] **V7. AVX-512 path (optional)**
  DoD: Auto defaults to AVX2, select AVX-512 with flag. Record performance/clock degradation comparison.
* [ ] **SVE2. SVMATCH experimental path (optional)**
  DoD: Feature gate only on supporting CPUs. Same accuracy, record benchmarks.

## Quality & Safety

* [ ] **Q0. UB/misalignment/override audit**
  DoD: Comments/justification/tests for all `unsafe` paths. Pass Miri/ASAN.
* [ ] **Q1. Regression bench CI**
  DoD: Fail if pattern length×hit rate bench GB/s falls below baseline.


## Work Order
1. S0→S1→I0→S2 → V0→V1→V3→V2→V4 → U0→U1 → I1 → T0/T1/T2
2. V5/V6/A0/A1 → L0/L1 → B0/B1
3. M0/M1 → R0/R1 → IO0/IO1
4. AC0/RX0 → V7/SVE2 → Q0/Q1