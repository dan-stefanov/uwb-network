use super::*;
use paste::paste;
use bitflags::bitflags;


#[repr(u8)]
#[derive(Clone, Copy, Eq, PartialEq)]
pub enum PhrMode {
    /// Standard Frame mode as per IEEE802.15.4 standard
    StandardFrame = 0x0,
    /// Long Frame mode encoding as per IEEE802.15.8 standard
    LongFrame = 0x1,
}

#[repr(u8)]
#[derive(Clone, Copy, Eq, PartialEq)]
pub enum StsPocketPosition {
    /// No STS in the packet
    NoSts = 0,
    /// The STS is between the SDF and the PHR
    AfterData = 1,
    /// The STS is after the data, (i.e. at the end of the packet)
    BeforeData = 2,
    /// The STS is after the SDF but there is no PHR or data
    StsOnly = 3,
}

#[repr(u8)]
#[derive(Clone, Copy, Eq, PartialEq)]
pub enum PdoaMode {
    Disable = 0x0,
    Mode1 = 0x1,
    _Reserved2 = 0x2,
    Mode3 = 0x3,
}

#[repr(u8)]
#[derive(Clone, Copy, Eq, PartialEq)]
pub enum ReceiverParameterSet {
    /// Parameter set 0, optimized for long preamble
    Long = 0x0,
    /// Parameter set 1, reserved
    _Reserved1 = 0x1,
    /// Default parameter set, optimized for short preamble
    Short = 0x2,
    _Reserved3 = 0x3,
}

#[repr(u8)]
#[derive(Clone, Copy, Eq, PartialEq)]
pub enum Channel {
    Channel5 = 0x0,
    Channel9 = 0x1,
}

#[repr(u8)]
#[derive(Clone, Copy, Eq, PartialEq)]
pub enum SfdType {
    IeeeSfd0 = 0,
    Decawave8 = 1,
    Decawave16 = 2,
    IeeeSfd2 = 3,
}

#[repr(u8)]
#[derive(Clone, Copy, Eq, PartialEq)]
pub enum ClockSource {
    /// Fastest available clock: FAST_RC/4, FAST_RC, PLL
    Auto = 0x0,
    /// FAST_RC/4, about 30MHz
    FastRcDiv4 = 0x1,
    /// PLL clock, 125 MHz
    Pll = 0x2,
    /// FAST_RC, about 120MHz
    FastRc = 0x3,
}

#[repr(u8)]
#[derive(Clone, Copy, Eq, PartialEq)]
pub enum BitRate {
    Kbs850 = 0,
    Kbs6810 = 1,
}

#[repr(u8)]
#[derive(Clone, Copy, Eq, PartialEq)]
pub enum TxPreambleLength {
    Symbols16 = 0b0000, // undocumented
    Symbols64 = 0b0001,
    Symbols1024 = 0b0010,
    Symbols4096 = 0b0011,
    Symbols32 = 0b0100,
    Symbols128 = 0b0101,
    Symbols1536 = 0b0110,
    _Reserved7 = 0b0111,
    _Reserved8 = 0b1000,
    Symbols256 = 0b1001,
    Symbols2048 = 0b1010,
    _Reserved11 = 0b1011,
    _Reserved12 = 0b1100,
    Symbols512 = 0b1101,
    _Reserved14 = 0b1110,
    _Reserved15 = 0b1111,
}

#[repr(u8)]
#[derive(Clone, Copy, Eq, PartialEq)]
pub enum RxPreambleLength {
    Symbols16 = 0b00,
    Symbols64 = 0b01,
    Symbols1024 = 0b10,
    Symbols4096 = 0b11,
}

#[repr(u8)]
#[derive(Clone, Copy, Eq, PartialEq)]
pub enum Pac {
    Symbols8 = 0,
    Symbols16 = 1,
    Symbols32 = 2,
    Symbols4 = 3,
}

#[repr(u8)]
#[derive(Clone, Copy, Eq, PartialEq)]
pub enum CalibrationMode {
    NormalOperation = 0,
    Calibration = 1,
    _Reserved2 = 2,
    _Reserved3 = 3,
}

