use memchr::memmem;

pub mod engine;
pub mod io;

/// Returns the index of the first occurrence of `needle` in `haystack`.
///
/// This function uses the efficient `memchr::memmem` implementation for searching.
/// An empty needle will match at position 0.
///
/// # Arguments
///
/// * `haystack` - The byte slice to search in
/// * `needle` - The byte pattern to search for
///
/// # Returns
///
/// * `Some(index)` - The byte index of the first match
/// * `None` - If no match is found
///
/// # Examples
///
/// ```rust
/// use simd_grep::find;
/// assert_eq!(find(b"hello", b""), Some(0));
/// assert_eq!(find(b"hello", b"ell"), Some(1));
/// assert_eq!(find(b"hello", b"xyz"), None);
/// ```
pub fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0)
    }
    memmem::find(haystack, needle)
}

/// Checks whether `needle` is contained within `haystack`.
///
/// This is a convenience function that returns a boolean instead of an index.
///
/// # Arguments
///
/// * `haystack` - The byte slice to search in
/// * `needle` - The byte pattern to search for
///
/// # Returns
///
/// * `true` - If the needle is found in the haystack
/// * `false` - If the needle is not found
///
/// # Examples
///
/// ```rust
/// use simd_grep::contains;
/// assert!(contains(b"hello world", b"world"));
/// assert!(!contains(b"hello", b"xyz"));
/// ```
#[inline]
pub fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    find(haystack, needle).is_some()
}
