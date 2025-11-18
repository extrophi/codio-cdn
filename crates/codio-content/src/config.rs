//! Configuration for content addressing operations.
//!
//! This module provides configuration options for chunking, compression,
//! and other content addressing features.

use serde::{Deserialize, Serialize};

/// Configuration for content addressing operations.
///
/// Controls how content is chunked, hashed, and processed.
///
/// # Example
///
/// ```rust
/// use codio_content::ContentConfig;
///
/// // Use default configuration (1MB chunks, no compression)
/// let config = ContentConfig::default();
/// assert_eq!(config.chunk_size, 1024 * 1024);
/// assert_eq!(config.enable_compression, false);
///
/// // Custom configuration
/// let custom_config = ContentConfig {
///     chunk_size: 512 * 1024, // 512KB chunks
///     enable_compression: true,
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContentConfig {
    /// Size of each content chunk in bytes.
    ///
    /// Default: 1MB (1024 * 1024 bytes)
    ///
    /// Smaller chunks:
    /// - Enable finer-grained deduplication
    /// - Increase overhead (more CIDs to track)
    /// - Better for parallel transfer of small files
    ///
    /// Larger chunks:
    /// - Reduce overhead
    /// - Better for large file transfers
    /// - Less deduplication granularity
    pub chunk_size: usize,

    /// Enable compression for content chunks.
    ///
    /// Default: false (Phase 1 - compression disabled)
    ///
    /// When enabled (future phase):
    /// - Reduces storage and bandwidth
    /// - Adds CPU overhead for compression/decompression
    /// - CIDs are computed on compressed data
    pub enable_compression: bool,
}

impl Default for ContentConfig {
    /// Create default configuration.
    ///
    /// Defaults:
    /// - chunk_size: 1MB (1024 * 1024 bytes)
    /// - enable_compression: false
    fn default() -> Self {
        Self {
            chunk_size: 1024 * 1024, // 1MB
            enable_compression: false,
        }
    }
}

impl ContentConfig {
    /// Create a new configuration with custom chunk size.
    ///
    /// # Arguments
    ///
    /// * `chunk_size` - Size of each chunk in bytes
    ///
    /// # Panics
    ///
    /// Panics if chunk_size is 0.
    ///
    /// # Example
    ///
    /// ```rust
    /// use codio_content::ContentConfig;
    ///
    /// let config = ContentConfig::new(512 * 1024); // 512KB chunks
    /// assert_eq!(config.chunk_size, 512 * 1024);
    /// ```
    pub fn new(chunk_size: usize) -> Self {
        assert!(chunk_size > 0, "Chunk size must be greater than 0");
        Self {
            chunk_size,
            enable_compression: false,
        }
    }

    /// Create configuration with compression enabled.
    ///
    /// # Arguments
    ///
    /// * `chunk_size` - Size of each chunk in bytes
    ///
    /// # Example
    ///
    /// ```rust
    /// use codio_content::ContentConfig;
    ///
    /// let config = ContentConfig::with_compression(1024 * 1024);
    /// assert!(config.enable_compression);
    /// ```
    pub fn with_compression(chunk_size: usize) -> Self {
        assert!(chunk_size > 0, "Chunk size must be greater than 0");
        Self {
            chunk_size,
            enable_compression: true,
        }
    }

    /// Validate the configuration.
    ///
    /// Returns true if the configuration is valid.
    ///
    /// Validation rules:
    /// - chunk_size must be > 0
    /// - chunk_size should be >= 1KB (recommended minimum)
    /// - chunk_size should be <= 100MB (recommended maximum)
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.chunk_size == 0 {
            return Err(ConfigError::InvalidChunkSize(
                "Chunk size must be greater than 0".to_string(),
            ));
        }

        if self.chunk_size < 1024 {
            return Err(ConfigError::InvalidChunkSize(
                "Chunk size below 1KB is not recommended".to_string(),
            ));
        }

        if self.chunk_size > 100 * 1024 * 1024 {
            return Err(ConfigError::InvalidChunkSize(
                "Chunk size above 100MB is not recommended".to_string(),
            ));
        }

        Ok(())
    }
}

/// Errors that can occur during configuration.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// Invalid chunk size
    #[error("Invalid chunk size: {0}")]
    InvalidChunkSize(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ContentConfig::default();
        assert_eq!(config.chunk_size, 1024 * 1024);
        assert_eq!(config.enable_compression, false);
    }

    #[test]
    fn test_new_config() {
        let config = ContentConfig::new(512 * 1024);
        assert_eq!(config.chunk_size, 512 * 1024);
        assert_eq!(config.enable_compression, false);
    }

    #[test]
    fn test_with_compression() {
        let config = ContentConfig::with_compression(1024 * 1024);
        assert_eq!(config.chunk_size, 1024 * 1024);
        assert!(config.enable_compression);
    }

    #[test]
    fn test_validate_valid_config() {
        let config = ContentConfig::new(1024 * 1024);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_chunk_size_too_small() {
        let config = ContentConfig {
            chunk_size: 512,
            enable_compression: false,
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_chunk_size_too_large() {
        let config = ContentConfig {
            chunk_size: 200 * 1024 * 1024,
            enable_compression: false,
        };
        assert!(config.validate().is_err());
    }

    #[test]
    #[should_panic(expected = "Chunk size must be greater than 0")]
    fn test_new_zero_chunk_size() {
        ContentConfig::new(0);
    }

    #[test]
    fn test_config_equality() {
        let config1 = ContentConfig::new(1024 * 1024);
        let config2 = ContentConfig::new(1024 * 1024);
        assert_eq!(config1, config2);
    }

    #[test]
    fn test_config_serialization() {
        let config = ContentConfig::with_compression(2 * 1024 * 1024);
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: ContentConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config, deserialized);
    }
}
