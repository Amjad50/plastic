use std::io::Cursor;

use crate::tests::NesTester;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TestState {
    Running,
    Passed,
    Failed,
}
const BLARGG_MEM_RESULT: u16 = 0x6000;
const BLARGG_STATE_RUNNING: u8 = 0x80;

fn get_test_state(nes: &NesTester) -> TestState {
    match nes.cpu_read_address(BLARGG_MEM_RESULT) {
        BLARGG_STATE_RUNNING => TestState::Running,
        0 => TestState::Passed,
        _ => TestState::Failed,
    }
}

#[test]
fn save_load_test() {
    // 0- perform normal test (this part should always pass)
    let file_path = "../test_roms/instr_test-v5/all_instrs.nes";

    // 1- make sure after start and advancing 2 frames does not pass the test
    let mut nes = NesTester::new(file_path).unwrap();
    nes.clock_for_frame();
    nes.clock_for_frame();
    assert_eq!(get_test_state(&nes), TestState::Running);

    // create it again, and then run until it passes
    nes = NesTester::new(file_path).unwrap();
    nes.clock_until_infinite_loop();
    nes.clock_until_memory_neq(BLARGG_MEM_RESULT, BLARGG_STATE_RUNNING);
    assert_eq!(get_test_state(&nes), TestState::Passed);

    // 2- save the state at which it was passing
    let mut buffer = Vec::new();
    nes.nes.save_state(&mut buffer).unwrap();

    // 3- create a new object and load the state
    nes = NesTester::new(file_path).unwrap();
    let mut c = Cursor::new(&buffer);
    nes.nes.load_state(&mut c).unwrap();
    assert!(c.position() == buffer.len() as u64);

    assert_eq!(get_test_state(&nes), TestState::Passed);
}
