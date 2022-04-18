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

impl<T> Component for BinaryGate<T>
where
    T: BinaryOp + Send,
{
    fn run(&mut self, stop: Arc<AtomicBool>) {
        loop {
            InputPin::wait_any(&[&self.input_a, &self.input_b]);
            if stop.load(Ordering::Relaxed) {
                break;
            }
            self.output.update(T::op(self.input_a.value(), self.input_b.value()));
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
