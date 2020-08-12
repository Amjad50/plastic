#[cfg(test)]
mod ppu_tests {
    use nes_tester::{TestError, NES};

    fn run_test(filename: &str, result_memory_address: u16) -> Result<(), TestError> {
        let mut nes = NES::new(filename)?;
        nes.reset_cpu();

        nes.clock_until_infinite_loop();

        let result = nes.cpu_read_address(result_memory_address);

        if result != 1 {
            Err(TestError::ResultError(result))
        } else {
            Ok(())
        }
    }

    fn run_sprite_hit_test(filename: &str) -> Result<(), TestError> {
        let result_memory_address = 0x00F8;

        let mut nes = NES::new(filename)?;
        nes.reset_cpu();

        // this is the top-left pixel of the word "PASSED" or "FAILED"
        nes.clock_until_pixel_appears(17, 48, 0x30);

        let result = nes.cpu_read_address(result_memory_address);

        if result != 1 {
            Err(TestError::ResultError(result))
        } else {
            Ok(())
        }
    }

    fn run_ppu_vbl_nmi_test(filename: &str) -> Result<(), TestError> {
        let result_memory_address = 0x6000;

        let mut nes = NES::new(filename)?;
        nes.reset_cpu();

        // first loop until an infnite loop (this infinite loop might be the
        // end or not), then loop until the value of `0x6000` is not `0x80`
        // the reason we can't loop until memory_neq from the beginning
        // is because the ram starts with all zeros, so it will stop after the
        // first instruction
        nes.clock_until_infinite_loop();
        // the default is 0x80, when the rom starts
        nes.clock_until_memory_neq(0x6000, 0x80);

        let result = nes.cpu_read_address(result_memory_address);

        if result != 0 {
            Err(TestError::ResultError(result))
        } else {
            Ok(())
        }
    }

    #[test]
    fn blargg_ppu_test_palette_ram() -> Result<(), TestError> {
        run_test("./tests/roms/blargg_ppu_tests/palette_ram.nes", 0x00f0)
    }

    #[test]
    fn blargg_ppu_test_power_up_palette() -> Result<(), TestError> {
        run_test("./tests/roms/blargg_ppu_tests/power_up_palette.nes", 0x00f0)
    }

    #[test]
    fn blargg_ppu_test_sprite_ram() -> Result<(), TestError> {
        run_test("./tests/roms/blargg_ppu_tests/sprite_ram.nes", 0x00f0)
    }

    #[test]
    fn blargg_ppu_test_vbl_clear_time() -> Result<(), TestError> {
        let filename = "./tests/roms/blargg_ppu_tests/vbl_clear_time.nes";
        let result_memory_address = 0x00f0;

        let mut nes = NES::new(filename)?;
        nes.reset_cpu();

        // 2 NMIs should occure
        nes.clock_until_nmi();
        nes.clock_until_nmi();
        nes.clock_until_infinite_loop();

        let result = nes.cpu_read_address(result_memory_address);

        if result != 1 {
            Err(TestError::ResultError(result))
        } else {
            Ok(())
        }
    }

    #[test]
    fn blargg_ppu_test_vram_access() -> Result<(), TestError> {
        run_test("./tests/roms/blargg_ppu_tests/vram_access.nes", 0x00f0)
    }

    #[test]
    fn sprite_hit_test_01_basics() -> Result<(), TestError> {
        run_sprite_hit_test("./tests/roms/sprite_hit_tests/01.basics.nes")
    }

    #[test]
    fn sprite_hit_test_02_alignment() -> Result<(), TestError> {
        run_sprite_hit_test("./tests/roms/sprite_hit_tests/02.alignment.nes")
    }

    #[test]
    fn sprite_hit_test_03_corners() -> Result<(), TestError> {
        run_sprite_hit_test("./tests/roms/sprite_hit_tests/03.corners.nes")
    }

    #[test]
    fn sprite_hit_test_04_flip() -> Result<(), TestError> {
        run_sprite_hit_test("./tests/roms/sprite_hit_tests/04.flip.nes")
    }

    #[test]
    fn sprite_hit_test_05_left_clip() -> Result<(), TestError> {
        run_sprite_hit_test("./tests/roms/sprite_hit_tests/05.left_clip.nes")
    }