#[repr(u8)]
#[derive(Clone, Copy, Eq, PartialEq)]
pub enum RxPfr {
    _Reserved00 = 0b00,
    Mhz16 = 0b01,
    Mhz64 = 0b10,
    _Reserved11 = 0b11,
}


bitflags! {
    #[derive(Clone, Copy, PartialEq, Eq)]
    pub struct EventsLow: u32 {
        /// Aggregate IRQ status, read-only
        const IRQS = 1 << 0;
        /// Clock PLL lock
        const CPLOCK = 1 << 1;
        /// SPI CRC error
        const SPICRCE = 1 << 2;
        /// Automatic acknowledge trigger
        const ATT = 1 << 3;
        /// Transmit frame begins
        const TXFRB = 1 << 4;
        /// Transmit preamble sent
        const TXPRS = 1 << 5;
        /// Transmit PHY header sent
        const TXPHS = 1 << 6;
        /// Transmit frame sent
        const TXFRS = 1 << 7;
        /// Receiver preamble detected
        const RXPRD = 1 << 8;
        /// Receiver SFD detected
        const RXSFDD = 1 << 9;
        /// CIA processing done
        const CIADONE = 1 << 10;
        /// Receiver PHY header detected
        const RXPHD = 1 << 11;
        /// Receiver PHY header error
        const RXPHE = 1 << 12;
        /// Receiver frame ready
        const RXFR = 1 << 13;
        /// Receiver FCS good
        const RXFCG = 1 << 14;
        /// Receiver FCS error
        const RXFCE = 1 << 15;
        /// Receiver Reed-Solomon frame sync loss
        const RXFSL = 1 << 16;
        /// Receiver frame wait timeout
        const RXFTO = 1 << 17;
        /// CIA processing error
        const CIAERR = 1 << 18;
        /// Low voltage warning
        const VWARN = 1 << 19;
        /// Receiver overrun
        const RXOVRR = 1 << 20;
        /// Preamble detection timeout
        const RXPTO = 1 << 21;
        /// SPI ready for host access
        const SPIRDY = 1 << 23;
        /// Switch to RC_INIT state
        const RCINIT = 1 << 24;
        /// Clock PLL losing lock
        const PLL_HILO = 1 << 25;
        /// Receiver SFD timeout
        const RXSTO = 1 << 26;
        /// Half period delay warning
        const HPDWARN = 1 << 27;
        /// STS error
        const CPERR = 1 << 28;
        /// Automatic frame filtering rejection
        const ARFE = 1 << 29;
    }
}

bitflags! {
    #[derive(Clone, Copy, PartialEq, Eq)]
    pub struct EventsHigh: u16 {
        /// Receiver preamble rejection
        const RXPREJ = 1 << 1;
        /// Voltage or temperature variation detected
        const VT_DET = 1 << 4;
        /// GPIO interrupt
        const GPIOIRQ = 1 << 5;
        /// AES_DMA operation complete
        const AES_DONE = 1 << 6;
        /// AES_DMA error
        const AES_ERR = 1 << 7;
        /// Command error
        const CMD_ERR = 1 << 8;
        /// SPI overflow error
        const SPI_OVF = 1 << 9;
        /// SPI underflow error
        const SPI_UNF = 1 << 10;
        /// SPI collision error
        const SPIERR = 1 << 11;
        /// Transmit CCA rejection
        const CCA_FAIL = 1 << 12;
    }
}

bitflags! {
    #[derive(Clone, Copy, PartialEq, Eq)]
    pub struct BriefEvents: u8 {
        /// TXFRB or TXPRS or TXPHS or TXFRS
        const TXOK = 1 << 0;
        /// AAT or CCA_FAIL
        const CCA_FAIL = 1 << 1;
        /// CIAERR
        const RXTSERR = 1 << 2;
        /// RXFR and CIADONE or RXFCG
        const RXOK = 1 << 3;
        /// RXFCE or RXFSL or RXPHE or ARFE or RXSTO or RXOVRR
        const RXERR = 1 << 4;
        /// RXFTO or RXPTO
        const RXTO = 1 << 5;
        /// VT_DET or GPIOIRQ or RCINIT or SPIRDY
        const SYS_EVENT = 1 << 6;
        /// AES_ERR or CMD_ERR or SPI_UNF or SPI_OVF or SPIERR or PLL_HILO or VWARN
        const SYS_PANIC = 1 << 7;
    }
}

