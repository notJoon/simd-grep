use std::io;

use memchr::memmem;
use crate::io::chunker::Chunker;

bitflags::bitflags! {
    /// Flags to control grep engine behavior.
    ///
    /// These flags modify how the grep engine processes matches and outputs results.
    /// Currently kept minimal at this stage.
    #[derive(Clone, Debug)]
    pub struct GrepFlags: u32 {
        /// Only count matches without reporting positions.
        const COUNT_ONLY = 1 << 0;
        /// Include line numbers in match reports.
        const LINE_NUMBER = 1 << 1; // TODO: placeholder for now
    }
}

/// Configuration options for the grep engine.
///
/// This struct encapsulates all the settings that control how the engine
/// searches for patterns, including chunk size, flags, and file identification.
#[derive(Clone, Debug)]
pub struct GrepOptions {
    pub chunk_bytes: usize,
    pub flags: GrepFlags,
    pub file_id: u32,
}

impl Default for GrepOptions {
    fn default() -> Self {
        Self {
            chunk_bytes: 8 * 1024 * 1024,
            flags: GrepFlags::empty(),
            file_id: 0,
        }
    }
}

/// A trait for receiving match notifications from the grep engine.
///
/// Implementations of this trait handle matches found during searches,
/// allowing customizable processing of search results.
pub trait MatchSink {
    /// Reports a single match found by the engine.
    ///
    /// # Arguments
    ///
    /// * `off` - Global byte offset within the entire file/stream
    /// * `len` - Match length (needle length)
    /// * `line_no` - 1-based line number (0 for "unknown" until line indexer is implemented)
    /// * `file_id` - Caller-provided file identifier
    fn on_match(&mut self, off: u64, len: u32, line_no: u32, file_id: u32);
}

/// An input source with `io::Read` semantic.
///
/// This trait is automatically implemented for all types that implement `io::Read`.
pub trait Source: io::Read {}
impl<T: io::Read> Source for T {}

/// The main grep engine that performs pattern searches.
///
/// This struct holds the compiled pattern and search options,
/// providing methods to search through various input sources.
pub struct GrepEngine<'p> {
    needle: &'p [u8],
    opts: GrepOptions,
}

impl<'p> GrepEngine<'p> {
    /// Creates a new engine for searching a single literal pattern.
    ///
    /// # Arguments
    ///
    /// * `needle` - The literal byte pattern to search for
    /// * `opts` - Configuration options for the search
    pub fn new_literal(needle: &'p [u8], opts: GrepOptions) -> Self {
        Self { needle, opts }
    }

    /// Runs the search pipeline on a `Source`, reporting all matches to the provided sink.
    ///
    /// # Arguments
    ///
    /// * `reader` - The input source to search through
    /// * `sink` - The sink that will receive match notifications
    ///
    /// # Returns
    ///
    /// * `Ok(())` - On successful completion
    /// * `Err(e)` - On I/O errors
    ///
    /// # Notes
    ///
    /// - Currently uses baseline `memmem::find` repeatedly inside each chunk
    /// - Overlap is handled in `Chunker`, so cross-boundary matches are found exactly once
    /// - Line numbers are reported as 0 (placeholder)
    pub fn search<R: Source>(&self, reader: &mut R, sink: &mut dyn MatchSink) -> io::Result<()> {
        // For overlap we need "needle.len() - 1" bytes from the previous chunk.
        let overlap = self.needle.len().saturating_sub(1);
        let mut chunker = Chunker::new(reader, self.opts.chunk_bytes, overlap);

        let mut total_count: u64 = 0;
        let nlen = self.needle.len() as u32;

        while let Some((global_base, chunk)) = chunker.next_chunk()? {
            if self.needle.is_empty() {
                // Empty needle convention: match at every position is nonsensical for grep.
                // We follow our S0 API rules and report a single hit at the start of the stream.
                if global_base == 0 {
                    sink.on_match(0, 0, 0, self.opts.file_id);
                    total_count += 1;
                }
                break;
            }

            // Repeatedly find all matches within the current chunk.
            // Important: Chunker ensures that every *new* byte range (excluding the previous
            // overlap except at the leading edge) is unique, so reporting here is safe.
            let mut search_off = 0usize;
            while let Some(rel) = memmem::find(&chunk[search_off..], self.needle) {
                let pos = search_off + rel;
                let global_off = (global_base + pos as u64) as u64;
                sink.on_match(global_off, nlen, 0, self.opts.file_id);
                total_count += 1;

                // Move past this match to find subsequent occurrences (including overlaps).
                search_off = pos + 1;
                if search_off >= chunk.len() {
                    break;
                }
            }
        }

        if self.opts.flags.contains(GrepFlags::COUNT_ONLY) {
            // A "count only" sink could be specialized; for now we expect the sink
            // implementation to handle "counting" if desired.
            let _ = total_count;
        }

        Ok(())
    }
}

/// A simple sink implementation that collects match data into vectors.
///
/// This sink is primarily used in tests and examples, storing all match
/// information for later inspection and verification.
#[derive(Default, Debug)]
pub struct VecSink {
    pub offs: Vec<u64>,
    pub lens: Vec<u32>,
    pub file_ids: Vec<u32>,
}
impl MatchSink for VecSink {
    fn on_match(&mut self, off: u64, len: u32, _line_no: u32, file_id: u32) {
        self.offs.push(off);
        self.lens.push(len);
        self.file_ids.push(file_id);
    }
}
