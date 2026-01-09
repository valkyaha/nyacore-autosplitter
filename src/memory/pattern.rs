//! Pattern scanning utilities for memory signature matching

/// Parse a pattern string into bytes with wildcards
///
/// Pattern format: "48 8B 05 ?? ?? ?? ?? 48 85 C0"
/// - Regular bytes are hex values (e.g., "48", "8B")
/// - Wildcards are "??" for any byte
pub fn parse_pattern(pattern: &str) -> Vec<Option<u8>> {
    pattern
        .split_whitespace()
        .map(|s| {
            if s == "??" || s == "?" {
                None
            } else {
                u8::from_str_radix(s, 16).ok()
            }
        })
        .collect()
}

/// Scan memory for a pattern
///
/// Returns the address of the first match, or None if not found
pub fn scan_pattern(
    reader: &dyn super::MemoryReader,
    base: usize,
    size: usize,
    pattern: &[Option<u8>],
) -> Option<usize> {
    if pattern.is_empty() {
        return None;
    }

    // Read the entire region
    let data = reader.read_bytes(base, size)?;

    // Search for the pattern
    'outer: for i in 0..data.len().saturating_sub(pattern.len()) {
        for (j, &expected) in pattern.iter().enumerate() {
            if let Some(byte) = expected {
                if data[i + j] != byte {
                    continue 'outer;
                }
            }
        }
        return Some(base + i);
    }

    None
}

/// Extract a relative address from a pattern match
///
/// Many patterns contain relative addresses (RIP-relative on x64).
/// This helper extracts the i32 offset and calculates the absolute address.
pub fn extract_relative_address(
    reader: &dyn super::MemoryReader,
    instruction_address: usize,
    offset_position: usize,
    instruction_length: usize,
) -> Option<usize> {
    let offset_addr = instruction_address + offset_position;
    let offset = reader.read_bytes(offset_addr, 4)?;
    let relative_offset = i32::from_le_bytes([offset[0], offset[1], offset[2], offset[3]]);

    // Calculate absolute address: instruction_address + instruction_length + relative_offset
    let absolute = (instruction_address as i64) + (instruction_length as i64) + (relative_offset as i64);
    Some(absolute as usize)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_pattern_basic() {
        let pattern = parse_pattern("48 8B 05");
        assert_eq!(pattern, vec![Some(0x48), Some(0x8B), Some(0x05)]);
    }

    #[test]
    fn test_parse_pattern_with_wildcards() {
        let pattern = parse_pattern("48 8B ?? ?? 00");
        assert_eq!(
            pattern,
            vec![Some(0x48), Some(0x8B), None, None, Some(0x00)]
        );
    }

    #[test]
    fn test_parse_pattern_empty() {
        let pattern = parse_pattern("");
        assert!(pattern.is_empty());
    }

    struct MockReader {
        data: Vec<u8>,
        base: usize,
    }

    impl super::super::MemoryReader for MockReader {
        fn read_bytes(&self, address: usize, size: usize) -> Option<Vec<u8>> {
            let offset = address.checked_sub(self.base)?;
            if offset + size <= self.data.len() {
                Some(self.data[offset..offset + size].to_vec())
            } else {
                None
            }
        }
    }

    #[test]
    fn test_scan_pattern_found() {
        let reader = MockReader {
            data: vec![0x00, 0x48, 0x8B, 0x05, 0x12, 0x34, 0x00, 0x00],
            base: 0x1000,
        };

        let pattern = parse_pattern("48 8B 05");
        let result = scan_pattern(&reader, 0x1000, 8, &pattern);
        assert_eq!(result, Some(0x1001));
    }

    #[test]
    fn test_scan_pattern_with_wildcard() {
        let reader = MockReader {
            data: vec![0x00, 0x48, 0x8B, 0xFF, 0x12, 0x34, 0x00, 0x00],
            base: 0x1000,
        };

        let pattern = parse_pattern("48 8B ?? 12");
        let result = scan_pattern(&reader, 0x1000, 8, &pattern);
        assert_eq!(result, Some(0x1001));
    }

    #[test]
    fn test_scan_pattern_not_found() {
        let reader = MockReader {
            data: vec![0x00, 0x00, 0x00, 0x00],
            base: 0x1000,
        };

        let pattern = parse_pattern("48 8B 05");
        let result = scan_pattern(&reader, 0x1000, 4, &pattern);
        assert_eq!(result, None);
    }
}
