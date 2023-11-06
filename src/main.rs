pub mod web;
pub mod commonio;
pub mod discord;

use std::thread;

use crate::web::web;
use crate::discord::discord;

fn main() {
    let t1 = thread::spawn(|| {
        discord();
    });
    let t2 = thread::spawn(|| {
        web();
    });

    t1.join().unwrap();
    t2.join().unwrap();
}