bitflags! {
    #[derive(Clone, Copy, PartialEq, Eq)]
    pub struct CalStatus: u8 {
        const CAL_DONE = 1;
    }
}

bitflags! {
    #[derive(Clone, Copy, PartialEq, Eq)]
    pub struct DbEvents: u8 {
        /// Receiver FCS good
        const RXFCG = 1 << 0;
        /// Receiver frame ready
        const RXFR = 1 << 1;
        /// CIA processing done
        const CIADONE = 1 << 2;
        /// STS error
        const CP_ERR = 1 << 3;
    }
}

pub type DgcCfgLutData = [u8; 28];  // 7 x 4 byte words
pub type DgcLutData = [u8; 28];     // 7 x 4 byte words

pub const SWING_SET_1_OFFSET: u16 = 0x00;
pub const SWING_SET_2_OFFSET: u16 = 0xE8;

reg_field!(0x00, 0x00, 4, DevId, RO, (), "Device Identifier");
reg_bytes!(0x00, 0x04, 8, Eui, RW, (), "Extended Unique Identifier");

reg_field!(0x00, 0x0c, 4, Panadr, RW, 0xffff_ffff, "PAN Identifier and Short Address");
reg_field!(0x00, 0x10, 4, SysCfg, RW, 0x0004_1188, "System Configuration");
reg_field!(0x00, 0x14, 2, FfCfg, RW, 0, "Frame filter configuration");
reg_bytes!(0x00, 0x1C, 4, SysTime, RO, (), "System Time Counter");
reg_field!(0x00, 0x24, 6, TxFctrl, RW, 0x0000_0000_1c0c, "Transmit frame control");
reg_field!(0x00, 0x24, 2, TxFctrlShort, RW, 0x1c0c, "Transmit frame control, bytes 0..=1");
reg_bytes!(0x00, 0x2c, 4, DxTime, RW, (), "Delayed send or receive time");
reg_bytes!(0x00, 0x30, 4, DrefTime, RW, (), "Delayed send or receive reference time");
reg_field!(0x00, 0x34, 3, RxFwto, RW, 0, "Receive frame wait timeout");
reg_field!(0x00, 0x4c, 4, RxFinfo, RO, (), "RX frame information");
// Chip does not support 16 byte access for RX_TIME register
reg_bytes!(0x00, 0x64, 5, RxTime, RO, (), "Receive accurate time stamp"); 
reg_bytes!(0x00, 0x70, 4, RxRawst, RO, (), "Receive raw time stamp");

reg_bytes!(0x00, 0x3c, 4, SysEnableLow, RW, (), "System event enable mask, bytes 0..=3");
reg_bytes!(0x00, 0x40, 2, SysEnableHigh, RW, (), "System event enable mask, bytes 4..=5");

reg_bytes!(0x00, 0x44, 4, SysStatusLow, RC, (), "System event status, bytes 0..=3");
reg_bytes!(0x00, 0x44, 2, SysStatusHigh, RC, (), "System event status, bytes 4..=5");

reg_field!(0x01, 0x08, 4, AckRespT, RW, 0x0000_0000, "Acknowledgement delay time and response time");
reg_field!(0x01, 0x0c, 4, TxPower, RW, 0x8282_8282, "Transmit power control");
reg_field!(0x01, 0x14, 2, ChanCtrl, RW, 0x0006, "Channel control register");
reg_field!(0x01, 0x24, 1, RdbStatus, RC, (), "RX double buffer status");

reg_field!(0x03, 0x18, 2, DgcCfg, RW, 0xf0f5, "The RX tuning configuration register");

