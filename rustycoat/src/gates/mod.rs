use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::core::ports::{InputPin, OutputPin};
use crate::core::Component;

pub struct BinaryGate<T>
where
    T: BinaryOp + Send,
{
    input_a: InputPin,
    input_b: InputPin,
    output: OutputPin,
    phantom_data: std::marker::PhantomData<T>,
}

impl<T> BinaryGate<T>
where
    T: BinaryOp + Send,
{
    pub fn new() -> Self {
        Self::with_initial_values(false, false)
    }

    pub fn with_initial_values(input_a: bool, input_b: bool) -> Self {
        Self {
            input_a: InputPin::with_initial_value(input_a),
            input_b: InputPin::with_initial_value(input_b),
            output: OutputPin::with_initial_value(T::op(input_a, input_b)),
            phantom_data: std::marker::PhantomData::default(),
        }
    }

    pub fn input_a(&mut self) -> &mut InputPin {
        &mut self.input_a
    }

    pub fn input_b(&mut self) -> &mut InputPin {
        &mut self.input_b
    }

    pub fn output(&mut self) -> &mut OutputPin {
        &mut self.output
    }
}

impl<T> Default for BinaryGate<T>
where
    T: BinaryOp + Send,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Component for BinaryGate<T>
where
    T: BinaryOp + Send,
{
    fn run(&mut self, stop: Arc<AtomicBool>) {
        loop {
            InputPin::wait_any(&mut [&mut self.input_a, &mut self.input_b]);
            if stop.load(Ordering::Relaxed) {
                break;
            }
            let output = T::op(self.input_a.value(), self.input_b.value());
            println!("{}", output);
            self.output.update(output);
        }
    }
}

pub trait BinaryOp {
    fn op(a: bool, b: bool) -> bool;
}

pub struct AndOp;
impl BinaryOp for AndOp {
    fn op(a: bool, b: bool) -> bool {
        a && b
    }
}
pub type AndGate = BinaryGate<AndOp>;

pub struct OrOp;
impl BinaryOp for OrOp {
    fn op(a: bool, b: bool) -> bool {
        a || b
    }
}
pub type OrGate = BinaryGate<OrOp>;

pub struct EorOp;
impl BinaryOp for EorOp {
    fn op(a: bool, b: bool) -> bool {
        a ^ b
    }
}
pub type EorGate = BinaryGate<EorOp>;

pub struct NandOp;
impl BinaryOp for NandOp {
    fn op(a: bool, b: bool) -> bool {
        !(a && b)
    }
}
pub type NandGate = BinaryGate<NandOp>;

pub struct NorOp;
impl BinaryOp for NorOp {
    fn op(a: bool, b: bool) -> bool {
        !(a || b)
    }
}
