use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;
use simd_grep::{find, contains};

#[test]
fn empty_needle_is_zero() {
    assert_eq!(find(b"", b""), Some(0));
    assert_eq!(find(b"abc", b""), Some(0));
}

#[test]
fn small_hits_and_misses() {
    let h = b"abcdef";
    assert_eq!(find(h, b"a"), Some(0));
    assert_eq!(find(h, b"f"), Some(5));
    assert_eq!(find(h, b"bc"), Some(1));
    assert_eq!(find(h, b"ef"), Some(4));
    assert_eq!(find(h, b"gh"), None);
    assert!(contains(h, b"cde"));
    assert!(!contains(h, b"zzz"));
}

#[test]
fn overlap_cases() {
    // Possible overlap match: "aaa" in "aaaaa" → first match is at 0
    assert_eq!(find(b"aaaaa", b"aaa"), Some(0));
    // Duplicate reporting will be handled in higher-level logic (S0 only returns first match)
}

#[test]
fn boundary_cases_prefix_suffix() {
    assert_eq!(find(b"NEEDLE--tail", b"NEEDLE"), Some(0));
    assert_eq!(find(b"head--NEEDLE", b"NEEDLE"), Some(6));
    assert_eq!(find(b"--head--", b"NEEDLE"), None);
}

#[test]
fn with_nul_bytes_and_binary_like() {
    let h = b"\x00\x00A\x00B\x00\x00";
    assert_eq!(find(h, b"A\x00B"), Some(2));
    assert_eq!(find(h, b"\x00\x00\x00"), None);
}

#[test]
fn random_blob_hit_middle() {
    // Plant needle in the middle of 32 KiB random data
    let mut rng = StdRng::seed_from_u64(0xC0FFEE);
    let mut blob = vec![0u8; 32 * 1024];
    rng.fill(blob.as_mut_slice());

    let needle = b"simdgrep-needle";
    let pos = blob.len() / 2 - needle.len() / 2;
    blob[pos..pos + needle.len()].copy_from_slice(needle);

    assert_eq!(find(&blob, needle), Some(pos));
}

#[test]
fn random_blob_no_hit() {
    let mut rng = StdRng::seed_from_u64(0xBAD5EED);
    let mut blob = vec![0u8; 64 * 1024];
    rng.fill(blob.as_mut_slice());

    // Very unlikely to appear by chance, but using long string to minimize collision probability
    let needle = b"this-needle-should-not-appear-here-very-unlikely-xxxxxxxx";
    assert_eq!(find(&blob, needle), None);
}

#[test]
fn long_pattern_edge() {
    let h = vec![b'x'; 1 << 20]; // 1MiB of 'x'
    let n = vec![b'x'; 64];
    // First match should be at 0
    assert_eq!(find(&h, &n), Some(0));
}
