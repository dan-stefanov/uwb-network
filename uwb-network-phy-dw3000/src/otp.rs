use crate::ral::regs::{DgcCfgLutData, DgcLutData};

mod addr {
    pub const LDOTUNE_CAL: u8 = 0x04;
    pub const BIASTUNE_CAL: u8 = 0x0A;
    pub const XTAL_TRIM: u8 = 0x1E;
    pub const DGC_CFG: u8 = 0x20;
    pub const DGC_LUT_CH5: u8 = 0x27;
    pub const DGC_LUT_CH9: u8 = 0x2e;
    pub const PLL_LOCK_CODE: u8 = 0x35;
}

#[allow(async_fn_in_trait)]
pub trait OtpRead {
    type Error;
    fn read_u32(&mut self, addr: u8) -> Result<u32, Self::Error>;

    fn read_u64(&mut self, addr: u8) -> Result<u64, Self::Error> {
        let word = [self.read_u32(addr)?, self.read_u32(addr + 1)?];
        Ok((word[1] as u64) << 32 | (word[0] as u64))
    }

    fn read_u8x28(&mut self, addr: u8) -> Result<[u8; 28], Self::Error> {
        let mut data = [0u8; 7 * 4];
        let (chunks, _) = data.as_chunks_mut::<4>();

        for (offset, chunk) in (0u8..).zip(chunks) {
            let word = self.read_u32(addr + offset)?;
            *chunk = word.to_le_bytes();
        }
        Ok(data)
    }

    fn ldotune_cal(&mut self) -> Result<u64, Self::Error> {
        self.read_u64(addr::LDOTUNE_CAL)
    }

    fn bias_tune(&mut self) -> Result<u32, Self::Error> {
        let value = self.read_u32(addr::BIASTUNE_CAL)?;
        Ok(value)
    }

    fn xtal_trim(&mut self) -> Result<u8, Self::Error> {
        let value = self.read_u32(addr::XTAL_TRIM)?;
        Ok((value as u8) & 0x7f)
    }

    fn rx_tune_dgc_cfg0(&mut self) -> Result<u32, Self::Error> {
        let value = self.read_u32(addr::DGC_CFG)?;
        Ok(value)
    }

    #[allow(dead_code)]
    fn rx_tune_dgc_cfg(&mut self) -> Result<DgcCfgLutData, Self::Error> {
        let data = self.read_u8x28(addr::DGC_CFG)?;
        Ok(data)
    }

    #[allow(dead_code)]
    fn rx_tune_dgc_lut_ch5(&mut self) -> Result<DgcLutData, Self::Error> {
        let data = self.read_u8x28(addr::DGC_LUT_CH5)?;
        Ok(data)
    }

    #[allow(dead_code)]
    fn rx_tune_dgc_lut_ch9(&mut self) -> Result<DgcLutData, Self::Error> {
        let data = self.read_u8x28(addr::DGC_LUT_CH9)?;
        Ok(data)
    }

    fn pll_lock_code(&mut self) -> Result<u32, Self::Error> {
        let value = self.read_u32(addr::PLL_LOCK_CODE)?;
        Ok(value)
    }
}
