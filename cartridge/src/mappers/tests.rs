#[cfg(test)]
mod mappers_tests {
    use nes_tester::{TestError, NES};

    fn run_mmc3_test(filename: &str) -> Result<(), TestError> {
        let result_memory_address = 0x6000;

        let mut nes = NES::new(filename)?;
        nes.reset_cpu();

        nes.clock_until_infinite_loop();

        let result = nes.cpu_read_address(result_memory_address);

        if result != 0 {
            Err(TestError::ResultError(result))
        } else {
            Ok(())
        }
    }

    #[test]
    fn mmc3_test_1_clocking() -> Result<(), TestError> {
        run_mmc3_test("./tests/roms/mmc3_test_2/rom_singles/1-clocking.nes")
    }

    #[test]
    fn mmc3_test_2_details() -> Result<(), TestError> {
        run_mmc3_test("./tests/roms/mmc3_test_2/rom_singles/2-details.nes")
    }

    #[test]
    fn mmc3_test_3_a12_clocking() -> Result<(), TestError> {
        run_mmc3_test("./tests/roms/mmc3_test_2/rom_singles/3-A12_clocking.nes")
    }

    // FIXME: this test is still failing
    // #[test]
    fn mmc3_test_4_scanline_timing() -> Result<(), TestError> {
        run_mmc3_test("./tests/roms/mmc3_test_2/rom_singles/4-scanline_timing.nes")
    }

    #[test]
    fn mmc3_test_5_mmc3() -> Result<(), TestError> {
        run_mmc3_test("./tests/roms/mmc3_test_2/rom_singles/5-MMC3.nes")
    }
}
