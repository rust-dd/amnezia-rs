//! Decoder for the RPG Maker 2000 `XYZ` image format.
//!
//! An `XYZ` file is the ASCII tag `XYZ1`, a little-endian `u16` width and
//! `u16` height, then a zlib stream that inflates to a 256-colour RGB palette
//! (768 bytes) followed by one palette index per pixel. This crate is
//! dev-time tooling for the asset converter and is never linked into the
//! shipped game binary.

use flate2::read::ZlibDecoder;
use std::io::Read;

const HEADER_LEN: usize = 8;
const PALETTE_LEN: usize = 768;

/// A decoded XYZ image as a tightly packed RGBA8 buffer.
pub struct XyzImage {
    pub width: u16,
    pub height: u16,
    pub rgba: Vec<u8>,
}

/// Errors returned while decoding an XYZ byte slice.
#[derive(Debug, thiserror::Error)]
pub enum XyzError {
    #[error("not an XYZ file: bad magic")]
    BadMagic,
    #[error("truncated header: need at least {HEADER_LEN} bytes")]
    TruncatedHeader,
    #[error("zlib inflate failed: {0}")]
    Inflate(#[from] std::io::Error),
    #[error("payload too small: expected {expected} bytes, got {got}")]
    PayloadTooSmall { expected: usize, got: usize },
}

/// Decode an XYZ image. When `transparent_index0` is true, every pixel whose
/// palette index is 0 is written with an alpha of 0; otherwise all pixels are
/// fully opaque.
pub fn decode(bytes: &[u8], transparent_index0: bool) -> Result<XyzImage, XyzError> {
    if bytes.len() < HEADER_LEN {
        return Err(XyzError::TruncatedHeader);
    }
    if &bytes[0..4] != b"XYZ1" {
        return Err(XyzError::BadMagic);
    }
    let width = u16::from_le_bytes([bytes[4], bytes[5]]);
    let height = u16::from_le_bytes([bytes[6], bytes[7]]);

    let mut inflated = Vec::new();
    ZlibDecoder::new(&bytes[HEADER_LEN..]).read_to_end(&mut inflated)?;

    let pixels = width as usize * height as usize;
    let expected = PALETTE_LEN + pixels;
    if inflated.len() < expected {
        return Err(XyzError::PayloadTooSmall {
            expected,
            got: inflated.len(),
        });
    }
    let (palette, indices) = inflated.split_at(PALETTE_LEN);

    let mut rgba = Vec::with_capacity(pixels * 4);
    for &index in &indices[..pixels] {
        let base = index as usize * 3;
        let alpha = if transparent_index0 && index == 0 {
            0
        } else {
            255
        };
        rgba.extend_from_slice(&[palette[base], palette[base + 1], palette[base + 2], alpha]);
    }

    Ok(XyzImage {
        width,
        height,
        rgba,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::Compression;
    use flate2::write::ZlibEncoder;
    use std::io::Write;

    fn make_xyz(width: u16, height: u16, palette: &[u8; 768], indices: &[u8]) -> Vec<u8> {
        let mut raw = palette.to_vec();
        raw.extend_from_slice(indices);
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&raw).unwrap();
        let compressed = encoder.finish().unwrap();
        let mut file = b"XYZ1".to_vec();
        file.extend_from_slice(&width.to_le_bytes());
        file.extend_from_slice(&height.to_le_bytes());
        file.extend_from_slice(&compressed);
        file
    }

    #[test]
    fn decodes_pixels_opaque() {
        let mut palette = [0u8; 768];
        palette[0..3].copy_from_slice(&[10, 20, 30]);
        palette[3..6].copy_from_slice(&[40, 50, 60]);
        let file = make_xyz(2, 1, &palette, &[0, 1]);
        let image = decode(&file, false).unwrap();
        assert_eq!((image.width, image.height), (2, 1));
        assert_eq!(image.rgba, vec![10, 20, 30, 255, 40, 50, 60, 255]);
    }

    #[test]
    fn maps_index_zero_to_transparent_when_requested() {
        let mut palette = [0u8; 768];
        palette[0..3].copy_from_slice(&[10, 20, 30]);
        palette[3..6].copy_from_slice(&[40, 50, 60]);
        let file = make_xyz(2, 1, &palette, &[0, 1]);
        let image = decode(&file, true).unwrap();
        assert_eq!(image.rgba, vec![10, 20, 30, 0, 40, 50, 60, 255]);
    }

    #[test]
    fn rejects_bad_magic() {
        assert!(matches!(
            decode(b"NOPE\0\0\0\0", false),
            Err(XyzError::BadMagic)
        ));
    }

    #[test]
    fn rejects_truncated_header() {
        assert!(matches!(
            decode(b"XYZ", false),
            Err(XyzError::TruncatedHeader)
        ));
    }

    #[test]
    fn rejects_payload_too_small() {
        let palette = [0u8; 768];
        let file = make_xyz(2, 1, &palette, &[]);
        assert!(matches!(
            decode(&file, false),
            Err(XyzError::PayloadTooSmall {
                expected: 770,
                got: 768
            })
        ));
    }

    #[test]
    fn rejects_non_zlib_payload() {
        let mut bytes = b"XYZ1".to_vec();
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&[0xFF, 0xFF, 0xFF, 0xFF]);
        assert!(matches!(decode(&bytes, false), Err(XyzError::Inflate(_))));
    }
}
