//! Added in the Otto fork of CADCraft (github.com/philipp-tch/cadcraft, 2026-10-30), the fork's own work: PROPERTIES `elevation` and `thickness` (and an insertion point's z) set from JSON, as AutoCAD's
//! Properties palette does. Before, `properties.set` answered `changed` and left them as they were.

use cadcraft_engine::Session;
use cadcraft_engine::doc::{EntityKind, Handle};
use serde_json::json;

fn last(s: &Session) -> Handle {
    s.doc().unwrap().model.last().unwrap().handle
}

#[test]
fn a_polyline_takes_its_elevation_and_thickness() {
    let mut s = Session::new();
    s.execute("pline", &json!({ "points": [[0, 0], [10, 0], [10, 5]], "closed": true })).unwrap();
    let h = last(&s);
    s.execute("properties.set", &json!({ "handles": [h.hex()], "elevation": 576.03, "thickness": 0.47 })).unwrap();
    let e = s.doc().unwrap().entity(h).cloned().unwrap();
    assert_eq!(e.common.thickness, 0.47);
    match &e.kind {
        EntityKind::LwPolyline(p) => assert_eq!(p.elevation, 576.03),
        k => panic!("not a polyline: {k:?}"),
    }
}

#[test]
fn a_block_reference_takes_the_z_of_its_insertion_point() {
    let mut s = Session::new();
    s.execute("circle", &json!({ "center": [0, 0], "radius": 0.2 })).unwrap();
    let c = last(&s);
    s.execute("block", &json!({ "name": "B", "base": [0, 0], "handles": [c.hex()], "keep": "delete" })).unwrap();
    s.execute("insert", &json!({ "name": "B", "at": [3, 3] })).unwrap();
    let h = last(&s);
    s.execute("properties.set", &json!({ "handles": [h.hex()], "elevation": 579.1 })).unwrap();
    let e = s.doc().unwrap().entity(h).cloned().unwrap();
    match &e.kind {
        EntityKind::Insert(i) => assert_eq!((i.insert.x, i.insert.y, i.insert.z), (3.0, 3.0, 579.1)),
        k => panic!("not a block reference: {k:?}"),
    }
}
