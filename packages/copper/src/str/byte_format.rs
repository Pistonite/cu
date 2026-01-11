/// Format integer in SI bytes.
///
/// The accuracy is 1 decimal, i.e `999.9T`.
///
/// Available units are `T`, `G`, `M`, `k`, `B`
pub struct ByteFormat(pub u64);
impl std::fmt::Display for ByteFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (unit_bytes, unit_char) in [
            (1000_000_000_000, 'T'),
            (1000_000_000, 'G'),
            (1000_000, 'M'),
            (1000, 'k'),
        ] {
            if self.0 >= unit_bytes {
                let whole = self.0 / unit_bytes;
                let deci = (self.0 % unit_bytes) * 10 / unit_bytes;
                return write!(f, "{whole}.{deci}{unit_char}");
            }
        }
        write!(f, "{}B", self.0)
    }
}
