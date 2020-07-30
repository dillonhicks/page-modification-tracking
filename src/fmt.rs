use std::fmt;


pub struct Hex<'a, N: fmt::LowerHex>(pub &'a N);

impl<'a, N: fmt::LowerHex> fmt::Debug for Hex<'a, N> {
    fn fmt(
        &self,
        f: &mut fmt::Formatter,
    ) -> fmt::Result {
        fmt::LowerHex::fmt(self.0, f)
    }
}


pub struct Binary<'a, N: fmt::Binary>(pub &'a N);

impl<'a, N: fmt::Binary> fmt::Debug for Binary<'a, N> {
    fn fmt(
        &self,
        f: &mut fmt::Formatter,
    ) -> fmt::Result {
        fmt::Binary::fmt(self.0, f)
    }
}
