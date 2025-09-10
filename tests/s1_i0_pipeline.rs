use std::io::Cursor;

use simd_grep::engine::{GrepEngine, GrepOptions, MatchSink, VecSink};

#[test]
fn finds_matches_within_single_chunk() {
    let data = b"xxx-NEEDLE-yyy-NEEDLE-zzz".to_vec();
    let mut reader = Cursor::new(data);
    let opts = GrepOptions { chunk_bytes: 8, ..Default::default() };
    let eng = GrepEngine::new_literal(b"NEEDLE", opts);

    let mut sink = VecSink::default();
    eng.search(&mut reader, &mut sink).unwrap();

    assert_eq!(sink.offs, vec![4, 15]); // positions of "NEEDLE"
    assert_eq!(sink.lens, vec![6, 6]);
}

#[test]
fn finds_boundary_crossing_match_due_to_overlap() {
    // Arrange data so that "NEEDLE" straddles the chunk boundary.
    // Chunk size is small to force multiple chunks: overlap = needle.len()-1 = 5.
    let payload = b"AAAAANEEE".to_vec(); // prefix
    let mid = b"DLEBBBBB".to_vec();      // carry across boundary
    let mut buf = Vec::new();
    buf.extend_from_slice(&payload);
    buf.extend_from_slice(&mid);

    let mut reader = Cursor::new(buf);
    let opts = GrepOptions { chunk_bytes: 9, ..Default::default() }; // force split near "NEE|DLE"
    let eng = GrepEngine::new_literal(b"NEEDLE", opts);

    let mut sink = VecSink::default();
    eng.search(&mut reader, &mut sink).unwrap();

    // The "NEEDLE" starts at 5 in the global stream.
    assert_eq!(sink.offs, vec![5]);
    assert_eq!(sink.lens, vec![6]);
}

#[derive(Default)]
struct CountingSink {
    n: u64,
}
impl MatchSink for CountingSink {
    fn on_match(&mut self, _off: u64, _len: u32, _line_no: u32, _file_id: u32) {
        self.n += 1;
    }
}

#[test]
fn reports_all_overlapping_occurrences() {
    // "aaaaa" contains "aaa" at positions 0,1,2 -> 3 matches.
    let mut reader = Cursor::new(b"aaaaa".to_vec());
    let opts = GrepOptions { chunk_bytes: 3, ..Default::default() };
    let eng = GrepEngine::new_literal(b"aaa", opts);

    let mut sink = CountingSink::default();
    eng.search(&mut reader, &mut sink).unwrap();

    assert_eq!(sink.n, 3);
}

#[test]
fn empty_input_and_empty_needle_behavior() {
    // Empty input and non-empty needle -> no matches.
    let mut r1 = Cursor::new(Vec::<u8>::new());
    let eng = GrepEngine::new_literal(b"abc", GrepOptions::default());
    let mut s1 = VecSink::default();
    eng.search(&mut r1, &mut s1).unwrap();
    assert!(s1.offs.is_empty());

    // Empty needle -> report a single match at offset 0 (engine convention).
    let mut r2 = Cursor::new(b"xyz".to_vec());
    let eng2 = GrepEngine::new_literal(b"", GrepOptions::default());
    let mut s2 = VecSink::default();
    eng2.search(&mut r2, &mut s2).unwrap();
    assert_eq!(s2.offs, vec![0]);
}
