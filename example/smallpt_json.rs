extern crate cg_tracing;

use cg_tracing::prelude::*;

fn main() {
    let (w, mut p) = utils::from_json("smallpt_plane.json", register! {});
    w.render(&mut p);
    p.save_png(&format!("example_{}.png", w.sample));
}