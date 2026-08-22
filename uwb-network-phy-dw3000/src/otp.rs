use crate::interface::Interface;
use crate::ral::{Error, RegisterAccess, regs};

mod addr {
    pub const LDOTUNE_CAL: u8 = 0x04;
    pub const BIASTUNE_CAL: u8 = 0x0A;
    pub const XTAL_TRIM: u8 = 0x1E;
    pub const DGC_CFG: u8 = 0x20;
    pub const DGC_LUT_CH5: u8 = 0x27;
    pub const DGC_LUT_CH9: u8 = 0x2e;
    pub const PLL_LOCK_CODE: u8 = 0x35;
}

pub struct OtpReader<'a, IF> {
    interface: &'a mut IF,
}

impl<'a, IF: Interface> OtpReader<'a, IF> {
    pub fn new(interface: &'a mut IF) -> Self {
        Self { interface }
    }

    fn read_u32(&mut self, addr: u8) -> Result<u32, Error<IF>> {
        let mut ral = RegisterAccess::new(self.interface);
        // set manual access mode
        ral.otp_cfg().write(|w| w.set_otp_man(true))?;
        // set the address
        ral.otp_addr().write(|w| w.set_otp_addr(addr as u16))?;
        // assert the read strobe
        ral.otp_cfg().write(|w| w.set_otp_read(true))?;
        // read the result
        let res = ral.otp_rdata().read_bytes()?;
        Ok(res)
    }

    fn read_u64(&mut self, addr: u8) -> Result<u64, Error<IF>> {
        let word = [self.read_u32(addr)?, self.read_u32(addr + 1)?];
        Ok((word[1] as u64) << 32 | (word[0] as u64))
    }

    fn read_u8x28(&mut self, addr: u8) -> Result<[u8; 28], Error<IF>> {
        let mut data = [0u8; 7 * 4];
        let (chunks, _) = data.as_chunks_mut::<4>();

        for (offset, chunk) in (0u8..).zip(chunks) {
            let word = self.read_u32(addr + offset)?;
            *chunk = word.to_le_bytes();
        }
        Ok(data)
    }

    pub fn ldotune_cal(&mut self) -> Result<u64, Error<IF>> {
        self.read_u64(addr::LDOTUNE_CAL)
    }

    pub fn bias_tune(&mut self) -> Result<u32, Error<IF>> {
        let value = self.read_u32(addr::BIASTUNE_CAL)?;
        Ok(value)
    }

    pub fn xtal_trim(&mut self) -> Result<u8, Error<IF>> {
        let value = self.read_u32(addr::XTAL_TRIM)?;
        Ok((value as u8) & 0x7f)
    }

    pub fn rx_tune_dgc_cfg0(&mut self) -> Result<u32, Error<IF>> {
        let value = self.read_u32(addr::DGC_CFG)?;
        Ok(value)
    }

    #[allow(dead_code)]
    pub fn rx_tune_dgc_cfg(&mut self) -> Result<regs::DgcCfgLutData, Error<IF>> {
        let data = self.read_u8x28(addr::DGC_CFG)?;
        Ok(data)
    }

    #[allow(dead_code)]
    pub fn rx_tune_dgc_lut_ch5(&mut self) -> Result<regs::DgcLutData, Error<IF>> {
        let data = self.read_u8x28(addr::DGC_LUT_CH5)?;
        Ok(data)
    }

    #[allow(dead_code)]
    pub fn rx_tune_dgc_lut_ch9(&mut self) -> Result<regs::DgcLutData, Error<IF>> {
        let data = self.read_u8x28(addr::DGC_LUT_CH9)?;
        Ok(data)
    }

    pub fn pll_lock_code(&mut self) -> Result<u32, Error<IF>> {
        let value = self.read_u32(addr::PLL_LOCK_CODE)?;
        Ok(value)
    }
}
