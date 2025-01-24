use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let info_plist = include_str!("Info.plist");
    fs::write(out_dir.join("Info.plist"), info_plist).unwrap();
}
