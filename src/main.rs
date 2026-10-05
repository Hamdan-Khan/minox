use std::fmt::Debug;

#[derive(Copy, Clone)]
struct Bit(bool);

impl Debug for Bit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let res = if self.0 { 1 } else { 0 };
        f.write_fmt(format_args!("{}", res))
    }
}

impl Bit {
    // and, or, not, xor operations on bool can already be performed in rust out
    // of the box, but to build upon the concept of deriving gates from
    // the basic NAND gate, we will use it as a building block to creat every
    // other gate.
    fn nand(&self, right: &Self) -> Self {
        Bit(!(self.0 & right.0))
    }

    fn not(&self) -> Self {
        self.nand(self)
    }

    fn and(&self, right: &Self) -> Self {
        Bit(!self.nand(right).0)
    }

    fn or(&self, right: &Self) -> Self {
        let not_left = self.not();
        let ands = not_left.and(&right.not());
        ands.not()
    }
}

struct Byte([Bit; 8]);

impl Byte {
    fn new() -> Self {
        Byte([
            Bit(false),
            Bit(false),
            Bit(false),
            Bit(false),
            Bit(false),
            Bit(false),
            Bit(false),
            Bit(false),
        ])
    }

    fn from_int(bin: u8) -> Self {
        let bits = std::array::from_fn(|i| Bit((bin & (1 << (7 - i))) != 0));
        Byte(bits)
    }
}

impl Debug for Byte {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let res = self
            .0
            .iter()
            .fold(0, |acc, &bit| (acc << 1) | (bit.0 as u8));
        f.write_fmt(format_args!("{:08b}", res))
    }
}

fn main() {
    let one = Bit(true);
    let zero = Bit(false);
    let byte = Byte::from_int(0b_0000_1000);
    println!("{:?}", one.nand(&zero));
    println!("{:?}", byte);
}