reg_field!(0x04, 0x0c, 4, RxCal, RW, 0x0000_0000, "RX calibration block configuration");
reg_field!(0x04, 0x14, 4, RxCalResi, RW, (), "RX calibration block result I");
reg_field!(0x04, 0x1c, 4, RxCalResq, RW, (), "RX calibration block result Q");
reg_bytes!(0x04, 0x20, 1, RxCalSts, RC, (), "RX calibration status");

reg_field!(0x06, 0x00, 2, Dtune0, RW, 0x101c, "Digital tuning register 0");
reg_bytes!(0x06, 0x02, 2, RxSfdToc, RW, (), "SFD detection timeout");
reg_bytes!(0x06, 0x04, 2, PreToc, RW, (), "Preamble detection timeout");
reg_bytes!(0x06, 0x0c, 4, Dtune3, RW, (), "Digital tuning register 3");

reg_bytes!(0x07, 0x10, 4, RfRxCtrlHi, RW, (), "Analog RX control register");
reg_bytes!(0x07, 0x1a, 1, RfTxCtrl1, RW, (), "Analog TX control register");
reg_bytes!(0x07, 0x1c, 4, RfTxCtrl2, RW, (), "Analog TX control register");
reg_field!(0x07, 0x40, 8, LdoTune, RW, (), "Internal LDO voltage tuning parameter");
reg_field!(0x07, 0x48, 4, LdoCtrl, RW, (), "Internal LDO control");
reg_bytes!(0x07, 0x51, 1, LdoRload, RW, (), "LDO tuning register");

reg_field!(0x08, 0x00, 1, SarCtrl, RW, 0x00, "SAR (sensor ADC) control");

reg_bytes!(0x09, 0x00, 2, PllCfg, RW, (), "PLL configuration");
reg_field!(0x09, 0x04, 4, PllCc, RW, (), "PLL coarse code");
reg_field!(0x09, 0x08, 2, PllCal, RW, 0x0031, "PLL calibration configuration");
reg_field!(0x09, 0x14, 1, Xtal, RW, 0x00, "Frequency synthesizer, crystal trim");

reg_bytes!(0x0b, 0x00, 4, OtpWdata, RW, (), "OTP data to program to a particular address");
reg_field!(0x0b, 0x04, 2, OtpAddr, RW, 0x0000, "OTP address to which to program the data");
reg_field!(0x0b, 0x08, 2, OtpCfg, RW, 0x0000, "OTP configuration register");
reg_field!(0x0b, 0x0c, 1, OtpStat, RO, (), "OTP memory programming status register");
reg_bytes!(0x0b, 0x10, 4, OtpRdata, RO, (), "OTP data read from given address");
reg_bytes!(0x0b, 0x14, 4, OtpSrdata, RO, (), "OTP Special Register (SR) read data");

reg_field!(0x0f, 0x30, 4, SysState, RO, (), "System states");

reg_field!(0x11, 0x00, 2, SoftRst, RW, 0x01ff, "Soft reset of the device blocks");
reg_field!(0x11, 0x04, 4, ClkCtrl, RW, 0xf030_0200, "PMSC clock control register");
reg_field!(0x11, 0x08, 4, SeqCtrl, RW, 0x8003_0638, "PMSC sequencing control register");
reg_field!(0x11, 0x1f, 2, BiasCtrl, RW, 0x1070, "Analog blocks calibration values");

// Swing RX register set are mapped to pointer B space
// Set PtrAddrB = DbDiag, PtrOffsetB = SWING_SET_X_OFFSET to access
reg_field!(0x1E, 0x00, 4, DbRxFinfo, RO, (), "RX frame information");
reg_bytes!(0x1E, 0x04, 5, DbRxTime, RO, (), "Receive accurate time stamp");

reg_bytes!(0x1f, 0x00, 1, FintStat, RO, (), "Fast system event status register");
reg_field!(0x1f, 0x04, 1, PtrAddrA, RW, 0x00, "Indirect pointer A register ID");
reg_field!(0x1f, 0x08, 2, PtrOffsetA, RW, 0x0000, "Indirect pointer A register offset");
reg_field!(0x1f, 0x0c, 1, PtrAddrB, RW, 0x00, "Indirect pointer B register ID");
reg_field!(0x1f, 0x10, 2, PtrOffsetB, RW, 0x0000, "Indirect pointer B register offset");

