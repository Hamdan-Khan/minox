#![allow(dead_code)]
use std::{array::from_fn, fmt::Debug};

#[derive(Copy, Clone, PartialEq)]
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
    fn nand(self, right: Self) -> Self {
        Bit(!(self.0 & right.0))
    }

    fn not(self) -> Self {
        self.nand(self)
    }

    fn and(self, right: Self) -> Self {
        Bit(!self.nand(right).0)
    }

    fn or(self, right: Self) -> Self {
        let not_left = self.not();
        let ands = not_left.and(right.not());
        ands.not()
    }

    fn xor(self, right: Self) -> Self {
        let not_left = self.not();
        let not_right = right.not();
        not_left.and(right).or(self.and(not_right))
    }

    fn mux(a: Self, b: Self, sel: Self) -> Self {
        let a_and_not_sel = sel.not().and(a);
        let b_and_sel = b.and(sel);
        a_and_not_sel.or(b_and_sel)
    }

    // result will be an enum of sum and carry bits i.e. (sum , carry)
    fn full_adder(self, second: Self, third: Self) -> (Self, Self) {
        // sum can be calculated using a XOR b XOR c
        let sum = self.xor(second).xor(third);
        // any two bits need to be 1, for a carry bit to be one:
        let carry = self.and(second).or(self.and(third)).or(second.and(third));
        (sum, carry)
    }
}

#[derive(PartialEq, Clone, Copy)]
struct Word([Bit; 32]);

impl Word {
    fn new() -> Self {
        Word(from_fn(|_i| Bit(false)))
    }

    fn from_int(bin: u32) -> Self {
        let bits = from_fn(|i| Bit((bin >> (31 - i)) & 1 != 0));
        Word(bits)
    }

    fn not(self) -> Self {
        let bits = from_fn(|i| self.0[i].not());
        Word(bits)
    }

    fn and(self, right: Self) -> Self {
        let bits = from_fn(|i| self.0[i].and(right.0[i]));
        Word(bits)
    }

    fn or(self, right: Self) -> Self {
        let bits = from_fn(|i| self.0[i].or(right.0[i]));
        Word(bits)
    }

    fn xor(self, right: Self) -> Self {
        let bits = from_fn(|i| self.0[i].xor(right.0[i]));
        Word(bits)
    }

    fn full_adder(self, right: Self) -> Self {
        // initially carry bit will be zero
        let mut carry_bit = Bit(false);
        let mut bits = [Bit(false); 32];

        // starting with index 31 because we want to start the addition with the
        // least significant bits i.e. the right most bits
        for i in (0..32).rev() {
            let (sum, carry) = self.0[i].full_adder(right.0[i], carry_bit);
            bits[i] = sum;
            carry_bit = carry;
        }

        Word(bits)
    }
}

impl Debug for Word {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let res = self
            .0
            .iter()
            .fold(0, |acc, &bit| (acc << 1) | (bit.0 as u32));
        f.write_fmt(format_args!("{:032b}", res))
    }
}

struct Clock {
    signal: Bit,
}

impl Clock {
    fn new() -> Self {
        Self { signal: Bit(false) }
    }

    fn toggle(&mut self) {
        self.signal = self.signal.not();
    }

    fn level(&self) -> Bit {
        self.signal
    }
}

struct DLatch {
    q: Bit,
}

impl DLatch {
    fn new() -> Self {
        Self { q: Bit(false) }
    }

    // simulating an SR latch with clock (enable) and data bit inputs
    fn step(&mut self, d: Bit, clk: Bit) {
        let s = d.nand(clk);
        let r = d.not().nand(clk);

        let mut q = self.q;
        let mut qb = q.not();
        // cross coupled nands will settle in two passes
        for _ in 0..2 {
            let q_next = s.nand(qb);
            let qb_next = r.nand(q);
            q = q_next;
            qb = qb_next;
        }
        self.q = q;
    }

    fn output(&self) -> Bit {
        self.q
    }
}

struct DFlipFlop {
    master_latch: DLatch,
    slave_latch: DLatch,
}

impl DFlipFlop {
    fn new() -> Self {
        Self {
            master_latch: DLatch::new(),
            slave_latch: DLatch::new(),
        }
    }

    fn step(&mut self, d: Bit, clk: Bit) {
        // rising edge, because master latch is enabled on low signal
        // (when clock signal is about to rise)
        self.master_latch.step(d, clk.not());
        let q_m = self.master_latch.output();
        self.slave_latch.step(q_m, clk);
    }

    fn output(&self) -> Bit {
        self.slave_latch.output()
    }
}

// unlike a plain d flip flop, that only remembers a data bit for one clock
// cycle, a bit register will indefinitely remember the data using a loop + mux
struct BitRegister {
    bit: DFlipFlop,
}

impl BitRegister {
    fn new() -> Self {
        Self {
            bit: DFlipFlop::new(),
        }
    }

    fn step(&mut self, d: Bit, load: Bit, clk: Bit) {
        let inp = Bit::mux(self.bit.output(), d, load);
        self.bit.step(inp, clk);
    }

    fn output(&self) -> Bit {
        self.bit.output()
    }
}

struct WordRegister {
    bit_registers: [BitRegister; 32],
}

impl WordRegister {
    fn new() -> Self {
        Self {
            bit_registers: from_fn(|_| BitRegister::new()),
        }
    }

    fn step(&mut self, d: Word, load: Bit, clk: Bit) {
        for (i, b) in self.bit_registers.iter_mut().enumerate() {
            b.step(d.0[i], load, clk);
        }
    }

    fn output(&self) -> Word {
        Word(from_fn(|i| self.bit_registers[i].output()))
    }
}

fn main() {
    let one = Bit(true);
    let zero = Bit(false);
    let byte1 = Word::from_int(0b_1000_1000_0000_1000);
    let byte2 = Word::from_int(0b_1000_1000_0000_1000);
    let byte3 = Word::new();
    println!("{:?}", one.nand(zero));
    println!("byte1: {:?}", byte1);
    println!("byte2: {:?}", byte2);
    println!("b1 and b2: {:?}", byte1.and(byte2));
    println!("byte3: {:?}", byte3);
    println!("not b3: {:?}", byte3.not());

    let mut clock = Clock::new();
    let mut dff_bit = DFlipFlop::new();
    let mut register_bit = BitRegister::new();
    // 10 full cycles
    for i in 0..20 {
        clock.toggle();
        let clk = clock.level();
        // the previous q passed as data means: the output of
        let prev_memory = dff_bit.output();
        // at 4th cycle, the clock signal is low so master captures it and
        // in the 5th cycle when the clock signal is high, slave latch captures and stores it
        if i == 3 {
            dff_bit.step(one, clk);
            register_bit.step(one, one, clk);
        } else {
            dff_bit.step(prev_memory, clk);
            // because load is zero, the data bit doesn't matter aka register stores the previous data bit
            register_bit.step(zero, zero, clk);
        }

        println!(
            "clock signal: {:?}, dff: {:?}, register: {:?}",
            clock.level(),
            dff_bit.output(),
            register_bit.output()
        );
    }
}

#[cfg(test)]
mod gate_truth_tables;

#[cfg(test)]
mod memory_tests;

#[test]
fn adder_tests() {
    let word1 = Word::from_int(0b_0000_1111_1010_1011);
    let word2 = Word::from_int(0b_0000_1000_0101_0100);
    assert_eq!(
        word1.full_adder(word2),
        Word::from_int(0b_0001_0111_1111_1111)
    );
}
