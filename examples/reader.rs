extern crate ru5102;

use ru5102::*;

fn main() {
    let mut reader = ru5102::Reader::new("/dev/ttyUSB0");
    reader.inventory();
}
