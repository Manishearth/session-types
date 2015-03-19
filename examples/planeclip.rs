// This is an implementation of the Sutherland-Hodgman (1974) reentrant polygon
// clipping algorithm. It takes a polygon represented as a number of vertices
// and cuts it according to the given planes.

// The implementation borrows heavily from Pucella-Tov (2008). See that paper
// for more explanation.

#![feature(plugin, custom_derive)]
#![plugin(rand_macros)]

extern crate session_types;
extern crate rand;

use session_types::*;

use std::sync::mpsc::{Sender, Receiver, channel};
use std::thread::spawn;
use std::num::Float;

#[derive(Debug, Copy, Rand)]
struct Point(f64, f64, f64);

#[derive(Debug, Copy, Rand)]
struct Plane(f64, f64, f64, f64);

fn above(Point(x, y, z): Point, Plane(a, b, c, d): Plane) -> bool {
    (a * x + b * y + c * z + d) / Float::sqrt(a * a + b * b + c * c) > 0.0
}

fn intersect(p1: Point, p2: Point, plane: Plane) -> Option<Point> {
    let Point(x1, y1, z1) = p1;
    let Point(x2, y2, z2) = p2;
    let Plane(a, b, c, d) = plane;

    if above(p1, plane) == above(p2, plane) {
        None
    } else {
        let t = (a * x1 + b * y1 + c * z1 + d) /
            (a * (x1 - x2) + b * (y1 - y2) + c * (z1 - z2));
        let x = x1 + (x2 - x1) * t;
        let y = y1 + (y2 - y1) * t;
        let z = z1 + (z2 - z1) * t;
        Some(Point(x, y, z))
    }
}

type SendList<A> = Rec<Choose<Eps, Send<A, Var<Z>>>>;

fn sendlist<A: std::marker::Send+Copy+'static>
    (tx: Sender<Chan<(), SendList<A>>>, xs: Vec<A>)
{
    let c = accept(tx).unwrap();
    let mut c = c.enter();
    for x in xs.iter() {
        let c1 = c.sel2().send(*x);
        c = c1.zero();
    }
    c.sel1().close();
}

fn recvlist<A: std::marker::Send+'static>
    (rx: Receiver<Chan<(), SendList<A>>>) -> Vec<A>
{
    let c = request(rx).unwrap();
    let mut v = Vec::new();
    let mut c = c.enter();
    loop {
        c = match c.offer() {
            Ok(c) => {
                c.close();
                break;
            }
            Err(c) => {
                let (c, x) = c.recv();
                v.push(x);
                c.zero()
            }
        }
    }

    v
}

fn clipper(plane: Plane,
           inrv: Receiver<Chan<(), SendList<Point>>>,
           outrv: Sender<Chan<(), SendList<Point>>>)
{
    let oc = accept(outrv).unwrap();
    let ic = request(inrv).unwrap();

    let mut oc = oc.enter();
    let mut ic = ic.enter();

    match ic.offer() {
        Ok(c) => {
            c.close();
            oc.sel1().close();
        }
        Err(ic2) => {
            let (ic2, pt0) = ic2.recv();
            ic = ic2.zero();
            let mut pt = pt0;
            loop {
                if above(pt, plane) {
                    oc = oc.sel2().send(pt).zero();
                }
                match ic.offer() {
                    Ok(c) => {
                        match intersect(pt, pt0, plane) {
                            Some(pt) => { oc = oc.sel2().send(pt).zero(); }
                            None => ()
                        }
                        c.close();
                        oc.sel1().close();
                        break;
                    }
                    Err(ic2) => {
                        let (ic2, pt2) = ic2.recv();
                        ic = ic2.zero();
                        match intersect(pt, pt2, plane) {
                            Some(pt) => { oc = oc.sel2().send(pt).zero(); }
                            None => ()
                        }
                        pt = pt2;
                    }
                }
            }
        }
    }
}

fn clipmany(planes: Vec<Plane>, points: Vec<Point>) -> Vec<Point> {
    let (tx, rx) = channel();
    spawn(move || sendlist(tx, points));
    let mut planes = planes;
    let mut rx = rx;
    loop {
        match planes.pop() {
            None => {break;}
            Some(p) => {
                let (tx2, rx2) = channel();
                spawn(move || clipper(p, rx, tx2));
                rx = rx2;
            }
        }
    }
    recvlist(rx)
}

fn normalize_point(Point(a,b,c): Point) -> Point {
    Point(10.0 * (a - 0.5),
          10.0 * (b - 0.5),
          10.0 * (c - 0.5))
}

fn normalize_plane(Plane(a,b,c,d): Plane) -> Plane {
    Plane(10.0 * (a - 0.5),
          10.0 * (b - 0.5),
          10.0 * (c - 0.5),
          10.0 * (d - 0.5))
}

fn bench(n: usize, m: usize) {
    let mut g = rand::thread_rng();
    let points = (0..n)
        .map(|_| rand::Rand::rand(&mut g))
        .map(normalize_point)
        .collect();
    let planes = (0..m)
        .map(|_| rand::Rand::rand(&mut g))
        .map(normalize_plane)
        .collect();

    let points = clipmany(planes, points);
    println!("{}", points.len());
}

fn main() {
    bench(100, 5);
}
