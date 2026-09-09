use flate2::Compression;
use flate2::write::ZlibEncoder;
use std::io::Write;
use std::path::Path;

#[test]
fn retains_original_png_panoramas_without_reencoding() {
    let tmp = Path::new(env!("CARGO_TARGET_TMPDIR")).join("original_png_panoramas");
    let input = tmp.join("in");
    let output = tmp.join("out");
    std::fs::create_dir_all(input.join("Panorama")).unwrap();
    let original = Path::new(env!("CARGO_MANIFEST_DIR")).join("../original/Panorama/Ground.png");
    std::fs::copy(&original, input.join("Panorama/Ground.png")).unwrap();
    assert_eq!(
        amnezia_convert::convert_graphics(&input, &output).unwrap(),
        1
    );
    assert_eq!(
        std::fs::read(original).unwrap(),
        std::fs::read(output.join("graphics/Panorama/Ground.png")).unwrap()
    );
}

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
fn picture_png_preserves_key_color_rgb_for_opaque_show_commands() {
    let tmp = Path::new(env!("CARGO_TARGET_TMPDIR")).join("picture_palette_color");
    let input = tmp.join("in");
    let output = tmp.join("out");
    std::fs::create_dir_all(input.join("Picture")).unwrap();
    let mut palette = [0; 768];
    palette[..6].copy_from_slice(&[10, 20, 30, 40, 50, 60]);
    std::fs::write(
        input.join("Picture/Test.xyz"),
        make_xyz(2, 1, &palette, &[0, 1]),
    )
    .unwrap();
    assert_eq!(
        amnezia_convert::convert_graphics(&input, &output).unwrap(),
        1
    );
    let image = image::open(output.join("graphics/Picture/Test.png"))
        .unwrap()
        .to_rgba8();
    assert_eq!(image.get_pixel(0, 0).0, [10, 20, 30, 0]);
    assert_eq!(image.get_pixel(1, 0).0, [40, 50, 60, 255]);
}

#[test]
fn converts_chipset_with_transparency() {
    let tmp = Path::new(env!("CARGO_TARGET_TMPDIR")).join("converts_chipset");
    let input = tmp.join("in");
    let output = tmp.join("out");
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(input.join("ChipSet")).unwrap();

    let mut palette = [0u8; 768];
    palette[0..3].copy_from_slice(&[10, 20, 30]);
    palette[3..6].copy_from_slice(&[40, 50, 60]);
    let bytes = make_xyz(2, 1, &palette, &[0, 1]);
    std::fs::write(input.join("ChipSet/Test.xyz"), bytes).unwrap();

    let count = amnezia_convert::convert_graphics(&input, &output).unwrap();
    assert_eq!(count, 1);

    let png = output.join("graphics/ChipSet/Test.png");
    assert!(png.exists(), "expected {}", png.display());
    let image = image::open(&png).unwrap().to_rgba8();
    assert_eq!(image.dimensions(), (2, 1));
    assert_eq!(image.get_pixel(0, 0).0, [10, 20, 30, 0]);
    assert_eq!(image.get_pixel(1, 0).0, [40, 50, 60, 255]);
}

#[test]
fn keeps_opaque_category_opaque() {
    let tmp = Path::new(env!("CARGO_TARGET_TMPDIR")).join("keeps_opaque_category");
    let input = tmp.join("in");
    let output = tmp.join("out");
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(input.join("Backdrop")).unwrap();

    let mut palette = [0u8; 768];
    palette[0..3].copy_from_slice(&[70, 80, 90]);
    let bytes = make_xyz(1, 1, &palette, &[0]);
    std::fs::write(input.join("Backdrop/Test.xyz"), bytes).unwrap();

    let count = amnezia_convert::convert_graphics(&input, &output).unwrap();
    assert_eq!(count, 1);

    let png = output.join("graphics/Backdrop/Test.png");
    let image = image::open(&png).unwrap().to_rgba8();
    assert_eq!(image.get_pixel(0, 0).0, [70, 80, 90, 255]);
}

#[test]
fn errors_on_missing_input() {
    let tmp = Path::new(env!("CARGO_TARGET_TMPDIR")).join("errors_on_missing_input");
    let input = tmp.join("does-not-exist");
    let output = tmp.join("out");
    let _ = std::fs::remove_dir_all(&tmp);

    assert!(amnezia_convert::convert_graphics(&input, &output).is_err());
}
