//! A layout whose page setup plots a window (plot area "window", DXF LAYOUT group 74 = 4) plots
//! that window, as AutoCAD does: the window's corner lands on the printable area's corner.
//! Drawings from AutoCAD often have their sheet away from the paper origin (limits from y = 297,
//! say) and a window around it; plotting from the origin put the whole sheet off the page.
//!
//! Added in the Otto fork of CADCraft (github.com/philipp-tch/cadcraft, 2026-10-21).

use cadcraft_doc::{Common, Drawing, EntityKind, Layout, LwPolyline, PageSetup, Space};
use cadcraft_geom::{PolyVertex, Vec2};
use cadcraft_io::{PdfOptions, pdf};

/// An A4 landscape layout in millimetres whose frame sits 297 mm up from the origin, plotted
/// through the window (0, 297)–(297, 507) around it.
fn drawing(plot_area: &str) -> Drawing {
    let mut d = Drawing::new_metric();
    d.header.set_i64("MEASUREMENT", 1);
    let mut l = Layout::new("SHEET", 3);
    l.page = PageSetup {
        paper: "ISO A4".into(),
        width_mm: 210.0,
        height_mm: 297.0,
        margins_mm: [0.0; 4],
        landscape: true,
        paper_mm: true,
        plot_area: plot_area.into(),
        plot_window: [0.0, 297.0, 297.0, 507.0],
        ..PageSetup::default()
    };
    d.layouts.push(l);
    let frame = [(10.0, 307.0), (287.0, 307.0), (287.0, 497.0), (10.0, 497.0)];
    let pl = LwPolyline {
        vertices: frame.iter().map(|(x, y)| PolyVertex::new(Vec2::new(*x, *y))).collect(),
        closed: true,
        const_width: 0.0,
        elevation: 0.0,
        plinegen: false,
    };
    d.add(&Space::Paper("SHEET".into()), Common::default(), EntityKind::LwPolyline(pl)).unwrap();
    d
}

/// The y coordinates (points) of the PDF's path operators.
fn path_ys(d: &Drawing) -> Vec<f64> {
    let bytes = pdf(d, &Space::Paper("SHEET".into()), &PdfOptions { compress: false, ..PdfOptions::default() }).unwrap();
    let text = String::from_utf8_lossy(&bytes);
    text.lines().filter(|l| l.ends_with(" m") || l.ends_with(" l")).filter_map(|l| l.split_whitespace().nth(1)?.parse().ok()).collect()
}

#[test]
fn a_layout_plotting_a_window_plots_that_window() {
    let ys = path_ys(&drawing("window"));
    assert!(!ys.is_empty(), "no path in the PDF");
    let (lo, hi) = ys.iter().fold((f64::MAX, f64::MIN), |(a, b), y| (a.min(*y), b.max(*y)));
    // The frame is 10 mm inside the window: 28.35 pt to 566.93 pt on a 595.28 pt high page.
    assert!((lo - 28.35).abs() < 0.1 && (hi - 566.93).abs() < 0.1, "frame from {lo} to {hi} pt");
}

#[test]
fn a_layout_plotting_the_layout_still_plots_from_the_origin() {
    // Plot area "layout": the sheet is the paper from the origin, so the frame stays where it is
    // (above the page here), as before.
    let ys = path_ys(&drawing("layout"));
    assert!(ys.iter().all(|y| *y > 595.0), "{ys:?}");
}
