use crate::bus_interface::{BusController, BusException, BusReader, BusWriter};

#[derive(Debug, Default)]
pub struct Cpu<B> {
    /// CPU bus
    bus: B,
    /// Registers
    x: [u32; 32],
    /// Program counter
    pc: u32,
    /// The mstatus register is an MXLEN-bit read/write register formatted as shown in Figure 1.6
    /// for RV64 and Figure 1.7 for RV32. The mstatus register keeps track of and controls
    /// the hart’s current operating state.
    mstatus: u32,
    cycle: u64,
    // @See. https://www.five-embeddev.com/riscv-isa-manual/latest/machine.html#machine-level-csrs
    /// The mscratch register is an MXLEN-bit read/write register dedicated for use by machine mode.
    /// Typically, it is used to hold a pointer to a machine-mode hart-local context space and swapped
    /// with a user register upon entry to an M-mode trap handler.
    mscratch: u32,
    /// The mtvec register is an MXLEN-bit WARL read/write register that holds trap vector configuration,
    /// consisting of a vector base address (BASE) and a vector mode (MODE).
    mtvec: u32,
    /// The mie is the corresponding MXLEN-bit read/write register containing interrupt enable bits.
    mie: u32,
    /// The mip register is an MXLEN-bit read/write register containing information on pending interrupts.
    mip: u32,
    /// mepc is an MXLEN-bit read/write register formatted as shown in Figure 1.21. The low bit of mepc
    /// (mepc[0]) is always zero. On implementations that support only IALIGN=32, the two low bits (mepc[1:0])
    /// are always zero.
    mepc: u32,
    /// Machine Trap Value Register (mtval)
    /// The mtval register is an MXLEN-bit read-write register formatted as shown in Figure 1.23.
    /// When a trap is taken into M-mode, mtval is either set to zero or written with
    /// exception-specific information to assist software in handling the trap. Otherwise,
    /// mtval is never written by the implementation, though it may be explicitly written by software.
    /// The hardware platform will specify which exceptions must set mtval informatively and which may
    /// unconditionally set it to zero.
    mtval: u32,
    /// Machine Cause Register (mcause)
    /// The mcause register is an MXLEN-bit read-write register formatted as shown in Figure 3.22. When
    /// a trap is taken into M-mode, mcause is written with a code indicating the event that caused the
    /// trap. Otherwise, mcause is never written by the implementation, though it may be explicitly
    /// written by software.
    mcause: u32,
    /// Exception code recoder.
    exception: u32,
    /// The Wait for Interrupt instruction (WFI) provides a hint to the implementation that
    /// the current hart can be stalled until an interrupt might need servicing.
    wait_for_interrupt: bool,
    /// Previous privilege mode.
    previous_mode: PrivilegeMode,
    /// This is used to reserve addresses for LR/SC
    reserved_load_addresses: std::collections::HashMap<u32, u32>,
    /// It is used to record exception reason for mtval
    cause: u32,
}

impl<B: BusController + BusReader + BusWriter> Cpu<B> {
    pub fn new(bus: B) -> Self {
        Self {
            x: [0; 32],
            pc: 0,
            mstatus: 0,
            cycle: 0,
            mscratch: 0,
            mtvec: 0,
            mie: 0,
            mip: 0,
            mepc: 0,
            mtval: 0,
            mcause: 0,
            bus,
            exception: 0,
            wait_for_interrupt: false,
            previous_mode: PrivilegeMode::Machine,
            reserved_load_addresses: std::collections::HashMap::new(),
            cause: 0,
        }
    }

    pub fn a0(&mut self, a0: u32) -> &mut Self {
        self.x[10] = a0;
        self
    }

    pub fn a1(&mut self, a1: u32) -> &mut Self {
        self.x[11] = a1;
        self
    }

    pub fn pc(&mut self, pc: u32) -> &mut Self {
        self.pc = pc;
        self
    }
}