// UG prescribes filling the fist 8 bytes of DgcCfgLut
reg_file!(0x03, 0x1c, 28, DgcCfgLut, RW, "Receiver tuning parameters");  
reg_file!(0x03, 0x38, 28, DgcLut, RW, "Receiver tuning parameters");

reg_file!(0x12, (), 1024, RxBuffer0, RO, "Rx frame data buffer 0");
reg_file!(0x13, (), 1024, RxBuffer1, RO, "Rx frame data buffer 1");
reg_file!(0x14, (), 1024, TxBuffer, WO, "Tx frame data buffer");
reg_file!(0x15, (), 12288, AccMem, RO, "CIR accumulator data"); // Note different offset step and leading dummy byte
reg_file!(0x17, (), 127, ScratchMem, RW, "Scratch RAM buffer");
reg_file!(0x18, (), 464, DbDiag, RW, "Double buffer diagnostic register set");
reg_file!(0x1D, (), 12288, IndirectPtrA, RW, "Indirect pointer A buffer");
reg_file!(0x1E, (), 12288, IndirectPtrB, RW, "Indirect pointer B buffer");

field_prim!(DevId, 0, 4, rev, u8, "Revision");
field_prim!(DevId, 4, 8, ver, u8, "Version");
field_prim!(DevId, 8, 16, model, u8, "Model");
field_prim!(DevId, 16, 32, ridtag, u16, "Register Identification Tag");

field_prim!(Panadr, 0, 16, shortaddr, u16, "Short address");
field_prim!(Panadr, 16, 32, pan_id, u16, "PAN identifier");

field_bool!(SysCfg, 0, ffen, "Frame Filtering Enable");
field_bool!(SysCfg, 1, dis_fcs_tx, "Disable auto-FCS Transmission");
field_bool!(SysCfg, 2, dis_fce, "Disable frame check error handling");
field_bool!(SysCfg, 3, dis_drxb, "Disable Double RX Buffer");
field_enum!(SysCfg, 4, 5, phr_mode, u8, PhrMode, "PHR mode");
field_bool!(SysCfg, 5, phr_6m8, "6.81 Mb/s data for PHR");
field_bool!(SysCfg, 6, spi_crcen, "Enable SPI CRC functionality");
field_bool!(SysCfg, 7, cia_ipatov, "Select CIA processing of the preamble CIR");
field_bool!(SysCfg, 8, cia_sts, "Select CIA processing of the STS CIR");
field_bool!(SysCfg, 9, rxwtoe, "Receive Wait Timeout Enable");
field_bool!(SysCfg, 10, rxautr, "Receiver Auto-Re-enable");
field_bool!(SysCfg, 11, auto_ack, "Automatic Acknowledgement Enable");
field_enum!(SysCfg, 12, 14, cp_spc, u8, StsPocketPosition, "STS packet configuration");
// bit 14 - reserved
field_bool!(SysCfg, 15, cp_sdc, "Super Deterministic Code (SDC) mode enabled");
field_enum!(SysCfg, 16, 18, pdoa_mode, u8, PdoaMode, "PDoA mode");
field_bool!(SysCfg, 18, fast_aat, "Enable fast RX to TX turn around mode");
// bits 18-31 - reserved

field_prim!(FfCfg, 0, 8, frame_type, u8, "Frame type filter bitmap");
field_bool!(FfCfg, 8, ffbc, "Behave as PAN coordinator");
field_bool!(FfCfg, 9, ffib, "Allow MAC implicit broadcast");
field_prim!(FfCfg, 10, 14, le_pend, u8, "Data pending for device at LE address");
field_bool!(FfCfg, 14, ssadrape, "Short source address data request ACK with PEND enable");
field_bool!(FfCfg, 15, lsadrape, "Long source address data request ACK with PEND enable");

