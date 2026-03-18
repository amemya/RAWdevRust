fn main() {
    let mut buf = Vec::new();
    let mut encoder = png::Encoder::new(&mut buf, 100, 100);
    // encoder.info_mut().icc_profile = Some(std::borrow::Cow::Borrowed(b"abc"));
    unimplemented!();
}
