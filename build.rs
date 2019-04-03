extern crate vergen;

use vergen::{generate_cargo_keys, ConstantsFlags};

fn main() {
    generate_cargo_keys(ConstantsFlags::SHA_SHORT | ConstantsFlags::SEMVER_FROM_CARGO_PKG)
        .expect("Unable to generate cargo keys!");
}
