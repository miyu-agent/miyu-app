use super::*;

#[test]
fn every_bundled_png_decodes_to_rgba() {
    for (name, bytes) in [
        ("tray", TRAY),
        ("stop", STOP),
        ("start", START),
        ("restart", RESTART),
        ("quit", QUIT),
    ] {
        let image = decode(bytes).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(
            image.pixels.len(),
            (image.width * image.height * 4) as usize,
            "{name}"
        );
        assert!(image.pixels.chunks(4).any(|p| p[3] > 0), "{name} 全透明");
    }
}

#[test]
fn the_menu_icons_are_the_menu_size() {
    for bytes in [STOP, START, RESTART, QUIT] {
        let image = decode(bytes).unwrap();
        assert_eq!((image.width, image.height), (MENU_SIZE, MENU_SIZE));
    }
}

#[test]
fn not_a_png_is_an_error() {
    assert!(decode(b"not a png").is_err());
}

#[test]
fn a_dot_is_solid_in_the_middle_and_clear_at_the_corner() {
    let image = dot([1, 2, 3], 32);
    assert_eq!((image.width, image.height), (32, 32));
    assert_eq!(image.pixels.len(), 32 * 32 * 4);
    let at = |x: usize, y: usize| &image.pixels[(y * 32 + x) * 4..(y * 32 + x) * 4 + 4];
    assert_eq!(at(16, 16), [1, 2, 3, 255]);
    assert_eq!(at(0, 0)[3], 0);
    assert_eq!(at(31, 31)[3], 0);
}