field_prim!(TxFctrl, 0, 10, txflen, u16, "Transmit frame length");
field_enum!(TxFctrl, 10, 11, txbr, u8, BitRate, "Transmit bit rate");
field_bool!(TxFctrl, 11, tr, "Transmit ranging enable");
field_enum!(TxFctrl, 12, 16, txpsr, u8, TxPreambleLength, "Transmit preamble symbol repetition (PSR)");
field_prim!(TxFctrl, 16, 26, txb_offset, u16, "Transmit buffer index offset");
// bits 26-39 - reserved
field_prim!(TxFctrl, 40, 48, fine_plen, u16, "Fine PSR control");

field_prim!(TxFctrlShort, 0, 10, txflen, u16, "Transmit frame length");
field_enum!(TxFctrlShort, 10, 11, txbr, u8, BitRate, "Transmit bit rate");
field_bool!(TxFctrlShort, 11, tr, "Transmit ranging enable");
field_enum!(TxFctrlShort, 12, 16, txpsr, u8, TxPreambleLength, "Transmit preamble symbol repetition (PSR)");

field_prim!(TxPower, 0, 8, data_pwr, u8, "Data transmit power");
field_prim!(TxPower, 8, 16, phr_pwr, u8, "PHY header transmit power");
field_prim!(TxPower, 16, 24, shr_pwr, u8, "SHR transmit power");
field_prim!(TxPower, 24, 32, sts_pwr, u8, "STS transmit power");

field_prim!(RxFwto, 0, 20, rx_fwto, u32, "Receive frame wait timeout, 1.0256us per unit");

field_prim!(RxFinfo, 0, 10, rxflen, u16, "Receive frame length");
// bit 10 - reserved
field_prim!(RxFinfo, 11, 13, rxnspl, u8, "Receive non-standard preamble length, estimated");
field_enum!(RxFinfo, 13, 14, rxbr, u8, BitRate, "Receive bit rate");
// bit 14 - reserved
field_bool!(RxFinfo, 15, rng, "Ranging flag in PHR");
field_enum!(RxFinfo, 16, 18, rxpfr, u8, RxPfr, "Receive PFR");
field_enum!(RxFinfo, 18, 20, rxpsr, u8, RxPreambleLength, "Receive preamble length from PHR");
field_prim!(RxFinfo, 20, 32, rxpacc, u16, "Preamble accumulation count");

field_prim!(AckRespT, 0, 20, w4r_tim, u8, "Wait-for-response turn-around time, 128 System ticks");
// bits 20-23 - reserved
field_prim!(AckRespT, 24, 32, ack_tim, u8, "Auto-acknowledgement turn-around time, preamble symbols");

field_enum!(ChanCtrl, 0, 1, rf_chan, u8, Channel, "Channel for Rx and Tx");
field_enum!(ChanCtrl, 1, 3, sfd_type, u8, SfdType, "SFD type");
field_prim!(ChanCtrl, 3, 7, tx_pcode, u8, "Transmit preamble code");
field_prim!(ChanCtrl, 8, 13, rx_pcode, u8, "Receive preamble code");
// bits 13-15 - reserved

field_prim!(RdbStatus, 0, 4, status_0, u8, "Receiver status flags, set 0");
field_prim!(RdbStatus, 4, 8, status_1, u8, "Receiver status flags, set 1");

field_bool!(DgcCfg, 0, rx_tune_en, "RX tuning enable bit");
// bits 1-8 - reserved
field_prim!(DgcCfg, 9, 14, thr_64, u8, "RX tuning threshold configuration for 64 MHz PRF");

field_enum!(RxCal, 0, 2, cal_mode, u8, CalibrationMode, "RX calibration mode");
field_bool!(RxCal, 4, cal_en, "RX calibration enable");
field_prim!(RxCal, 16, 20, comp_dly, u8, "RX calibration tuning value");

field_prim!(RxCalResi, 0, 29, resi, u32, "Calibration result I");

field_prim!(RxCalResq, 0, 29, resq, u32, "Calibration result Q");