// @See https://github.com/riscv/riscv-isa-manual/releases/download/Priv-v1.12/riscv-privileged-20211203.pdf p39
#[derive(Debug, PartialEq)]
#[allow(dead_code)]
enum Exception {
    // 0x0: Instruction address misaligned
    InstructionAddressMisaligned = 0x0,
    // 0x1: Instruction access fault
    InstructionAccessFault = 0x1,
    // 0x2: Illegal instruction
    IllegalInstruction = 0x2,
    // 0x3: Breakpoint
    Breakpoint = 0x3,
    // 0x4: Load address misaligned
    LoadAddressMisaligned = 0x4,
    // 0x5: Load access fault
    LoadAccessFault = 0x5,
    // 0x6: Store/AMO address misaligned
    StoreAmoAddressMisaligned = 0x6,
    // 0x7: Store/AMO access fault
    StoreAmoAccessFault = 0x7,
    // 0x8: Environment call from U-mode
    EnvironmentCallUmode = 0x8,
    // 0x9: Environment call from S-mode
    EnvironmentCallSmode = 0x9,
    // 0xB: Environment call from M-mode
    EnvironmentCallMmode = 0xB,
    // 0xC: Instruction page fault
    InstructionPageFault = 0xC,
    // 0xD: Load page fault
    LoadPageFault = 0xD,
    // 0xF: Store/AMO page fault
    StoreAmoPageFault = 0xF,
}

impl From<BusException> for Exception {
    fn from(value: BusException) -> Self {
        match value {
            BusException::LoadAccessFault => Exception::LoadAccessFault,
            BusException::LoadAddressMisaligned => Exception::LoadAddressMisaligned,
            BusException::StoreAccessFault => Exception::StoreAmoAccessFault,
            BusException::StoreAddressMisaligned => Exception::StoreAmoAddressMisaligned,
        }
    }
}

impl From<Exception> for u32 {
    fn from(value: Exception) -> Self {
        value as u32
    }
}

/// @See https://www.five-embeddev.com/riscv-isa-manual/latest/machine.html#sec:mcause
#[derive(Debug, PartialEq, Clone, Copy)]
#[allow(dead_code)]
enum Interrupt {
    MachineTimerInterrupt,
}

