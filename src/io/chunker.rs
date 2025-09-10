//! Chunked reader with overlap (S1).
//!
//! The chunker turns a streaming `Read` source into a sequence of
//! (global_offset, chunk_slice) pairs, preserving `overlap` bytes from the
//! previous chunk to ensure matches crossing boundaries are not missed.
//!
//! Termination rule (important):
//! - If at EOF no new bytes are read and only the carried overlap remains,
//!   we must NOT return another chunk (to avoid infinite loops). We end the
//!   iteration instead.
//!
//! Invariants:
//! - Overlap = max(needle.len() - 1, 0).
//! - Returned slices never exceed the valid data range in the internal buffer.
//! - `global_offset` always points to the beginning of the returned slice in
//!   the global stream.

use std::cmp;
use std::io::{self, Read};

/// A chunked reader that processes data in fixed-size chunks with overlap.
///
/// This struct manages reading from a stream in chunks while preserving overlap
/// bytes between consecutive chunks to ensure pattern matches that span chunk
/// boundaries are not missed.
pub struct Chunker<'a, R: Read> {
    reader: &'a mut R,
    /// Working buffer (capacity >= chunk_size + overlap).
    buf: Vec<u8>,
    /// Preferred chunk payload size (excluding overlap).
    chunk_size: usize,
    /// Number of bytes to carry from the previous tail.
    overlap: usize,
    /// Number of valid bytes currently in `buf` (prefix of the buffer).
    len: usize,
    /// Whether the underlying stream reached EOF.
    eof: bool,
    /// Global offset for the next returned chunk.
    next_global_off: u64,
}

impl<'a, R: Read> Chunker<'a, R> {
    /// Creates a new `Chunker` with the specified chunk size and overlap.
    ///
    /// # Arguments
    ///
    /// * `reader` - The source to read data from
    /// * `chunk_size` - Preferred size of each chunk (excluding overlap)
    /// * `overlap` - Number of bytes to preserve from the previous chunk
    ///
    /// # Notes
    ///
    /// The internal buffer capacity will be at least `chunk_size + overlap`,
    /// with a minimum of 4KB to ensure reasonable performance even with small chunk sizes.
    pub fn new(reader: &'a mut R, chunk_size: usize, overlap: usize) -> Self {
        // Ensure some minimum capacity so tiny chunk sizes still work.
        let cap = cmp::max(4 * 1024, chunk_size.saturating_add(overlap));
        Self {
            reader,
            buf: vec![0u8; cap],
            chunk_size,
            overlap,
            len: 0,
            eof: false,
            next_global_off: 0,
        }
    }

    /// Reads the next chunk from the stream.
    ///
    /// # Returns
    ///
    /// * `Ok(Some((global_offset, chunk_slice)))` - A tuple containing the global byte offset
    ///   and a slice of the chunk data when data is available
    /// * `Ok(None)` - When the stream is exhausted
    /// * `Err(e)` - On I/O errors
    ///
    /// # Behavior
    ///
    /// This method handles overlap by carrying bytes from the previous chunk to ensure
    /// matches spanning chunk boundaries are not missed. It prevents infinite loops by
    /// not returning chunks that contain only previously-seen overlap bytes.
    pub fn next_chunk(&mut self) -> io::Result<Option<(u64, &[u8])>> {
        // If we already signaled EOF and have no buffered data, we are done.
        if self.eof && self.len == 0 {
            return Ok(None);
        }

        // Carry tail bytes from the previous chunk to the front.
        let mut carry = 0usize;
        if self.len > 0 && self.overlap > 0 {
            carry = self.len.min(self.overlap);

            // Advance global offset by the number of newly-consumed bytes
            // from the last returned chunk (len - carry).
            let advanced = self.len - carry;
            self.next_global_off = self.next_global_off.saturating_add(advanced as u64);

            // Move the last `carry` bytes to the beginning of the buffer.
            if carry > 0 {
                let start = self.len - carry;
                self.buf.copy_within(start..self.len, 0);
            }
            // Now the valid prefix is exactly the carried bytes.
            self.len = carry;
        } else if self.len == 0 {
            // First read; global offset starts at 0.
            self.next_global_off = 0;
        }

        // Read up to `chunk_size` fresh bytes after the carried prefix.
        let mut filled = 0usize;
        while filled < self.chunk_size {
            let dst = &mut self.buf[self.len + filled..];
            if dst.is_empty() {
                break;
            }
            let n = self.reader.read(dst)?;
            if n == 0 {
                self.eof = true;
                break;
            }
            filled += n;
        }

        self.len += filled;

        // If there's no data at all (empty input), we are done.
        if self.len == 0 {
            return Ok(None);
        }

        // When at EOF and the current buffer only contains the carried overlap
        // (i.e., no new bytes were read), returning another chunk would repeat
        // the same slice forever. Stop the iteration.
        if self.eof && filled == 0 && self.len <= self.overlap && self.len > 0 {
            // Clear to make subsequent calls return None deterministically.
            self.len = 0;
            return Ok(None);
        }

        let base = self.next_global_off;
        let out = &self.buf[..self.len];
        Ok(Some((base, out)))
    }
}