field_enum!(Dtune0, 0, 2, pac, u8, Pac, "Preamble acquisition chunk size");
// bits 2-3 - reserved
field_bool!(Dtune0, 4, dt0b4, "Undocumented bit");

field_prim!(LdoTune, 0, 60, ldo_tune, u64, "Internal LDO voltage tuning parameter");

field_bool!(LdoCtrl, 0, vddms1_en, "VDDMS1 enable");
field_bool!(LdoCtrl, 1, vddms2_en, "VDDMS2 enable");
field_bool!(LdoCtrl, 2, vddms3_en, "VDDMS3 enable");
field_bool!(LdoCtrl, 4, vddpll_en, "VDDPLL enable");
field_bool!(LdoCtrl, 5, vddtx1_en, "VDDTX1 enable");
field_bool!(LdoCtrl, 6, vddtx2_en, "VDDTX2 enable");
field_bool!(LdoCtrl, 8, vddif2_en, "VDDIF2 enable");
field_bool!(LdoCtrl, 11, vddhvtx_en, "VDDHVTX enable");
field_bool!(LdoCtrl, 21, vddtx1_vref, "VDDTX1 VREF");
field_bool!(LdoCtrl, 22, vddtx2_vref, "VDDTX2 VREF");
field_bool!(LdoCtrl, 27, vddhvtx_vref, "VDDHVTX VREF");

field_bool!(SarCtrl, 0, sar_start, "Start SAR conversion");

field_prim!(PllCc, 0, 22, code, u32, "PLL calibration coarse code for both channels");

field_bool!(PllCal, 1, use_old, "use the coarse code value as set in PLL_CC");
field_prim!(PllCal, 4, 8, pll_cfg_ld, u8, "PLL calibration configuration value");
field_bool!(PllCal, 8, cal_en, "Force recalibration, self clear");

field_prim!(Xtal, 0, 6, xtal_trim, u8, "Crystal trim");

field_prim!(OtpAddr, 0, 11, otp_addr, u16, "Otp access address");
// bits 11-15 - reserved

field_bool!(OtpCfg, 0, otp_man, "Enable manual control over OTP interface");
field_bool!(OtpCfg, 1, otp_read, "OTP read enable");
field_bool!(OtpCfg, 2, otp_write, "OTP write enable");
field_bool!(OtpCfg, 3, otp_write_mr, "OTP write mode");
// bits 4-5 - reserved
field_bool!(OtpCfg, 6, dgc_kick, "Load and set RX_TUNE_CAL");
field_bool!(OtpCfg, 7, ldo_kick, "Load and set LDOTUNE_CAL");
field_bool!(OtpCfg, 8, bias_kick, "Load and set BIASTUNE_CAL");
// bits 9 - reserved
field_bool!(OtpCfg, 10, ops_kick, "Load and set operating parameter");
field_enum!(OtpCfg, 11, 13, ops_sel, u8, ReceiverParameterSet, "Operating parameter set selection");
field_enum!(OtpCfg, 13, 14, dgc_sel, u8, Channel, "RX_TUNE parameter set selection");
// bits 14-15 - reserved

field_bool!(OtpStat, 0, otp_prog_done, "OTP programming done");
field_bool!(OtpStat, 1, otp_vpp_ok, "OTP programming voltage OK");
// bits 2-7 - reserved

field_prim!(SysState, 0, 4, tx_state, u8, "Transmit SM state");
// bits 4-7 - reserved
field_prim!(SysState, 8, 14, rx_state, u8, "Receive SM state");
// bits 4-15 - reserved
field_prim!(SysState, 16, 21, pmsc_state, u8, "PMSC SM state");
// bits 21-31 - reserved

field_bool!(SoftRst, 0, arm_rst, "ARM block reset, active low");
field_bool!(SoftRst, 1, prgn_rst, "PRGN block reset, active low");
field_bool!(SoftRst, 2, cia_rst, "CIA block reset, active low");
field_bool!(SoftRst, 3, bist_rst, "BIST block reset, active low");
field_bool!(SoftRst, 4, rx_rst, "RX block reset, active low");
field_bool!(SoftRst, 5, tx_rst, "TX block reset, active low");
field_bool!(SoftRst, 6, hif_rst, "HIF block reset, active low");
field_bool!(SoftRst, 7, pmsc_rst, "PMSC block reset, active low");
field_bool!(SoftRst, 8, gpio_rst, "GPIO block reset, active low");