    #[test]
    fn sprite_hit_test_06_right_edge() -> Result<(), TestError> {
        run_sprite_hit_test("./tests/roms/sprite_hit_tests/06.right_edge.nes")
    }

    #[test]
    fn sprite_hit_test_07_screen_bottom() -> Result<(), TestError> {
        run_sprite_hit_test("./tests/roms/sprite_hit_tests/07.screen_bottom.nes")
    }

    #[test]
    fn sprite_hit_test_08_double_height() -> Result<(), TestError> {
        run_sprite_hit_test("./tests/roms/sprite_hit_tests/08.double_height.nes")
    }

    // FIXME: this test is still failing
    // #[test]
    fn sprite_hit_test_09_timing_basics() -> Result<(), TestError> {
        run_sprite_hit_test("./tests/roms/sprite_hit_tests/09.timing_basics.nes")
    }

    #[test]
    fn sprite_hit_test_10_timing_order() -> Result<(), TestError> {
        run_sprite_hit_test("./tests/roms/sprite_hit_tests/10.timing_order.nes")
    }

    #[test]
    fn sprite_hit_test_11_edge_timing() -> Result<(), TestError> {
        run_sprite_hit_test("./tests/roms/sprite_hit_tests/11.edge_timing.nes")
    }

    #[test]
    fn ppu_vbl_nmi_test_01_vbl_basics() -> Result<(), TestError> {
        run_ppu_vbl_nmi_test("./tests/roms/ppu_vbl_nmi/rom_singles/01-vbl_basics.nes")
    }

    #[test]
    fn ppu_vbl_nmi_test_02_vbl_set_time() -> Result<(), TestError> {
        run_ppu_vbl_nmi_test("./tests/roms/ppu_vbl_nmi/rom_singles/02-vbl_set_time.nes")
    }

    // FIXME: this test is still failing
    // #[test]
    fn ppu_vbl_nmi_test_03_vbl_clear_time() -> Result<(), TestError> {
        run_ppu_vbl_nmi_test("./tests/roms/ppu_vbl_nmi/rom_singles/03-vbl_clear_time.nes")
    }

    #[test]
    fn ppu_vbl_nmi_test_04_nmi_control() -> Result<(), TestError> {
        run_ppu_vbl_nmi_test("./tests/roms/ppu_vbl_nmi/rom_singles/04-nmi_control.nes")
    }

    // FIXME: this test is still failing
    // #[test]
    fn ppu_vbl_nmi_test_05_nmi_timing() -> Result<(), TestError> {
        run_ppu_vbl_nmi_test("./tests/roms/ppu_vbl_nmi/rom_singles/05-nmi_timing.nes")
    }

    // FIXME: this test is still failing
    // #[test]
    fn ppu_vbl_nmi_test_06_suppression() -> Result<(), TestError> {
        run_ppu_vbl_nmi_test("./tests/roms/ppu_vbl_nmi/rom_singles/06-suppression.nes")
    }

    #[test]
    fn ppu_vbl_nmi_test_07_nmi_on_timing() -> Result<(), TestError> {
        run_ppu_vbl_nmi_test("./tests/roms/ppu_vbl_nmi/rom_singles/07-nmi_on_timing.nes")
    }

    // FIXME: this test is still failing
    // #[test]
    fn ppu_vbl_nmi_test_08_nmi_off_timing() -> Result<(), TestError> {
        run_ppu_vbl_nmi_test("./tests/roms/ppu_vbl_nmi/rom_singles/08-nmi_off_timing.nes")
    }

    #[test]
    fn ppu_vbl_nmi_test_09_even_odd_frames() -> Result<(), TestError> {
        run_ppu_vbl_nmi_test("./tests/roms/ppu_vbl_nmi/rom_singles/09-even_odd_frames.nes")
    }

    #[test]
    fn ppu_vbl_nmi_test_10_even_odd_timing() -> Result<(), TestError> {
        run_ppu_vbl_nmi_test("./tests/roms/ppu_vbl_nmi/rom_singles/10-even_odd_timing.nes")
    }
}
