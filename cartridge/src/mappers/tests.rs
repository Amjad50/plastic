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

    /// the return code is the position within the 4 details result code
    /// WRAM, PRG ROM, IRQ, and CHR ROM/RAM.
    fn run_holy_mapperel_test(filename: &str) -> Result<(), TestError> {
        let mut nes = NES::new(filename)?;
        nes.reset_cpu();

        // cannot use until infinite loop :(
        nes.clock_until_pixel_appears(194, 65, 0x38);

        for i in 0x2118..=0x211B {
            if nes.ppu_read_address(i) != 0x30 {
                return Err(TestError::ResultError((i - 0x2118 + 1) as u8));
            }
        }

        Ok(())
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

    #[test]
    fn holy_mapperel_m0_p32k_c8k_v_test() -> Result<(), TestError> {
        run_holy_mapperel_test("./tests/roms/holy-mapperel-bin-0.02/testroms/M0_P32K_C8K_V.nes")
    }

    #[test]
    fn holy_mapperel_m0_p32k_cr32k_v_test() -> Result<(), TestError> {
        run_holy_mapperel_test("./tests/roms/holy-mapperel-bin-0.02/testroms/M0_P32K_CR32K_V.nes")
    }

    #[test]
    fn holy_mapperel_m0_p32k_cr8k_v_test() -> Result<(), TestError> {
        run_holy_mapperel_test("./tests/roms/holy-mapperel-bin-0.02/testroms/M0_P32K_CR8K_V.nes")
    }

    #[test]
    fn holy_mapperel_m1_p128k_test() -> Result<(), TestError> {
        run_holy_mapperel_test("./tests/roms/holy-mapperel-bin-0.02/testroms/M1_P128K.nes")
    }

    #[test]
    fn holy_mapperel_m1_p128k_c128k_test() -> Result<(), TestError> {
        run_holy_mapperel_test("./tests/roms/holy-mapperel-bin-0.02/testroms/M1_P128K_C128K.nes")
    }

    #[test]
    fn holy_mapperel_m1_p128k_c128k_s8k_test() -> Result<(), TestError> {
        run_holy_mapperel_test(
            "./tests/roms/holy-mapperel-bin-0.02/testroms/M1_P128K_C128K_S8K.nes",
        )
    }

    #[test]
    fn holy_mapperel_m1_p128k_c128k_w8k_test() -> Result<(), TestError> {
        run_holy_mapperel_test(
            "./tests/roms/holy-mapperel-bin-0.02/testroms/M1_P128K_C128K_W8K.nes",
        )
    }

    #[test]
    fn holy_mapperel_m1_p128k_c32k_test() -> Result<(), TestError> {
        run_holy_mapperel_test("./tests/roms/holy-mapperel-bin-0.02/testroms/M1_P128K_C32K.nes")
    }

    #[test]
    fn holy_mapperel_m1_p128k_c32k_s8k_test() -> Result<(), TestError> {
        run_holy_mapperel_test("./tests/roms/holy-mapperel-bin-0.02/testroms/M1_P128K_C32K_S8K.nes")
    }

    #[test]
    fn holy_mapperel_m1_p128k_c32k_w8k_test() -> Result<(), TestError> {
        run_holy_mapperel_test("./tests/roms/holy-mapperel-bin-0.02/testroms/M1_P128K_C32K_W8K.nes")
    }

    #[test]
    fn holy_mapperel_m1_p128k_cr8k_test() -> Result<(), TestError> {
        run_holy_mapperel_test("./tests/roms/holy-mapperel-bin-0.02/testroms/M1_P128K_CR8K.nes")
    }

    // FIXME: this test is still failing
    // #[test]
    fn holy_mapperel_m1_p512k_cr8k_s32k_test() -> Result<(), TestError> {
        run_holy_mapperel_test(
            "./tests/roms/holy-mapperel-bin-0.02/testroms/M1_P512K_CR8K_S32K.nes",
        )
    }

    // FIXME: this test is still failing
    // #[test]
    fn holy_mapperel_m1_p512k_cr8k_s8k_test() -> Result<(), TestError> {
        run_holy_mapperel_test("./tests/roms/holy-mapperel-bin-0.02/testroms/M1_P512K_CR8K_S8K.nes")
    }

    // FIXME: this test is still failing
    // #[test]
    fn holy_mapperel_m1_p512k_s32k_test() -> Result<(), TestError> {
        run_holy_mapperel_test("./tests/roms/holy-mapperel-bin-0.02/testroms/M1_P512K_S32K.nes")
    }

    // FIXME: this test is still failing
    // #[test]
    fn holy_mapperel_m1_p512k_s8k_test() -> Result<(), TestError> {
        run_holy_mapperel_test("./tests/roms/holy-mapperel-bin-0.02/testroms/M1_P512K_S8K.nes")
    }

    #[test]
    fn holy_mapperel_m2_p128k_cr8k_v_test() -> Result<(), TestError> {
        run_holy_mapperel_test("./tests/roms/holy-mapperel-bin-0.02/testroms/M2_P128K_CR8K_V.nes")
    }

    #[test]
    fn holy_mapperel_m2_p128k_v_test() -> Result<(), TestError> {
        run_holy_mapperel_test("./tests/roms/holy-mapperel-bin-0.02/testroms/M2_P128K_V.nes")
    }

    #[test]
    fn holy_mapperel_m3_p32k_c32k_h_test() -> Result<(), TestError> {
        run_holy_mapperel_test("./tests/roms/holy-mapperel-bin-0.02/testroms/M3_P32K_C32K_H.nes")
    }

    #[test]
    fn holy_mapperel_m4_p128k_test() -> Result<(), TestError> {
        run_holy_mapperel_test("./tests/roms/holy-mapperel-bin-0.02/testroms/M4_P128K.nes")
    }

    #[test]
    fn holy_mapperel_m4_p128k_cr32k_test() -> Result<(), TestError> {
        run_holy_mapperel_test("./tests/roms/holy-mapperel-bin-0.02/testroms/M4_P128K_CR32K.nes")
    }

    #[test]
    fn holy_mapperel_m4_p128k_cr8k_test() -> Result<(), TestError> {
        run_holy_mapperel_test("./tests/roms/holy-mapperel-bin-0.02/testroms/M4_P128K_CR8K.nes")
    }

    #[test]
    fn holy_mapperel_m4_p256k_c256k_test() -> Result<(), TestError> {
        run_holy_mapperel_test("./tests/roms/holy-mapperel-bin-0.02/testroms/M4_P256K_C256K.nes")
    }

    #[test]
    fn holy_mapperel_m7_p128k_test() -> Result<(), TestError> {
        run_holy_mapperel_test("./tests/roms/holy-mapperel-bin-0.02/testroms/M7_P128K.nes")
    }

    #[test]
    fn holy_mapperel_m7_p128k_cr8k_test() -> Result<(), TestError> {
        run_holy_mapperel_test("./tests/roms/holy-mapperel-bin-0.02/testroms/M7_P128K_CR8K.nes")
    }

    #[test]
    fn holy_mapperel_m9_p128k_c64k_test() -> Result<(), TestError> {
        run_holy_mapperel_test("./tests/roms/holy-mapperel-bin-0.02/testroms/M9_P128K_C64K.nes")
    }

    // FIXME: this test is still failing
    // #[test]
    fn holy_mapperel_m10_p128k_c64k_s8k_test() -> Result<(), TestError> {
        run_holy_mapperel_test(
            "./tests/roms/holy-mapperel-bin-0.02/testroms/M10_P128K_C64K_S8K.nes",
        )
    }

    // FIXME: this test is still failing
    // #[test]
    fn holy_mapperel_m10_p128k_c64k_w8k_test() -> Result<(), TestError> {
        run_holy_mapperel_test(
            "./tests/roms/holy-mapperel-bin-0.02/testroms/M10_P128K_C64K_W8K.nes",
        )
    }

    // FIXME: this test is still failing
    // #[test]
    fn holy_mapperel_m11_p64k_c64k_v_test() -> Result<(), TestError> {
        run_holy_mapperel_test("./tests/roms/holy-mapperel-bin-0.02/testroms/M11_P64K_C64K_V.nes")
    }

    // FIXME: this test is still failing
    // #[test]
    fn holy_mapperel_m11_p64k_cr32k_v_test() -> Result<(), TestError> {
        run_holy_mapperel_test("./tests/roms/holy-mapperel-bin-0.02/testroms/M11_P64K_CR32K_V.nes")
    }

    // FIXME: this test is still failing
    // #[test]
    fn holy_mapperel_m28_p512k_test() -> Result<(), TestError> {
        run_holy_mapperel_test("./tests/roms/holy-mapperel-bin-0.02/testroms/M28_P512K.nes")
    }

    // FIXME: this test is still failing
    // #[test]
    fn holy_mapperel_m28_p512k_cr32k_test() -> Result<(), TestError> {
        run_holy_mapperel_test("./tests/roms/holy-mapperel-bin-0.02/testroms/M28_P512K_CR32K.nes")
    }

    // FIXME: this test is still failing
    // #[test]
    fn holy_mapperel_m34_p128k_cr8k_h_test() -> Result<(), TestError> {
        run_holy_mapperel_test("./tests/roms/holy-mapperel-bin-0.02/testroms/M34_P128K_CR8K_H.nes")
    }

    // FIXME: this test is still failing
    // #[test]
    fn holy_mapperel_m34_p128k_h_test() -> Result<(), TestError> {
        run_holy_mapperel_test("./tests/roms/holy-mapperel-bin-0.02/testroms/M34_P128K_H.nes")
    }

    // FIXME: this test is still failing
    // #[test]
    fn holy_mapperel_m66_p64k_c16k_v_test() -> Result<(), TestError> {
        run_holy_mapperel_test("./tests/roms/holy-mapperel-bin-0.02/testroms/M66_P64K_C16K_V.nes")
    }

    // FIXME: this test is still failing
    // #[test]
    fn holy_mapperel_m69_p128k_c64k_s8k_test() -> Result<(), TestError> {
        run_holy_mapperel_test(
            "./tests/roms/holy-mapperel-bin-0.02/testroms/M69_P128K_C64K_S8K.nes",
        )
    }

    // FIXME: this test is still failing
    // #[test]
    fn holy_mapperel_m69_p128k_c64k_w8k_test() -> Result<(), TestError> {
        run_holy_mapperel_test(
            "./tests/roms/holy-mapperel-bin-0.02/testroms/M69_P128K_C64K_W8K.nes",
        )
    }

    // FIXME: this test is still failing
    // #[test]
    fn holy_mapperel_m78_3_p128k_c64k_test() -> Result<(), TestError> {
        run_holy_mapperel_test("./tests/roms/holy-mapperel-bin-0.02/testroms/M78.3_P128K_C64K.nes")
    }

    // FIXME: this test is still failing
    // #[test]
    fn holy_mapperel_m118_p128k_c64k_test() -> Result<(), TestError> {
        run_holy_mapperel_test("./tests/roms/holy-mapperel-bin-0.02/testroms/M118_P128K_C64K.nes")
    }

    // FIXME: this test is still failing
    // #[test]
    fn holy_mapperel_m180_p128k_cr8k_h_test() -> Result<(), TestError> {
        run_holy_mapperel_test("./tests/roms/holy-mapperel-bin-0.02/testroms/M180_P128K_CR8K_H.nes")
    }

    // FIXME: this test is still failing
    // #[test]
    fn holy_mapperel_m180_p128k_h_test() -> Result<(), TestError> {
        run_holy_mapperel_test("./tests/roms/holy-mapperel-bin-0.02/testroms/M180_P128K_H.nes")
    }
}
