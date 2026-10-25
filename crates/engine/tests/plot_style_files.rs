//! AutoCAD plot style table files (`.ctb`): `stylesmanager load` reads them as they are, and
//! OPEN finds a table a layout names next to the drawing (the hookup of `cadcraft_io::pia`).
//!
//! Added in the Otto fork of CADCraft (github.com/philipp-tch/cadcraft, 2026-10-25).

use cadcraft_engine::Session;
use cadcraft_engine::cmd::file::{IoHooks, set_io, set_plot_style_reader};
use serde_json::json;

fn hooks() {
    set_io(IoHooks {
        read: |b, name| cadcraft_io::read(b, name).map_err(|e| e.to_string()),
        write: |d, name| cadcraft_io::write(d, name).map_err(|e| e.to_string()),
        plot: None,
    });
    set_plot_style_reader(cadcraft_io::pia::read);
}

/// `data` as a zlib stream of stored (uncompressed) deflate blocks, so the test needs no
/// compressor.
fn zlib_stored(data: &[u8]) -> Vec<u8> {
    let mut out = vec![0x78, 0x01];
    let chunks: Vec<&[u8]> = data.chunks(0xFFFF).collect();
    for (i, c) in chunks.iter().enumerate() {
        out.push(u8::from(i + 1 == chunks.len()));
        let n = c.len() as u16;
        out.extend_from_slice(&n.to_le_bytes());
        out.extend_from_slice(&(!n).to_le_bytes());
        out.extend_from_slice(c);
    }
    let (mut a, mut b) = (1u32, 0u32);
    for &x in data {
        a = (a + u32::from(x)) % 65521;
        b = (b + a) % 65521;
    }
    out.extend_from_slice(&((b << 16) | a).to_be_bytes());
    out
}

/// A colour-dependent table file: colour 1 prints red at lineweight index 5, colour 3 blue at
/// index 18, every other colour keeps the object's colour and lineweight.
fn ctb() -> Vec<u8> {
    let mut text = String::from("description=\"Synthetic\naci_table_available=TRUE\ncustom_lineweight_display_units=0\nplot_style{\n");
    for i in 0..255 {
        let (color, lw) = match i {
            0 => ("-1006698496", 5),  // 0xC3FF0000, red
            2 => ("-1023409921", 18), // 0xC30000FF, blue
            _ => ("-1", 0),
        };
        text.push_str(&format!(
            " {i}{{\n  name=\"Color_{}\n  color={color}\n  mode_color={color}\n  color_policy=1\n  screen=100\n  linetype=31\n  lineweight={lw}\n  end_style=4\n  join_style=5\n }}\n",
            i + 1
        ));
    }
    text.push_str("}\ncustom_lineweight_table{\n");
    let weights = [
        0.0, 0.05, 0.09, 0.1, 0.13, 0.15, 0.18, 0.2, 0.25, 0.3, 0.35, 0.4, 0.45, 0.5, 0.53, 0.6, 0.65, 0.7, 0.8, 0.9, 1.0, 1.06, 1.2, 1.4, 1.58, 2.0,
        2.11,
    ];
    for (i, w) in weights.iter().enumerate() {
        text.push_str(&format!(" {i}={w}\n"));
    }
    text.push_str("}\n");
    let z = zlib_stored(text.as_bytes());
    let mut out = b"PIAFILEVERSION_2.0,CTBVER1,compress\r\npmzlibcodec".to_vec();
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&(text.len() as u32).to_le_bytes());
    out.extend_from_slice(&(z.len() as u32).to_le_bytes());
    out.extend_from_slice(&z);
    out
}