impl From<Interrupt> for u32 {
    fn from(value: Interrupt) -> Self {
        match value {
            Interrupt::MachineTimerInterrupt => 0x8000_0007,
        }
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum PrivilegeMode {
    User,
    SuperVisor,
    Reserved,
    Machine,
}

impl Default for PrivilegeMode {
    fn default() -> Self {
        Self::Machine
    }
}

impl From<PrivilegeMode> for u32 {
    fn from(value: PrivilegeMode) -> Self {
        match value {
            PrivilegeMode::User => 0,
            PrivilegeMode::SuperVisor => 1,
            PrivilegeMode::Reserved => 2,
            PrivilegeMode::Machine => 3,
        }
    }
}

impl From<u32> for PrivilegeMode {
    fn from(value: u32) -> Self {
        match value {
            0b00 => PrivilegeMode::User,
            0b01 => PrivilegeMode::SuperVisor,
            0b11 => PrivilegeMode::Machine,
            _ => PrivilegeMode::Reserved,
        }
    }
}

mod helpers {
    pub(crate) fn rd(ir: u32) -> usize {
        ((ir >> 7) & 0x1f) as usize
    }

    pub(crate) fn rs1(ir: u32) -> usize {
        ((ir >> 15) & 0x1f) as usize
    }
}

#[derive(Debug, Copy, Clone)]
pub enum CpuState {
    Idle,
    Active,
}

impl<B: BusController + BusWriter + BusReader> Cpu<B> {
    pub fn add_cycles(&mut self, count: u32) {
        self.cycle = self.cycle.wrapping_add(count as u64);
    }

    pub fn bus(&self) -> &B {
        &self.bus
    }

    pub fn step(&mut self) -> CpuState {
        // Drive bus state
        self.bus.step(&mut self.mip);

        // If interrupted
        // MTIP（bit 7）：Machine timer interrupt pending
        if self.mip & 0x80 != 0 {
            self.wait_for_interrupt = false;
        } else if self.wait_for_interrupt {
            return CpuState::Idle;
        }

        // bit3 in mstatus is MIE: Machine Interrupt Enable
        if (self.mip & 0x80 != 0) && (self.mie & 0x80 != 0) && (self.mstatus & 0x8 != 0) {
            self.exception = Interrupt::MachineTimerInterrupt.into();
            self.process_exception();
            return CpuState::Active;
        }

        self.cycle = self.cycle.wrapping_add(1);

        let Ok(ir) = self.bus.read32(self.pc) else {
                self.record_exception(Exception::InstructionAddressMisaligned, self.pc);
                self.process_exception();
                return CpuState::Active;
            };

        match ir & 0x7f {
            0b0110111 => self.write_back(helpers::rd(ir), ir & 0xfffff000), // LUI
            0b0010111 => {
                self.write_back(helpers::rd(ir), self.pc.wrapping_add(ir & 0xfffff000))
                // AUIPC
            }
            0b1101111 => self.jal(ir),    // JAL
            0b1100111 => self.jalr(ir),   // JALR
            0b1100011 => self.branch(ir), // Branch
            0b0000011 => self.load(ir),   // Load
            0b0100011 => self.store(ir),  // Store
            0b0110011 if (ir & 0x02000000) != 0 && (ir & 0b100000) != 0 => {
                self.multi_or_div(ir) // RV32M
            }
            0b0010011 | 0b0110011 => self.op(ir), // Op
            0b0001111 => {}                       // Fence.i, NOP in this emulator.
            0b1110011 => {
                // Zicsr
                self.zicsr(ir);
                if self.wait_for_interrupt {
                    return CpuState::Idle;
                }
            }
            0b0101111 => self.atomic(ir), // RV32A
            _ => {
                self.record_exception(Exception::IllegalInstruction, ir);
            }
        }

        if self.exception != 0 {
            self.process_exception();
            return CpuState::Active;
        }

        self.pc += 4;
        self.process_exception();
        CpuState::Active
    }

    fn record_exception(&mut self, e: Exception, cause: u32) {
        // When a hardware breakpoint is triggered, or an address-misaligned, access-fault, or page-fault exception
        // occurs on an instruction fetch, load, or store,  mtval is written with the faulting virtual address.
        // On an illegal instruction trap, mtval may be written with the first XLEN or ILEN bits of the faulting instruction
        // as described below. For other traps, mtval is set to zero, but a future standard may redefine
        // mtval’s setting for other traps.
        self.exception = e.into();
        self.cause = cause;
    }

    fn process_exception(&mut self) {
        if self.exception != 0 {
            // Interrupt
            if self.exception & 0x80000000 != 0 {
                self.mcause = self.exception;
                self.mtval = 0;
            } else {
                // Exception
                self.mcause = self.exception;
                self.mtval = self.cause;
            }
            self.mepc = self.pc;
            let prev: u32 = self.previous_mode.into();
            self.mstatus = (((self.mstatus) & 0x08) << 4) | (prev << 11);
            self.pc = self.mtvec;
            self.previous_mode = PrivilegeMode::Machine;
            self.exception = 0;
        }
    }

    fn write_back(&mut self, rd: usize, v: u32) {
        if rd != 0 {
            self.x[rd] = v
        }
    }

    fn jal(&mut self, ir: u32) {
        let rd = helpers::rd(ir);
        let rel = ((ir & 0x80000000) >> 11)
            | ((ir & 0x7fe00000) >> 20)
            | ((ir & 0x00100000) >> 9)
            | (ir & 0x000ff000);
        let rel = if rel & 0x00100000 != 0 { rel | 0xffe00000 } else { rel } as i32;
        let v = self.pc + 4;
        self.pc = (self.pc as i64 + rel as i64 - 4) as u32;
        self.write_back(rd, v);
    }

    fn jalr(&mut self, ir: u32) {
        let rd = helpers::rd(ir);
        let imm = ir >> 20;
        let imm_s = imm | if (imm & 0x800) != 0 { 0xfffff000 } else { 0 };
        let v = self.pc + 4;
        let rs1 = self.x[helpers::rs1(ir)];
        self.pc = (((rs1 as i64 + imm_s as i32 as i64) & !1) - 4) as u32;
        self.write_back(rd, v)
    }

    fn branch(&mut self, ir: u32) {
        // Branch
        let immm4 = ((ir & 0xf00) >> 7)
            | ((ir & 0x7e000000) >> 20)
            | ((ir & 0x80) << 4)
            | ((ir >> 31) << 12);
        let immm4 = if immm4 & 0x1000 != 0 { immm4 | 0xffffe000 } else { immm4 };
        let immm4 = (self.pc as i64 + immm4 as i32 as i64 - 4) as u32;
        let rs1 = self.x[helpers::rs1(ir)] as i32;
        let rs2 = self.x[((ir >> 20) & 0x1f) as usize] as i32;
        match (ir >> 12) & 0x7 {
            // BEQ, BNE, BLT, BGE, BLTU, BGEU
            0b000 => (rs1 == rs2).then(|| self.pc = immm4),
            0b001 => (rs1 != rs2).then(|| self.pc = immm4),
            0b100 => (rs1 < rs2).then(|| self.pc = immm4),
            0b101 => (rs1 >= rs2).then(|| self.pc = immm4),
            0b110 => ((rs1 as u32) < (rs2 as u32)).then(|| self.pc = immm4),
            0b111 => ((rs1 as u32) >= (rs2 as u32)).then(|| self.pc = immm4),
            _ => {
                self.record_exception(Exception::IllegalInstruction, ir);
                None
            }
        };
    }

    fn load(&mut self, ir: u32) {
        let rd = helpers::rd(ir);
        let rs1 = self.x[helpers::rs1(ir)];
        let imm = ir >> 20;
        let imm_s = (imm | (if (imm & 0x800) != 0 { 0xfffff000 } else { 0 })) as i32;
        let rsval = (rs1 as i64 + imm_s as i64) as u32;

        match (ir >> 12) & 0x7 {
            // LB, LH, LW, LBU, LHU
            0b000 => match self.bus.read8(rsval) {
                Ok(v) => self.x[rd] = (v as i8) as u32,
                Err(e) => self.record_exception(e.into(), rsval),
            },
            0b001 => match self.bus.read16(rsval) {
                Ok(v) => self.x[rd] = (v as i16) as u32,
                Err(e) => self.record_exception(e.into(), rsval),
            },
            0b010 => match self.bus.read32(rsval) {
                Ok(v) => self.x[rd] = v,
                Err(e) => self.record_exception(e.into(), rsval),
            },
            0b100 => match self.bus.read8(rsval) {
                Ok(v) => self.x[rd] = v as u32,
                Err(e) => self.record_exception(e.into(), rsval),
            },
            0b101 => match self.bus.read16(rsval) {
                Ok(v) => self.x[rd] = v as u32,
                Err(e) => self.record_exception(e.into(), rsval),
            },
            _ => {
                self.record_exception(Exception::IllegalInstruction, ir);
            }
        }
        // }
    }

    fn store(&mut self, ir: u32) {
        let rs1 = self.x[helpers::rs1(ir)];
        let rs2 = self.x[((ir >> 20) & 0x1f) as usize];
        let mut addr = ((ir >> 7) & 0x1f) | ((ir & 0xfe000000) >> 20);
        if addr & 0x800 != 0 {
            addr |= 0xfffff000;
        }
        let addr = addr.wrapping_add(rs1);

        match (ir >> 12) & 0x7 {
            // SB, SH, SW
            0b000 => {
                self.bus
                    .write8(addr, rs2 as u8)
                    .unwrap_or_else(|e| self.record_exception(e.into(), addr));
            }
            0b001 => {
                self.bus
                    .write16(addr, rs2 as u16)
                    .unwrap_or_else(|e| self.record_exception(e.into(), addr));
            }
            0b010 => {
                self.bus
                    .write32(addr, rs2)
                    .unwrap_or_else(|e| self.record_exception(e.into(), addr));
            }
            _ => {
                self.record_exception(Exception::IllegalInstruction, ir);
            }
        };
    }

    // RV32A
    fn atomic(&mut self, ir: u32) {
        let rd = helpers::rd(ir);
        let rs1 = self.x[helpers::rs1(ir)];
        let mut rs2 = self.x[((ir >> 20) & 0x1f) as usize];
        let f = (ir >> 27) & 0x1f;

        let v = match self.bus.read32(rs1) {
            Ok(v) => v,
            Err(e) => {
                self.record_exception(e.into(), rs1);
                return;
            }
        };

        match f {
            // LR.W
            // Load-Reserved Word
            0b00010 => {
                self.reserved_load_addresses.insert(rs1, v);
                self.write_back(rd, v)
            }
            // SC.W
            // Store-Conditional Word
            0b00011 => {
                if let Some(val) = self.reserved_load_addresses.get(&rs1) {
                    if *val == v {
                        self.bus
                            .write32(rs1, rs2)
                            .unwrap_or_else(|e| self.record_exception(e.into(), rs1));
                        self.write_back(rd, 0);
                    } else {
                        self.write_back(rd, 1);
                    }
                } else {
                    self.write_back(rd, 1);
                }
            }
            // AMOSWAP.W
            0b00001 => {
                self.bus
                    .write32(rs1, rs2)
                    .unwrap_or_else(|e| self.record_exception(e.into(), rs1));
                self.write_back(rd, v);
            }
            0b00000 => {
                rs2 = rs2.wrapping_add(v);
                self.bus
                    .write32(rs1, rs2)
                    .unwrap_or_else(|e| self.record_exception(e.into(), rs1));
                self.write_back(rd, v)
            }
            // AMOXOR.W
            0b00100 => {
                rs2 ^= v;
                self.bus
                    .write32(rs1, rs2)
                    .unwrap_or_else(|e| self.record_exception(e.into(), rs1));
                self.write_back(rd, v)
            }
            // AMOAND.W
            0b01100 => {
                rs2 &= v;
                self.bus
                    .write32(rs1, rs2)
                    .unwrap_or_else(|e| self.record_exception(e.into(), rs1));
                self.write_back(rd, v)
            }
            // AMOOR.W
            0b01000 => {
                rs2 |= v;
                self.bus
                    .write32(rs1, rs2)
                    .unwrap_or_else(|e| self.record_exception(e.into(), rs1));
                self.write_back(rd, v)
            }
            // AMOMIN.W
            0b10000 => {
                rs2 = if (rs2 as i32) < (v as i32) { rs2 } else { v };
                self.bus
                    .write32(rs1, rs2)
                    .unwrap_or_else(|e| self.record_exception(e.into(), rs1));
                self.write_back(rd, v)
            }
            // AMOMAX.W
            0b10100 => {
                rs2 = if (rs2 as i32) > (v as i32) { rs2 } else { v };
                self.bus
                    .write32(rs1, rs2)
                    .unwrap_or_else(|e| self.record_exception(e.into(), rs1));
                self.write_back(rd, v)
            }
            // AMOMINU.W
            0b11000 => {
                rs2 = if rs2 < v { rs2 } else { v };
                self.bus
                    .write32(rs1, rs2)
                    .unwrap_or_else(|e| self.record_exception(e.into(), rs1));
                self.write_back(rd, v)
            }
            // AMOMAXU.W
            0b11100 => {
                rs2 = if rs2 > v { rs2 } else { v };
                self.bus
                    .write32(rs1, rs2)
                    .unwrap_or_else(|e| self.record_exception(e.into(), rs1));
                self.write_back(rd, v)
            }
            _ => {
                self.record_exception(Exception::IllegalInstruction, ir);
            }
        }
    }

    // RV32M
    fn multi_or_div(&mut self, ir: u32) {
        let rd = helpers::rd(ir);
        let imm = ir >> 20;
        let imm = imm | if (imm & 0x800) != 0 { 0xfffff000 } else { 0 };
        let rs1 = self.x[helpers::rs1(ir)];
        let is_reg = (ir & 0b100000) != 0;
        let rs2 = if is_reg { self.x[imm as usize & 0x1f] } else { imm };

        let mut v = 0;
        match (ir >> 12) & 7 {
            0b000 => v = rs1.wrapping_mul(rs2), // MUL
            0b001 => v = ((rs1 as i32 as i64).wrapping_mul(rs2 as i32 as i64) >> 32) as u32, // MULH
            0b010 => v = ((rs1 as i32 as i64).wrapping_mul(rs2 as i64) >> 32) as u32, // MULHSU
            0b011 => v = ((rs1 as u64).wrapping_mul(rs2 as u64) >> 32) as u32, // MULHU
            0b100 => v = if rs2 == 0 { !0 } else { (rs1).wrapping_div(rs2) }, // DIV
            0b101 => v = if rs2 == 0 { u32::MAX } else { rs1 / rs2 }, // DIVU
            0b110 if rs2 == 0 => v = 0,         // REM
            0b110 => v = (rs1 as i32).wrapping_rem(rs2 as i32) as u32, // REM
            0b111 => v = if rs2 == 0 { rs1 } else { rs1 % rs2 }, // REMU
            _ => {
                self.record_exception(Exception::IllegalInstruction, ir);
            }
        }
        self.write_back(rd, v);
    }

    // Op
    fn op(&mut self, ir: u32) {
        let rd = helpers::rd(ir);
        let imm = ir >> 20;
        let imm = imm | if (imm & 0x800) != 0 { 0xfffff000 } else { 0 };
        let rs1 = self.x[helpers::rs1(ir)];
        let reg = (ir & 0b100000) != 0;
        let rs2 = if reg { self.x[imm as usize & 0x1f] } else { imm };

        let mut v = 0;
        match (ir >> 12) & 7 {
            0b000 if reg && (ir & 0x4000_0000) != 0 => v = rs1.wrapping_sub(rs2),
            0b000 => v = rs1.wrapping_add(rs2),
            0b001 => v = rs1 << (rs2 & 0x1f),
            0b010 => v = if (rs1 as i32) < (rs2 as i32) { 1 } else { 0 },
            0b011 => v = if rs1 < rs2 { 1 } else { 0 },
            0b100 => v = rs1 ^ rs2,
            0b101 if (ir & 0x40000000) != 0 => v = ((rs1 as i32) >> (rs2 & 0x1f)) as u32,
            0b101 => v = rs1 >> (rs2 & 0x1f),
            0b110 => v = rs1 | rs2,
            0b111 => v = rs1 & rs2,
            _ => {
                self.record_exception(Exception::IllegalInstruction, ir);
            }
        }
        self.write_back(rd, v)
    }

    fn zicsr(&mut self, ir: u32) {
        // Zicsr
        let rd = helpers::rd(ir);
        let mut v = 0;
        let csr = ir >> 20;
        let op = (ir >> 12) & 0b111;
        if (op & 3) != 0 {
            let rs1imm = (ir >> 15) & 0x1f;
            let rs1 = self.x[rs1imm as usize];
            let mut val = rs1;
            // https://raw.githubusercontent.com/riscv/virtual-memory/main/specs/663-Svpbmt.pdf
            // Generally, support for Zicsr
            match csr {
                0x340 => v = self.mscratch,
                0x305 => v = self.mtvec,
                0x304 => v = self.mie,
                0xC00 => v = self.cycle as u32,
                0x341 => v = self.mepc,
                0x300 => v = self.mstatus,
                0x342 => v = self.mcause,
                0x343 => v = self.mtval,
                0xf11 => v = 0x00000000, // mvendorid
                0x301 => v = 0x00000000, // misa
                _ => {}
            }

            match op {
                0b001 => val = rs1,         //CSRRW
                0b010 => val = v | rs1,     //CSRRS
                0b011 => val = v & !rs1,    //CSRRC
                0b101 => val = rs1imm,      //CSRRWI
                0b110 => val = v | rs1imm,  //CSRRSI
                0b111 => val = v & !rs1imm, //CSRRCI
                _ => {}
            }

            match csr {
                0x340 => self.mscratch = val,
                0x305 => self.mtvec = val,
                0x304 => self.mie = val,
                0x344 => self.mip = val,
                0x341 => self.mepc = val,
                0x300 => self.mstatus = val,
                0x342 => self.mcause = val,
                0x343 => self.mtval = val,
                _ => {}
            }
        } else if op == 0b000 {
            if csr == 0x105 {
                //WFI
                self.mstatus |= 8;
                self.wait_for_interrupt = true; //Inform environment we want to go to sleep.
                self.pc += 4;
            } else if (csr & 0xff) == 0x02 {
                // MRET
                let prev_mstatus = self.mstatus;
                let prev_mode: u32 = self.previous_mode.into();
                self.mstatus = ((prev_mstatus & 0x80) >> 4) | (prev_mode << 11) | 0x80;
                self.previous_mode = PrivilegeMode::from(prev_mstatus >> 11);
                self.pc = self.mepc - 4;
            } else {
                match csr {
                    0 if self.previous_mode != PrivilegeMode::User => {
                        self.exception = Exception::EnvironmentCallMmode.into()
                    }
                    0 => self.exception = Exception::EnvironmentCallUmode.into(),
                    1 => self.exception = Exception::Breakpoint.into(),
                    _ => self.record_exception(Exception::IllegalInstruction, ir),
                }
            }
        } else {
            self.record_exception(Exception::IllegalInstruction, ir);
        }
        self.write_back(rd, v);
    }
}
