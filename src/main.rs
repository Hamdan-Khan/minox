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

    // result will be an enum of sum and carry bits i.e. (sum , carry)
    fn full_adder(self, second: Self, third: Self) -> (Self, Self) {
        // sum can be calculated using a XOR b XOR c
        let sum = self.xor(second).xor(third);
        // any two bits need to be 1, for a carry bit to be one:
        let carry = self.and(second).or(self.and(third)).or(second.and(third));
        (sum, carry)
    }
}

#[derive(PartialEq)]
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
    // 10 full cycles
    for _ in 0..20 {
        clock.toggle();
        println!("clock signal: {:?}", clock.level());
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