fn folder(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("cadcraft-ctb-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn stylesmanager_loads_an_autocad_ctb_file() {
    hooks();
    let dir = folder("load");
    let path = dir.join("Office.ctb");
    std::fs::write(&path, ctb()).unwrap();
    let mut s = Session::new();
    s.execute("stylesmanager", &json!({"action": "load", "path": path.to_string_lossy()})).unwrap();
    let d = s.doc().unwrap();
    let t = cadcraft_engine::doc::plot_style_table(d, "office.ctb").expect("the table is loaded under its file name");
    assert!(t.is_color_dependent());
    let red = t.by_color(1).unwrap();
    assert_eq!(red.color.map(|c| (c.0, c.1, c.2)), Some((255, 0, 0)));
    assert!(red.lineweight.is_some());
    let blue = t.by_color(3).unwrap();
    assert_eq!(blue.color.map(|c| (c.0, c.1, c.2)), Some((0, 0, 255)));
    assert!(blue.lineweight > red.lineweight);
    let other = t.by_color(2).unwrap();
    assert!(other.color.is_none() && other.lineweight.is_none());
    // A file that only starts like one is refused, not loaded.
    std::fs::write(dir.join("bad.ctb"), b"PIAFILEVERSION_2.0,CTBVER1,compress\r\npmzlibcodec\x01").unwrap();
    assert!(s.execute("stylesmanager", &json!({"action": "load", "path": dir.join("bad.ctb").to_string_lossy()})).is_err());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn open_finds_a_named_table_next_to_the_drawing_and_reports_a_missing_one() {
    hooks();
    let dir = folder("open");
    let dwg = dir.join("plan.dxf");
    let mut s = Session::new();
    let layouts: Vec<String> = s.doc().unwrap().layouts.iter().map(|l| l.name.clone()).collect();
    assert!(layouts.len() >= 2, "{layouts:?}");
    s.execute("pagesetup", &json!({"layout": layouts[0], "plotStyleTable": "Office.ctb"})).unwrap();
    s.execute("pagesetup", &json!({"layout": layouts[1], "plotStyleTable": "Missing.ctb"})).unwrap();
    s.execute("saveas", &json!({"path": dwg.to_string_lossy()})).unwrap();
    std::fs::write(dir.join("office.CTB"), ctb()).unwrap();

    let mut s = Session::new();
    s.execute("open", &json!({"path": dwg.to_string_lossy()})).unwrap();
    let d = s.doc().unwrap();
    assert!(d.plot_style_tables.iter().any(|t| t.name.eq_ignore_ascii_case("Office.ctb")), "found next to the drawing");
    assert!(cadcraft_engine::doc::plot_style_table(d, "Missing.ctb").is_none());
    assert!(s.log.iter().any(|l| l.contains("Office.ctb") && l.contains("found")), "{:?}", s.log);
    assert!(s.log.iter().any(|l| l.contains("Missing.ctb") && l.contains("not found")), "{:?}", s.log);
    let _ = std::fs::remove_dir_all(&dir);
}

/// Opened by a bare file name (as `cadcraft-cli run plan.dxf` does), the drawing's folder is the
/// current one, and the table beside it is found too.
#[test]
fn open_by_a_bare_file_name_finds_the_table_in_the_current_folder() {
    hooks();
    let dir = folder("bare");
    let mut s = Session::new();
    let layout = s.doc().unwrap().layouts[0].name.clone();
    s.execute("pagesetup", &json!({"layout": layout, "plotStyleTable": "Office.ctb"})).unwrap();
    s.execute("saveas", &json!({"path": dir.join("plan.dxf").to_string_lossy()})).unwrap();
    std::fs::write(dir.join("Office.ctb"), ctb()).unwrap();
    // The other tests here use absolute paths, so moving the current folder does not touch them.
    std::env::set_current_dir(&dir).unwrap();
    let mut s = Session::new();
    s.execute("open", &json!({"path": "plan.dxf"})).unwrap();
    assert!(s.doc().unwrap().plot_style_tables.iter().any(|t| t.name.eq_ignore_ascii_case("Office.ctb")), "{:?}", s.log);
    std::env::set_current_dir(std::env::temp_dir()).unwrap();
    let _ = std::fs::remove_dir_all(&dir);
}
