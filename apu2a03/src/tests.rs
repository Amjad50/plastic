#[cfg(test)]
mod apu_tests {
    use nes_tester::{TestError, NES};

    fn run_blargg_apu_test(filename: &str) -> Result<(), TestError> {
        let mut nes = NES::new(filename)?;
        nes.reset_cpu();

        nes.clock_until_infinite_loop();

        let result = nes.cpu_read_address(0x00f0);

        if result != 1 {
            Err(TestError::ResultError(result))
        } else {
            Ok(())
        }
    }

    // FIXME: this test is still failing
    // #[test]
    fn blargg_apu_test_01_len_ctr() -> Result<(), TestError> {
        run_blargg_apu_test("./tests/roms/blargg_apu_2005.07.30/01.len_ctr.nes")
    }

    #[test]
    fn blargg_apu_test_02_len_table() -> Result<(), TestError> {
        run_blargg_apu_test("./tests/roms/blargg_apu_2005.07.30/02.len_table.nes")
    }

    #[test]
    fn blargg_apu_test_03_irq_flag() -> Result<(), TestError> {
        run_blargg_apu_test("./tests/roms/blargg_apu_2005.07.30/03.irq_flag.nes")
    }

    #[test]
    fn blargg_apu_test_04_clock_jitter() -> Result<(), TestError> {
        run_blargg_apu_test("./tests/roms/blargg_apu_2005.07.30/04.clock_jitter.nes")
    }

    #[test]
    fn blargg_apu_test_05_len_timing_mode0() -> Result<(), TestError> {
        run_blargg_apu_test("./tests/roms/blargg_apu_2005.07.30/05.len_timing_mode0.nes")
    }

    #[test]
    fn blargg_apu_test_06_len_timing_mode1() -> Result<(), TestError> {
        run_blargg_apu_test("./tests/roms/blargg_apu_2005.07.30/06.len_timing_mode1.nes")
    }

    #[test]
    fn blargg_apu_test_07_irq_flag_timing() -> Result<(), TestError> {
        run_blargg_apu_test("./tests/roms/blargg_apu_2005.07.30/07.irq_flag_timing.nes")
    }

    #[test]
    fn blargg_apu_test_08_irq_timing() -> Result<(), TestError> {
        run_blargg_apu_test("./tests/roms/blargg_apu_2005.07.30/08.irq_timing.nes")
    }

    // FIXME: this test is still failing
    // #[test]
    fn blargg_apu_test_09_reset_timing() -> Result<(), TestError> {
        run_blargg_apu_test("./tests/roms/blargg_apu_2005.07.30/09.reset_timing.nes")
    }

    // FIXME: this test is still failing
    // #[test]
    fn blargg_apu_test_10_len_halt_timing() -> Result<(), TestError> {
        run_blargg_apu_test("./tests/roms/blargg_apu_2005.07.30/10.len_halt_timing.nes")
    }

    // FIXME: this test is still failing
    // #[test]
    fn blargg_apu_test_11_len_reload_timing() -> Result<(), TestError> {
        run_blargg_apu_test("./tests/roms/blargg_apu_2005.07.30/11.len_reload_timing.nes")
    }
}
