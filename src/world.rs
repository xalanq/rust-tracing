use crate::{geo::*, pic::*, ray::*, utils::clamp, vct::*, Flt, PI};
use pbr::ProgressBar;
use rand::prelude::*;
use std::time::Duration;

type VecHit = Vec<Box<dyn Hittable>>;

pub struct World {
    lock: VecHit,
    cam: Ray,
    sample: i32,
    depth: i32,
    thread_num: i32,
    ratio: Flt,
    n1: Flt,
    n2: Flt,
    r0: Flt,
}

impl World {
    pub fn new(
        cam: Ray,
        sample: i32,
        depth: i32,
        thread_num: i32,
        ratio: Flt,
        na: Flt,
        ng: Flt,
    ) -> Self {
        Self {
            lock: Vec::new(),
            cam,
            sample,
            depth,
            thread_num,
            ratio,
            n1: na / ng,
            n2: ng / na,
            r0: ((na - ng) * (na - ng)) / ((na + ng) * (na + ng)),
        }
    }

    pub fn add(&mut self, obj: Box<dyn Hittable>) -> &mut Self {
        self.lock.push(obj);
        self
    }

    fn find<'a>(&'a self, r: &Ray, objs: &'a VecHit) -> Option<(&'a Geo, Vct, Vct)> {
        let mut t: Flt = 1e30;
        let mut obj = None;
        objs.iter().for_each(|o| {
            if let Some(d) = o.hit(r) {
                if d < t {
                    t = d;
                    obj = Some(o);
                }
            }
        });
        if let Some(o) = obj {
            let g = o.get();
            let pos = r.origin + r.direct * t;
            let norm = (pos - g.position).norm();
            return Some((g, pos, norm));
        }
        None
    }

    fn trace(&self, r: &Ray, mut depth: i32, rng: &mut ThreadRng, objs: &VecHit) -> Vct {
        if let Some((g, pos, norm)) = self.find(r, objs) {
            let mut cl = g.color;
            depth += 1;
            if depth > self.depth {
                let p = cl.x.max(cl.y.max(cl.z));
                if rng.gen::<Flt>() < p {
                    cl /= p;
                } else {
                    return g.emission;
                }
            }
            let mut ff = || {
                let nd = norm.dot(&r.direct);
                if g.texture == Texture::Diffuse {
                    let w = if nd < 0.0 { norm } else { -norm };
                    let (r1, r2) = (PI * 2.0 * rng.gen::<Flt>(), rng.gen::<Flt>());
                    let r2s = r2.sqrt();
                    let u = (if w.x.abs() <= 0.1 {
                        Vct::new(1.0, 0.0, 0.0)
                    } else {
                        Vct::new(0.0, 1.0, 0.0)
                    } % w)
                        .norm();
                    let v = w % u;
                    let d = (u * r1.cos() + v * r1.sin()) * r2s + w * (1.0 - r2).sqrt();
                    return self.trace(&Ray::new(pos, d.norm()), depth, rng, objs);
                }
                let refl = Ray::new(pos, r.direct - norm * (2.0 * nd));
                if g.texture == Texture::Specular {
                    return self.trace(&refl, depth, rng, objs);
                }
                let w = if nd < 0.0 { norm } else { -norm };
                let (it, ddw) = (norm.dot(&w) > 0.0, r.direct.dot(&w));
                let (n, sign) = if it { (self.n1, 1.0) } else { (self.n2, -1.0) };
                let cos2t = 1.0 - n * n * (1.0 - ddw * ddw);
                if cos2t < 0.0 {
                    return self.trace(&refl, depth, rng, objs);
                }
                let td = (r.direct * n - norm * ((ddw * n + cos2t.sqrt()) * sign)).norm();
                let refr = Ray::new(pos, td);
                let c = if it { 1.0 + ddw } else { 1.0 - td.dot(&norm) };
                let cc = c * c;
                let re = self.r0 + (1.0 - self.r0) * cc * cc * c;
                let tr = 1.0 - re;
                if depth > 2 {
                    let p = 0.25 + 0.5 * re;
                    if rng.gen::<Flt>() < p {
                        self.trace(&refl, depth, rng, objs) * (re / p)
                    } else {
                        self.trace(&refr, depth, rng, objs) * (tr / (1.0 - p))
                    }
                } else {
                    self.trace(&refl, depth, rng, objs) * re
                        + self.trace(&refr, depth, rng, objs) * tr
                }
            };
            return g.emission + cl * ff();
        }
        Vct::zero()
    }

    fn gend(rng: &mut ThreadRng) -> Flt {
        let r = 2.0 * rng.gen::<Flt>();
        if r < 1.0 {
            r.sqrt() - 1.0
        } else {
            1.0 - (2.0 - r).sqrt()
        }
    }

    pub fn render(&self, p: &mut Pic) {
        let (w, h) = (p.w, p.h);
        let (fw, fh) = (w as Flt, h as Flt);
        let cx = Vct::new(fw * self.ratio / fh, 0.0, 0.0);
        let cy = (cx % self.cam.direct).norm() * self.ratio;
        let sample = self.sample / 4;
        let inv = 1.0 / sample as Flt;
        println!("w: {}, h: {}, sample: {}, actual sample: {}", w, h, self.sample, sample * 4);
        let mut pb = ProgressBar::new((w * h) as u64);
        pb.set_max_refresh_rate(Some(Duration::from_secs(1)));

        let mut data: Vec<(usize, usize)> = Vec::new();
        (0..w).for_each(|x| (0..h).for_each(|y| data.push((x, y))));
        data.shuffle(&mut rand::thread_rng());

        println!("start render with {} threads.", self.thread_num);
        data.into_iter().for_each(|(x, y)| {
            let objs = &self.lock;
            let mut sum = Vct::zero();
            let (fx, fy) = (x as Flt, y as Flt);
            for sx in 0..2 {
                for sy in 0..2 {
                    let mut c = Vct::zero();
                    let mut rng = rand::thread_rng();
                    for _ in 0..sample {
                        let (fsx, fsy) = (sx as Flt, sy as Flt);
                        let ccx = cx * (((fsx + 0.5 + Self::gend(&mut rng)) / 2.0 + fx) / fw - 0.5);
                        let ccy = cy * (((fsy + 0.5 + Self::gend(&mut rng)) / 2.0 + fy) / fh - 0.5);
                        let d = ccx + ccy + self.cam.direct;
                        let r = Ray::new(self.cam.origin + d * 130.0, d.norm());
                        c += self.trace(&r, 0, &mut rng, objs) * inv;
                    }
                    sum += Vct::new(clamp(c.x), clamp(c.y), clamp(c.z)) * 0.25;
                }
            }
            p.set(x, h - y - 1, &sum);
            pb.inc();
        });
        pb.finish_println("Rendering completed");
    }
}