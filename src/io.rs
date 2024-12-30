/// IO functionality for terminals and JS-WASM interop.

#[cfg(target_family = "unix")]
pub fn get_char() -> u8 {
    use std::io::Read;
    std::io::stdin()
        .bytes()
        .next()
        .and_then(|result| result.ok())
        .unwrap_or(0)
}

#[cfg(target_family = "wasm")]
pub fn get_char() -> u8 {
    getChar() as u8
}

#[cfg(target_family = "unix")]
pub fn put_char(c: u8) {
    use std::io::Write;

    let mut stdout = std::io::stdout().lock();
    stdout.write(&[c]).ok();
    stdout.flush().ok();
}

#[cfg(target_family = "wasm")]
pub fn put_char(c: u8) {
    putChar(c);
}


#[cfg(target_family = "wasm")]
#[wasm_bindgen::prelude::wasm_bindgen(module = "/lib.js")]
extern "C" {
    fn getChar() -> u32;
    fn putChar(val: u8);
}