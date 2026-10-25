//! A twisted layout viewport's view centre (group 12) is in display coordinates (DCS): the model
//! point turned by the view twist (51), measured from the view target (17), per the public DXF
//! Reference. CADCraft keeps the model point; reading turns it back, writing turns it again.
//! Far from the origin (national grids) the two readings differ by kilometres.
//!
//! Added in the Otto fork of CADCraft (github.com/philipp-tch/cadcraft, 2026-10-20).

use cadcraft_doc::*;
use cadcraft_geom::{Vec2, Vec3};

const MODEL: Vec2 = Vec2::new(2_600_000.0, 1_200_000.0);
const TWIST: f64 = -30.0_f64 * std::f64::consts::PI / 180.0;

fn groups(text: &str) -> Vec<(String, String)> {
    let lines: Vec<&str> = text.lines().map(str::trim).collect();
    lines.chunks(2).filter_map(|c| Some((c.first()?.to_string(), c.get(1)?.to_string()))).collect()
}
fn text_of(gs: &[(String, String)]) -> String {
    gs.iter().map(|(c, v)| format!("{c}\n{v}\n")).collect()
}
fn viewport(d: &Drawing) -> Viewport {
    let sheet = Space::Paper(d.layouts[0].name.clone());
    d.space(&sheet)
        .expect("the layout")
        .iter()
        .find_map(|e| match &e.kind {
            EntityKind::Viewport(v) => Some(v.clone()),
            _ => None,
        })
        .expect("a layout viewport")
}
/// A drawing with one layout viewport twisted by 30° and looking at `MODEL`.
fn drawing() -> Drawing {
    let mut d = Drawing::new_metric();
    let sheet = Space::Paper(d.layouts[0].name.clone());
    let vp = Viewport {
        center: Vec3::new(230.0, 80.0, 0.0),
        width: 420.0,
        height: 120.0,
        view_center: MODEL,
        view_height: 60.0,
        id: 2,
        twist: TWIST.rem_euclid(std::f64::consts::TAU),
        ..Viewport::default()
    };
    d.add(&sheet, Common::default(), EntityKind::Viewport(vp)).unwrap();
    d
}
/// The VIEWPORT record's (12, 22) groups.
fn group12(gs: &[(String, String)]) -> (usize, Vec2) {
    let record = |i: usize| gs[i + 1..].iter().take_while(|g| g.0 != "0");
    let i = (0..gs.len()).find(|&i| gs[i].0 == "0" && gs[i].1 == "VIEWPORT" && record(i).any(|g| g.0 == "51")).expect("twisted VIEWPORT");
    let j = i + gs[i..].iter().position(|g| g.0 == "12").expect("group 12");
    (j, Vec2::new(gs[j].1.parse().unwrap(), gs[j + 1].1.parse().unwrap()))
}

#[test]
fn view_centre_is_written_in_dcs_and_read_back_as_the_model_point() {
    let d = drawing();
    let text = cadcraft_io::write_dxf(&d);
    let (_, dcs) = group12(&groups(&text));
    assert!((dcs - MODEL.rotate(TWIST)).len() < 1e-6, "group 12 {dcs:?}");
    let back = viewport(&cadcraft_io::read_dxf(text.as_bytes()).unwrap());
    assert!((back.view_center - MODEL).len() < 1e-6, "model centre {:?}", back.view_center);
    assert!((back.model_to_paper(MODEL) - Vec2::new(230.0, 80.0)).len() < 1e-6);
}

#[test]
fn view_target_moves_the_model_centre() {
    // Another program's file: group 12 measured from a view target (17).
    let target = Vec2::new(1000.0, -500.0);
    let mut gs = groups(&cadcraft_io::write_dxf(&drawing()));
    let (j, _) = group12(&gs);
    let dcs = (MODEL - target).rotate(TWIST);
    gs[j].1 = dcs.x.to_string();
    gs[j + 1].1 = dcs.y.to_string();
    for (k, (c, v)) in [("17", target.x), ("27", target.y), ("37", 0.0)].into_iter().enumerate() {
        gs.insert(j + 2 + k, (c.to_string(), v.to_string()));
    }
    let back = viewport(&cadcraft_io::read_dxf(text_of(&gs).as_bytes()).unwrap());
    assert!((back.view_center - MODEL).len() < 1e-6, "model centre {:?}", back.view_center);
}
