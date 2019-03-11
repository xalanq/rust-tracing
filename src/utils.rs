use crate::{
    cam::Cam,
    geo::{Hittable, Plane, Sphere},
    pic::Pic,
    world::World,
};
pub type Flt = f64;
pub const PI: Flt = std::f64::consts::PI as Flt;
pub const EPS: Flt = 1e-4;

pub fn clamp(x: Flt) -> Flt {
    if x < 0.0 {
        0.0
    } else {
        if x > 1.0 {
            1.0
        } else {
            x
        }
    }
}

pub fn to_byte(x: Flt) -> u8 {
    (clamp(x).powf(1.0 / 2.2) * 255.0 + 0.5) as u8
}

use serde::de::DeserializeOwned;
use serde_json::Value;
use std::collections::HashMap;
use std::fs;

pub type FromJsonFunc = fn(Value) -> Box<dyn Hittable>;

pub fn new_from_json<T: Hittable + DeserializeOwned + 'static>(v: Value) -> Box<dyn Hittable> {
    Box::new(serde_json::from_value::<T>(v).expect("Invalid Value"))
}

pub fn from_json(filename: &str, custom: HashMap<String, FromJsonFunc>) -> (World, Pic) {
    let data = fs::read_to_string(filename).expect(&format!("Unable to read {}", filename));
    let mut data: Value = serde_json::from_str(&data).expect("Cannot convert to json");
    let w: usize = serde_json::from_value(data["width"].take()).expect("Invalid width");
    let h: usize = serde_json::from_value(data["height"].take()).expect("Invalid height");
    let p = Pic::new(w, h);
    let camera: Cam = serde_json::from_value(data["camera"].take()).expect("Invalid camera");
    let sample: usize = serde_json::from_value(data["sample"].take()).expect("Invalid sample");
    let max_depth: usize =
        serde_json::from_value(data["max_depth"].take()).expect("Invalid maximum depth");
    let thread_num: usize =
        serde_json::from_value(data["thread_num"].take()).expect("Invalid thread number");
    let stack_size: usize =
        serde_json::from_value(data["stack_size"].take()).expect("Invalid stack size");
    let na: Flt = serde_json::from_value(data["Na"].take()).expect("Invalid Na");
    let ng: Flt = serde_json::from_value(data["Ng"].take()).expect("Invalid Ng");
    let mut w = World::new(camera, sample, max_depth, thread_num, stack_size, na, ng);
    match data["objects"].take() {
        Value::Array(objs) => {
            objs.into_iter().for_each(|_obj| {
                let mut obj = _obj;
                match obj["type"].take() {
                    Value::String(tp) => match tp.as_ref() {
                        "Sphere" => w.add(new_from_json::<Sphere>(obj)),
                        "Plane" => w.add(new_from_json::<Plane>(obj)),
                        _ => {
                            if let Some(f) = custom.get(&tp) {
                                w.add(f(obj));
                                return;
                            }
                            panic!("Unknown obj");
                        }
                    },
                    _ => panic!("Invalid obj"),
                };
            });
            (w, p)
        }
        _ => panic!("objs is not an array"),
    }
}