field_enum!(ClkCtrl, 0, 2, sys_clk, u8, ClockSource, "System clock selection");
field_prim!(ClkCtrl, 2, 4, rx_clk, u8, "Receiver clock selection");
field_prim!(ClkCtrl, 4, 6, tx_clk, u8, "Transmitter clock selection");
field_bool!(ClkCtrl, 6, acc_clk_en, "Force Accumulator clock enable");
// bit 7 - reserved
field_bool!(ClkCtrl, 8, cia_clk_en, "Force CIA clock enable");
// bit 9 - reserved
field_bool!(ClkCtrl, 10, sar_clk_en, "Temperature and voltage ADC clock enable");
// bits 11-14 - reserved
field_bool!(ClkCtrl, 15, acc_mclk_en, "Accumulator memory clock enable");
field_bool!(ClkCtrl, 16, gpio_clk_en, "GPIO clock enable");
// bit 17 - reserved
field_bool!(ClkCtrl, 18, gpio_dclk_en, "GPIO de-bounce clock enable");
field_bool!(ClkCtrl, 19, gpio_drst_en, "GPIO de-bounce reset, active low");
// bits 20-22 - reserved
field_bool!(ClkCtrl, 23, lp_clk_en, "Kilohertz clock enable");
// bits 24-31 - reserved

// bits 0-7 - reserved
field_bool!(SeqCtrl, 8, ainit2idle, "Automatic IDLE_RC to IDLE_PLL");
// bits 9-10 - reserved
field_bool!(SeqCtrl, 11, atx2slp, "After TX automatically Sleep");
field_bool!(SeqCtrl, 12, arx2slp, "After RX automatically Sleep");
// bits 13-14 - reserved
field_bool!(SeqCtrl, 15, pll_sync, "Enable a 1GHz clock used for some external SYNC modes");
// bit 16 - reserved
field_bool!(SeqCtrl, 17, ciarune, "CIA run enable");
// bits 18-22 - reserved
field_bool!(SeqCtrl, 23, force2init, "Force to IDLE_RC state");
// bits 24-25 - reserved
field_prim!(SeqCtrl, 26, 32, lp_clk_div, u8, "Kilohertz clock divisor");

field_prim!(BiasCtrl, 0, 14, bias_ctrl, u16, "Analog blocks calibration value");
field_prim!(BiasCtrl, 0, 5, manual_bits, u8, "Bits that are not automatically copied from OTP");

field_prim!(DbRxFinfo, 0, 10, rxflen, u16, "Receive frame length");
// bit 10 - reserved
field_prim!(DbRxFinfo, 11, 13, rxnspl, u8, "Receive non-standard preamble length, estimated");
field_enum!(DbRxFinfo, 13, 14, rxbr, u8, BitRate, "Receive bit rate");
// bit 14 - reserved
field_bool!(DbRxFinfo, 15, rng, "Ranging flag in PHR");
field_enum!(DbRxFinfo, 16, 18, rxpfr, u8, RxPfr, "Receive PFR");
field_enum!(DbRxFinfo, 18, 19, rxpsr, u8, RxPreambleLength, "Receive preamble length from PHR");
field_prim!(DbRxFinfo, 20, 32, rxpacc, u16, "Preamble accumulation count");

field_prim!(PtrAddrA, 0, 5, ptra_base, u8, "Indirect pointer A register ID");
field_prim!(PtrOffsetA, 0, 15, ptra_ofs, u16, "Indirect pointer A register ID");
field_prim!(PtrAddrB, 0, 5, ptrb_base, u8, "Indirect pointer B register ID");
field_prim!(PtrOffsetB, 0, 15, ptrb_ofs, u16, "Indirect pointer B register ID");
