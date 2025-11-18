//! Configuration for content addressing functionality.

use serde::{Deserialize, Serialize};

use crate::{DEFAULT_CHUNK_SIZE, MAX_CHUNK_SIZE, MIN_CHUNK_SIZE};

/// Configuration for content addressing operations.
///
/// # Example
///
/// ```rust
/// use codio_content::config::ContentConfig;
///
/// let config = ContentConfig::default();
/// assert_eq!(config.chunk_size, 1024 * 1024); // 1MB
/// assert!(!config.enable_compression);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentConfig {
    /// Size of each content chunk in bytes.
    ///
    /// Default: 1MB (1024 * 1024 bytes)
    /// Range: 1KB to 10MB
    pub chunk_size: usize,

    /// Whether to enable compression for content.
    ///
    /// Default: false (Phase 1 - no compression)
    /// Future: Will support gzip/brotli compression
    pub enable_compression: bool,

    /// Whether to enable parallel chunking.
    ///
    /// Default: false (single-threaded for Phase 1)
    /// Future: Will support parallel processing for large files
    pub enable_parallel: bool,

    /// Maximum number of chunks to process in parallel.
    ///
    /// Only applies when enable_parallel is true.
    /// Default: Number of CPU cores
    pub max_parallel_chunks: usize,
}

impl Default for ContentConfig {
    fn default() -> Self {
        Self {
            chunk_size: DEFAULT_CHUNK_SIZE,
            enable_compression: false,
            enable_parallel: false,
            max_parallel_chunks: num_cpus::get(),
        }
    }
}

impl ContentConfig {
    /// Creates a new ContentConfig with custom chunk size.
    ///
    /// # Panics
    ///
    /// Panics if chunk_size is outside the valid range (1KB to 10MB).
    ///
    /// # Example
    ///
    /// ```rust
    /// use codio_content::config::ContentConfig;
    ///
    /// let config = ContentConfig::new(512 * 1024); // 512KB chunks
    /// assert_eq!(config.chunk_size, 512 * 1024);
    /// ```
    pub fn new(chunk_size: usize) -> Self {
        assert!(
            chunk_size >= MIN_CHUNK_SIZE,
            "Chunk size must be at least {} bytes",
            MIN_CHUNK_SIZE
        );
        assert!(
            chunk_size <= MAX_CHUNK_SIZE,
            "Chunk size must be at most {} bytes",
            MAX_CHUNK_SIZE
        );

        Self {
            chunk_size,
            ..Default::default()
        }
    }

    /// Creates a ContentConfig with compression enabled.
    ///
    /// # Example
    ///
    /// ```rust
    /// use codio_content::config::ContentConfig;
    ///
    /// let config = ContentConfig::with_compression(1024 * 1024);
    /// assert!(config.enable_compression);
    /// ```
    pub fn with_compression(chunk_size: usize) -> Self {
        Self {
            chunk_size,
            enable_compression: true,
            ..Default::default()
        }
    }

    /// Creates a ContentConfig with parallel processing enabled.
    ///
    /// # Example
    ///
    /// ```rust
    /// use codio_content::config::ContentConfig;
    ///
    /// let config = ContentConfig::with_parallel(1024 * 1024, 4);
    /// assert!(config.enable_parallel);
    /// assert_eq!(config.max_parallel_chunks, 4);
    /// ```
    pub fn with_parallel(chunk_size: usize, max_parallel_chunks: usize) -> Self {
        Self {
            chunk_size,
            enable_parallel: true,
            max_parallel_chunks,
            ..Default::default()
        }
    }

    /// Validates the configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - chunk_size is outside valid range
    /// - max_parallel_chunks is 0
    ///
    /// # Example
    ///
    /// ```rust
    /// use codio_content::config::ContentConfig;
    ///
    /// let config = ContentConfig::default();
    /// assert!(config.validate().is_ok());
    /// ```
    pub fn validate(&self) -> Result<(), String> {
        if self.chunk_size < MIN_CHUNK_SIZE {
            return Err(format!(
                "Chunk size {} is below minimum {}",
                self.chunk_size, MIN_CHUNK_SIZE
            ));
        }

        if self.chunk_size > MAX_CHUNK_SIZE {
            return Err(format!(
                "Chunk size {} exceeds maximum {}",
                self.chunk_size, MAX_CHUNK_SIZE
            ));
        }

        if self.enable_parallel && self.max_parallel_chunks == 0 {
            return Err("max_parallel_chunks must be greater than 0".to_string());
        }

        Ok(())
    }

    /// Returns the estimated number of chunks for content of the given size.
    ///
    /// # Example
    ///
    /// ```rust
    /// use codio_content::config::ContentConfig;
    ///
    /// let config = ContentConfig::new(1024 * 1024);
    /// assert_eq!(config.estimate_chunks(5 * 1024 * 1024), 5);
    /// ```
    pub fn estimate_chunks(&self, content_size: usize) -> usize {
        if content_size == 0 {
            return 0;
        }
        content_size.div_ceil(self.chunk_size)
    }
}

// Helper function to get number of CPUs (stub for now)
mod num_cpus {
    pub fn get() -> usize {
        // Default to 4 cores if we can't detect
        std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ContentConfig::default();
        assert_eq!(config.chunk_size, DEFAULT_CHUNK_SIZE);
        assert!(!config.enable_compression);
        assert!(!config.enable_parallel);
    }

    #[test]
    fn test_new_config() {
        let config = ContentConfig::new(512 * 1024);
        assert_eq!(config.chunk_size, 512 * 1024);
    }

    #[test]
    #[should_panic]
    fn test_invalid_chunk_size_too_small() {
        ContentConfig::new(512); // Below MIN_CHUNK_SIZE
    }

    #[test]
    #[should_panic]
    fn test_invalid_chunk_size_too_large() {
        ContentConfig::new(20 * 1024 * 1024); // Above MAX_CHUNK_SIZE
    }

    #[test]
    fn test_with_compression() {
        let config = ContentConfig::with_compression(1024 * 1024);
        assert!(config.enable_compression);
    }

    #[test]
    fn test_with_parallel() {
        let config = ContentConfig::with_parallel(1024 * 1024, 8);
        assert!(config.enable_parallel);
        assert_eq!(config.max_parallel_chunks, 8);
    }

    #[test]
    fn test_validate() {
        let config = ContentConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_estimate_chunks() {
        let config = ContentConfig::new(1024 * 1024);
        assert_eq!(config.estimate_chunks(5 * 1024 * 1024), 5);
        assert_eq!(config.estimate_chunks(5 * 1024 * 1024 + 512), 6);
        assert_eq!(config.estimate_chunks(0), 0);
    }
}
