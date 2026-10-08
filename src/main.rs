#![allow(dead_code)]
use std::{array::from_fn, fmt::Debug};

use rustc_hash::FxHashMap;

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

    fn to_int(self) -> u8 {
        if self.0 { 1 } else { 0 }
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

    fn to_int(self) -> u32 {
        self.0
            .iter()
            .fold(0, |acc, &bit| (acc << 1) | (bit.0 as u32))
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

    fn full_adder(self, right: Self, carry_in: Bit) -> (Self, Bit) {
        // for subtraction, the initial carry bit is 1 i.e.
        // A + (~B) + 1
        let mut carry_bit = carry_in;
        let mut bits = [Bit(false); 32];

        // starting with index 31 because we want to start the addition with the
        // least significant bits i.e. the right most bits
        for i in (0..32).rev() {
            let (sum, carry) = self.0[i].full_adder(right.0[i], carry_bit);
            bits[i] = sum;
            carry_bit = carry;
        }

        (Word(bits), carry_bit)
    }
}

impl Debug for Word {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let res = self.to_int();
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

// in riscv, the regfile consists of 32 registers with a 5-bit address space
struct RegFile {
    registers: [WordRegister; 32],
}

impl RegFile {
    fn new() -> Self {
        Self {
            registers: from_fn(|_| WordRegister::new()),
        }
    }
}

struct ProgramCounter(WordRegister);

impl ProgramCounter {
    fn new() -> Self {
        // points to 0 address initially
        Self(WordRegister::new())
    }

    fn step(&self) -> Word {
        self.0.output()
    }
}

// ideally an L1 cache inside the processor, but we'll address it using 32-bits and
// treat it as a RAM because in this simulation, because the overhead for accessing instructions
// from the RAM and L1 cache would be the same.
struct InstructionMemory {
    // using `Word` or `WordRegister` is not relevant here since we're not concerned with
    // how the hardware of a RAM works, so choosing the simplest storage way I can think of
    instructions: FxHashMap<u32, u32>,
}

impl InstructionMemory {
    fn new() -> Self {
        let mut map = FxHashMap::default();
        map.insert(0, 0);
        Self { instructions: map }
    }

    fn step(&self, addr: Word) -> Word {
        match self.instructions.get(&addr.to_int()) {
            Some(&instruction) => Word::from_int(instruction),
            None => Word::new(),
        }
    }
}

// helper to extract bits from a word (can think of it as routing wires)
fn bits<const START: usize, const LEN: usize>(w: Word) -> [Bit; LEN] {
    from_fn(|i| w.0[START + i])
}

// fetch, decode, execute
fn fde() {
    let instruction_memory = InstructionMemory::new();
    let program_counter = ProgramCounter::new();
    let mut clock = Clock::new();

    for _i in 0..20 {
        clock.toggle();
        let clk = clock.level();

        // get the address from the program counter output and fetch the
        // instruction from the inst memory
        let pc_output = program_counter.0.output();
        let instruction = instruction_memory.step(pc_output);

        // since the bits in the instruction word are ordered MSB (left) to LSB (right),
        // the indexing to extract bits would be opposite i.e. 25 as start index insted
        // of 0 for the opcode
        let op_code = bits::<25, 7>(instruction);
    }
}

fn main() {
    fde();
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
        word1.full_adder(word2, Bit(false)).0,
        Word::from_int(0b_0001_0111_1111_1111)
    );
}
