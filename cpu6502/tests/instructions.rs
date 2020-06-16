extern crate cpu6502;

use cpu6502::instruction;
use cpu6502::instruction::Instruction;

// TODO: create tests for instructions and cpu
#[test]
fn adding() {
    assert_eq!(2 , 1 + 1);
}

#[test]
fn subtracting() {
    assert_eq!(0, 1 - 1);
}

