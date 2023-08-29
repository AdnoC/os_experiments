use spin::Mutex;
use elain::Align;
use tock_registers::{
    register_bitfields,
    registers::InMemoryRegister,
    interfaces::{ReadWriteable, Writeable},
};
use static_assertions::{assert_eq_size, assert_eq_align};

// https://stackoverflow.com/a/53646925
const fn max(a: usize, b: usize) -> usize {
    [a, b][(a < b) as usize]
}
const fn min(a: usize, b: usize) -> usize {
    [a, b][(a > b) as usize]
}

pub mod mmap {
    use crate::units::GIB;

    pub const END_RAM_ADDR: usize = (4 * GIB - 1) as usize;
}


mod page_size_64_kb {
    use crate::units::{TIB, MIB, KIB};
    use core::ops::RangeInclusive;

    pub const SIZE: usize = 64 * KIB as usize;
    pub const TABLE_ADRESS_PADDING_BITS: usize = 4;
    // With 64KB ganules we don't use level0
    pub const LEVEL0_TABLE_MAX_SIZE: usize = 1;
    pub const LEVEL0_TABLE_COVERAGE: usize = 4 * TIB as usize;
    pub const LEVEL0_BIT_RANGE: RangeInclusive<u64> = 0..=0;

    pub const LEVEL1_TABLE_MAX_SIZE: usize = 64;
    pub const LEVEL1_TABLE_COVERAGE: usize = 4 * TIB as usize;
    pub const LEVEL1_BIT_RANGE: RangeInclusive<u64> = 42..=47;

    pub const LEVEL2_TABLE_MAX_SIZE: usize = 8192;
    pub const LEVEL2_TABLE_COVERAGE: usize = 512 * MIB as usize;
    pub const LEVEL2_BIT_RANGE: RangeInclusive<u64> = 29..=41;

    pub const LEVEL3_TABLE_MAX_SIZE: usize = 8192;
    pub const LEVEL3_TABLE_COVERAGE: usize = 64 * KIB as usize;
    pub const LEVEL3_BIT_RANGE: RangeInclusive<u64> = 16..=28;
}

mod page_size_4_kb {
    use crate::units::{GIB, MIB, KIB};
    use core::ops::RangeInclusive;

    pub const SIZE: usize = 4 * KIB as usize;
    pub const TABLE_ADRESS_PADDING_BITS: usize = 0;

    pub const LEVEL0_TABLE_MAX_SIZE: usize = 512;
    pub const LEVEL0_TABLE_COVERAGE: usize = 512 * GIB as usize;
    pub const LEVEL0_BIT_RANGE: RangeInclusive<u64> = 39..=47;

    pub const LEVEL1_TABLE_MAX_SIZE: usize = 512;
    pub const LEVEL1_TABLE_COVERAGE: usize = 1 * GIB as usize;
    pub const LEVEL1_BIT_RANGE: RangeInclusive<u64> = 30..=38;

    pub const LEVEL2_TABLE_MAX_SIZE: usize = 512;
    pub const LEVEL2_TABLE_COVERAGE: usize = 2 * MIB as usize;
    pub const LEVEL2_BIT_RANGE: RangeInclusive<u64> = 21..=29;

    pub const LEVEL3_TABLE_MAX_SIZE: usize = 512;
    pub const LEVEL3_TABLE_COVERAGE: usize = 4 * KIB as usize;
    pub const LEVEL3_BIT_RANGE: RangeInclusive<u64> = 12..=20;
}
mod page_size {
    use super::{mmap, max, min};
    pub use super::page_size_4_kb::*;

    pub const LEVEL0_TABLE_SIZE: usize = min(max(mmap::END_RAM_ADDR / LEVEL0_TABLE_COVERAGE, 1), LEVEL0_TABLE_MAX_SIZE);
    pub const LEVEL1_TABLE_SIZE: usize = min(max(mmap::END_RAM_ADDR / LEVEL1_TABLE_COVERAGE, 1), LEVEL1_TABLE_MAX_SIZE);
    pub const LEVEL2_TABLE_SIZE: usize = min(max(mmap::END_RAM_ADDR / LEVEL2_TABLE_COVERAGE, 1), LEVEL2_TABLE_MAX_SIZE);
    pub const LEVEL3_TABLE_SIZE: usize = min(max(mmap::END_RAM_ADDR / LEVEL3_TABLE_COVERAGE, 1), LEVEL3_TABLE_MAX_SIZE);
}


static TRANLSATION_TABLES: Mutex<TranslationTable> = Mutex::new(TranslationTable::new());


const NUM_LEVEL_0: usize = page_size::LEVEL0_TABLE_SIZE;
const NUM_LEVEL_1: usize = page_size::LEVEL1_TABLE_SIZE;
const NUM_LEVEL_2: usize = page_size::LEVEL2_TABLE_SIZE;
const NUM_LEVEL_3: usize = page_size::LEVEL3_TABLE_SIZE;


struct TranslationTable {
    level0: DescriptorTable<NUM_LEVEL_0, {page_size::SIZE}>,
    level1: [DescriptorTable<NUM_LEVEL_1, {page_size::SIZE}>; NUM_LEVEL_0],
    level2: [[DescriptorTable<NUM_LEVEL_2, {page_size::SIZE}>; NUM_LEVEL_1]; NUM_LEVEL_0],
    level3: [[[EntryTable<NUM_LEVEL_3, {page_size::SIZE}>; NUM_LEVEL_2]; NUM_LEVEL_1]; NUM_LEVEL_0],
}

#[repr(C)]
#[derive(Clone, Copy)]
struct DummyTable<const N: usize, const A: usize>([u64; N], Align<A>) where Align<A>: elain::Alignment;

#[repr(C)]
struct DescriptorTable<const N: usize, const A: usize>([TableDescriptionR; N], Align<A>) where Align<A>: elain::Alignment;
#[repr(C)]
struct EntryTable<const N: usize, const A: usize>([PageEntryR; N], Align<A>) where Align<A>: elain::Alignment;

impl TranslationTable {
    const fn new() -> Self {
        use core::mem::transmute;

        assert_eq_size!(TableDescriptionR, u64);
        assert_eq_size!(PageEntryR, u64);
        assert_eq_align!(TableDescriptionR, u64);
        assert_eq_align!(PageEntryR, u64);
        // Its dumb that I need to do this, but there is no better way to initialize this
        // Safety: InMemoryRegister are #[repr(transparent)] with the uint type
        unsafe {
            TranslationTable {
                level0: transmute(DummyTable([0u64; NUM_LEVEL_0], Align::<{page_size::SIZE}>::NEW)),
                level1: transmute([DummyTable([0u64; NUM_LEVEL_1], Align::<{page_size::SIZE}>::NEW); NUM_LEVEL_0]),
                level2: transmute([[DummyTable([0u64; NUM_LEVEL_2], Align::<{page_size::SIZE}>::NEW); NUM_LEVEL_1]; NUM_LEVEL_0]),
                level3: transmute([[[DummyTable([0u64; NUM_LEVEL_3], Align::<{page_size::SIZE}>::NEW); NUM_LEVEL_2]; NUM_LEVEL_1]; NUM_LEVEL_0]),
            }
        }
    }


    fn translate_virt_to_phys(&self, virt_addr: u64) -> Option<u64> {
        use tock_registers::interfaces::Readable;
        use core::ops::RangeInclusive;
        fn get_bit_range(mut val: u64, addr_range: RangeInclusive<u64>) -> u64 {
            // let orig_val = val;
            let mask = u64::MAX << 1;
            for bidx in 0..(u64::BITS) {
                if !addr_range.contains(&(bidx as _)) {
                    val &= mask.rotate_left(bidx);
                }
            }
            // println!("bits {:?} of {}({:b}) = {}({:b})", addr_range, orig_val, orig_val, val, val);
            val >> addr_range.start()
        }

        let l0_idx = get_bit_range(virt_addr, page_size::LEVEL0_BIT_RANGE) as usize;
        let l1_idx = get_bit_range(virt_addr, page_size::LEVEL1_BIT_RANGE) as usize;
        let l2_idx = get_bit_range(virt_addr, page_size::LEVEL2_BIT_RANGE) as usize;
        let l3_idx = get_bit_range(virt_addr, page_size::LEVEL3_BIT_RANGE) as usize;
println!("virt: {:#x?}, idxs: {:?} ({:x?})", virt_addr, (l0_idx, l1_idx, l2_idx, l3_idx), (l0_idx, l1_idx, l2_idx, l3_idx));

        if l0_idx >= NUM_LEVEL_0 || l1_idx >= NUM_LEVEL_1 || l2_idx >= NUM_LEVEL_2 || l3_idx >= NUM_LEVEL_3 {
            return None;
        }

        let l3_entry = &self.level3[l0_idx][l1_idx][l2_idx].0[l3_idx];

        Some(l3_entry.read(PageEntry::OUTPUT_ADDR) >> page_size::TABLE_ADRESS_PADDING_BITS)
    }

    fn verify_table_pointers(&self) {
        use tock_registers::interfaces::Readable;
        for (l0_idx, l0_entry) in self.level0.0.iter().enumerate() {
            let l1_addr = &self.level1[l0_idx].0 as *const TableDescriptionR as usize as u64;
            let stored_addr = l0_entry.read(TableDescriptor::OUTPUT_ADDR) >> page_size::TABLE_ADRESS_PADDING_BITS;
            assert_eq!(l1_addr, stored_addr);

            for (l1_idx, l1_entry) in self.level1[l0_idx].0.iter().enumerate() {
                let l2_addr = &self.level2[l0_idx][l1_idx].0 as *const TableDescriptionR as usize as u64;
                let stored_addr = l1_entry.read(TableDescriptor::OUTPUT_ADDR) >> page_size::TABLE_ADRESS_PADDING_BITS;
                assert_eq!(l2_addr, stored_addr);

                for (l2_idx, l2_entry) in self.level2[l0_idx][l1_idx].0.iter().enumerate() {
                    let l3_addr = &self.level3[l0_idx][l1_idx][l2_idx].0 as *const PageEntryR as usize as u64;
                    let stored_addr = l2_entry.read(TableDescriptor::OUTPUT_ADDR) >> page_size::TABLE_ADRESS_PADDING_BITS;
                    assert_eq!(l3_addr, stored_addr);

                }
            }
        }
    }

    fn verify_identity_mapping(&self) {
        let mem_range = 0..=mmap::END_RAM_ADDR;
        for virt_addr in mem_range.clone().step_by(page_size::LEVEL3_TABLE_COVERAGE) {
            if !mem_range.contains(&virt_addr) { break;}

            let virt_addr = virt_addr as u64;
            let phys_addr = self.translate_virt_to_phys(virt_addr);
            if phys_addr.is_none() {
                panic!("Address 0x{:x} could not be mapped", virt_addr);
            }
            let phys_addr = phys_addr.unwrap();
            if virt_addr != phys_addr {
                panic!("Virt(0x{:x}) != Phys(0x{:x})", virt_addr, phys_addr);
            }
        }
    }

    fn populate_table_entries(&mut self) {
        let l0_shift = page_size::LEVEL0_TABLE_COVERAGE.trailing_zeros();
        let l1_shift = page_size::LEVEL1_TABLE_COVERAGE.trailing_zeros();
        let l2_shift = page_size::LEVEL2_TABLE_COVERAGE.trailing_zeros();
        let l3_shift = page_size::LEVEL3_TABLE_COVERAGE.trailing_zeros();
        for (l0_idx, l0_entry) in self.level0.0.iter_mut().enumerate() {
            let l1_addr = &self.level1[l0_idx].0 as *const TableDescriptionR as usize as u64;
            // let l1_addr = l1_addr << page_size::TABLE_ADRESS_PADDING_BITS;
            l0_entry.set(l1_addr);
            l0_entry.modify(
                TableDescriptor::VALID::SET +
                TableDescriptor::TYPE::Table +
                TableDescriptor::TABLE_ACCESS_PERMISSION::UrwPrw
            );

            for (l1_idx, l1_entry) in self.level1[l0_idx].0.iter_mut().enumerate() {
                let l2_addr = &self.level2[l0_idx][l1_idx].0 as *const TableDescriptionR as usize as u64;
                // let l2_addr = l2_addr << page_size::TABLE_ADRESS_PADDING_BITS;
                l1_entry.set(l2_addr);
                l1_entry.modify(
                    TableDescriptor::VALID::SET +
                    TableDescriptor::TYPE::Table +
                    TableDescriptor::TABLE_ACCESS_PERMISSION::UrwPrw
                );

                for (l2_idx, l2_entry) in self.level2[l0_idx][l1_idx].0.iter_mut().enumerate() {
                    let l3_addr = &self.level3[l0_idx][l1_idx][l2_idx].0 as *const PageEntryR as usize as u64;
                    // let l3_addr = l3_addr << page_size::TABLE_ADRESS_PADDING_BITS;
                    l2_entry.set(l3_addr);
                    l2_entry.modify(
                        TableDescriptor::VALID::SET +
                        TableDescriptor::TYPE::Table +
                        TableDescriptor::TABLE_ACCESS_PERMISSION::UrwPrw
                    );

                    for (l3_idx, l3_entry) in self.level3[l0_idx][l1_idx][l2_idx].0.iter_mut().enumerate() {
                        let phys_addr = (l0_idx << l0_shift) | (l1_idx << l1_shift) | (l2_idx << l2_shift) | (l3_idx << l3_shift);
                        // let phys_addr = phys_addr << page_size::TABLE_ADRESS_PADDING_BITS;
                        l3_entry.set(phys_addr as u64);
                        l3_entry.modify(
                            PageEntry::VALID::SET +
                            PageEntry::TYPE::Page +
                            PageEntry::ACCESS_FLAG::SET
                        );
    // {
    //
    //
    //      use tock_registers::interfaces::Readable;
    // let raw_addr = page_size::LEVEL3_TABLE_COVERAGE * l3_idx;
    // let raw_addr_end = (page_size::LEVEL3_TABLE_COVERAGE * (l3_idx + 1)) - 1;
    // let raw_val = l3_entry.get();//.read(TableDescriptor::OUTPUT_ADDR);
    // let mut stored_val = raw_val;
    // let addr_range = 16..47;
    // let mask = u64::MAX << 1;
    // for bidx in 0..(u64::BITS) {
    //     if !addr_range.contains(&bidx) {
    //         stored_val &= mask.rotate_left(bidx);
    //     }
    // }
    //     println!("lvl3 addr = {:#x?} - {:#x?}, l2 pointer = {:#x?}, l2 ptr without 0 = {:#x?}, raw l2 = {:#x?}",
    //              raw_addr, raw_addr_end, stored_val, (stored_val >> 16), raw_val);
    //
    //     crate::uart::spin_until_enter();
    //                 }
                    }
                }
            }
        }

    // {
    //     fn get_bit_range(mut val: u64, start: usize, end: usize) -> u64 {
    //         let addr_range = start..=end;
    //         let mask = u64::MAX << 1;
    //         for bidx in 0..(u64::BITS) {
    //             if !addr_range.contains(&(bidx as _)) {
    //                 val &= mask.rotate_left(bidx);
    //             }
    //         }
    //         val >> start
    //     }
    //
    //     let virt_to_phys = |virt_addr: u64| -> u64 {
    //         use tock_registers::interfaces::Readable;
    //         let l1_idx = get_bit_range(virt_addr, 42, 47) as usize;
    //         let l2_idx = get_bit_range(virt_addr, 29, 41) as usize;
    //         let l3_idx = get_bit_range(virt_addr, 16, 28) as usize;
    //         // println!("virt: {:#x?}, idxs: {:?} ({:x?})", virt_addr, (l1_idx, l2_idx, l3_idx), (l1_idx, l2_idx, l3_idx));
    //
    //         if l1_idx >= NUM_LEVEL_1 || l2_idx >= NUM_LEVEL_2 || l3_idx >= NUM_LEVEL_3 {
    //             println!("virt: {:#x?}, idxs: {:?} ({:x?})", virt_addr, (l1_idx, l2_idx, l3_idx), (l1_idx, l2_idx, l3_idx));
    //
    //         }
    //         let l3_entry = &self.level3[0][l1_idx][l2_idx][l3_idx];
    //
    //         l3_entry.read(PageEntry::OUTPUT_ADDR) >> page_size::TABLE_ADRESS_PADDING_BITS
    //     };
    // let mem_range = (0..0xFFFF_FFFFu64);
    //     for virt_addr in mem_range.clone().step_by(page_size::LEVEL3_TABLE_COVERAGE).skip(8159 * 6) {
    //         if !mem_range.contains(&virt_addr) { break;}
    //
    //         let virt_addr = virt_addr as u64;
    //         let phys_addr = virt_to_phys(virt_addr);
    //         assert_eq!(virt_addr, phys_addr);
    //         // println!("virt -> phys: {:#x?} -> {:#x?}", virt_addr, phys_addr);
    //     }
    // }

        //
        // println!("shifts: {}, {}, {}, {}", l0_shift, l1_shift, l2_shift, l3_shift);
        // use tock_registers::interfaces::Readable;
        // for (l0_idx, l0_entry) in self.level0.iter_mut().enumerate() {
        //     println!("l0: {:?}", l0_entry.extract());
        //     for (l1_idx, l1_entry) in self.level1[l0_idx].iter_mut().enumerate() {
        //         println!("  l1: {:?}", l1_entry.extract());
        //         for (l2_idx, l2_entry) in self.level2[l0_idx][l1_idx].iter_mut().enumerate() {
        //             println!("    l2: {:?}", l2_entry.extract());
        //             for (l3_idx, l3_entry) in self.level3[l0_idx][l1_idx][l2_idx].iter_mut().enumerate() {
        //                 if l3_idx % 15 == 0 {
        //                     crate::uart::spin_until_enter();
        //                     println!();
        //                     print!("      l3:");
        //                 }
        //             print!("  {:?}", l3_entry.read(PageEntry::OUTPUT_ADDR));
        //             }
        //         }
        //     }
        // }
    }

}

pub fn init() -> Result<(), &'static str> {
    use aarch64_cpu::{
        registers::*,
        asm::barrier,
    };
        use tock_registers::interfaces::*;
        // Fail early if translation granule is not supported.
        if !ID_AA64MMFR0_EL1.matches_all(ID_AA64MMFR0_EL1::TGran64::Supported) {
            return Err( "Translation granule not supported in HW");
        }

    {
        let mut table = TRANLSATION_TABLES.lock();
        table.populate_table_entries();
        // table.translate_virt_to_phys(0xc0000000);
        // table.translate_virt_to_phys(crate::bus_to_phys(0x7E21_5040) as u6);
        // crate::uart::spin_until_enter();
        // println!("Populated tables.");
        // table.verify_table_pointers();
        // println!("Tables point to the correct tables.");
        // table.verify_identity_mapping();
        // println!("Tables map address space to itself.");
    }

      // Define the memory types being mapped.
        MAIR_EL1.write(
            // Attribute 0 - Cacheable normal DRAM.
            MAIR_EL1::Attr0_Normal_Outer::WriteBack_NonTransient_ReadWriteAlloc +
        MAIR_EL1::Attr0_Normal_Inner::WriteBack_NonTransient_ReadWriteAlloc +

        // Attribute 1 - Device.
        MAIR_EL1::Attr1_Device::nonGathering_nonReordering_EarlyWriteAck,
        );

    // let table_base_addr = &TRANLSATION_TABLES.lock().level0.0 as *const [TableDescriptionR; NUM_LEVEL_0] as usize as u64;
    let table_base_addr = &TRANLSATION_TABLES.lock().level1[0].0 as *const [TableDescriptionR; NUM_LEVEL_1] as usize as u64;
    // Set the address of the translation tables for lower half of virt address space
    TTBR0_EL1.set_baddr(table_base_addr);
    // Set the address of the translation tables for upper half of virt address space
    // TTBR1_EL1.set_baddr(table_base_addr);

    TCR_EL1.write(
        TCR_EL1::TBI0::Used +
        TCR_EL1::A1::TTBR0 +
        // Our Intermediate Physical Address (IPA) is 4TiB large
        TCR_EL1::IPS::Bits_42 +
        // TCR_EL1::IPS::Bits_32 +
        // Inner shareable (idk what this means atm)
        TCR_EL1::SH0::Inner +
        // 64-bit granule size for TTBR0
        TCR_EL1::TG0::KiB_4 +
        // On TLB miss, walk translation table instead of faulting
        TCR_EL1::EPD0::EnableTTBR0Walks +
        TCR_EL1::EPD1::EnableTTBR1Walks +
        TCR_EL1::T0SZ.val(0x19/* 64 - 48#<{(| mmap::END_RAM_ADDR.trailing_zeros() as u64 |)}># */) +
        TCR_EL1::IRGN0::WriteBack_ReadAlloc_NoWriteAlloc_Cacheable +
        TCR_EL1::ORGN0::WriteBack_ReadAlloc_NoWriteAlloc_Cacheable
     );

    // TCR_EL1.write(
    //     TCR_EL1::TBI0::Used +
    //     TCR_EL1::A1::TTBR0 +
    //     // Our Intermediate Physical Address (IPA) is 4TiB large
    //     // TCR_EL1::IPS::Bits_42 +
    //     TCR_EL1::IPS::Bits_40 +
    //     // Inner shareable (idk what this means atm)
    //     TCR_EL1::SH0::Inner +
    //     // 64-bit granule size for TTBR0
    //     TCR_EL1::TG0::KiB_64 +
    //     // On TLB miss, walk translation table instead of faulting
    //     TCR_EL1::EPD0::EnableTTBR0Walks +
    //     TCR_EL1::EPD1::EnableTTBR1Walks +
    //     TCR_EL1::T0SZ.val(mmap::END_RAM_ADDR.trailing_zeros() as u64) +
    //     TCR_EL1::IRGN0::WriteBack_ReadAlloc_WriteAlloc_Cacheable +
    //     TCR_EL1::ORGN0::WriteBack_ReadAlloc_WriteAlloc_Cacheable
    //  );

    println!("Populated tables and set mmu args. Enabling mmu");
    // Not sure what this argument does
    barrier::isb(barrier::SY);

    // Actually enable the MMU
    SCTLR_EL1.modify(SCTLR_EL1::M::SET);

    // Again, not sure what this argument does
    barrier::isb(barrier::SY);

    Ok(())
}


type TableDescriptionR = InMemoryRegister<u64, TableDescriptor::Register>;
type PageEntryR = InMemoryRegister<u64, PageEntry::Register>;

register_bitfields! {
    u64,

    TableDescriptor [
        VALID                               OFFSET(0)   NUMBITS(1) [],
        TYPE                                OFFSET(1)   NUMBITS(1) [
            Block = 0,
            Table = 1,
        ],
        // Some reserved bits here

        // https://armv8-ref.codingbelief.com/en/chapter_d4/d43_1_vmsav8-64_translation_table_descriptor_formats.html
        // Actual offset changes based on page size
        OUTPUT_ADDR                         OFFSET(12)  NUMBITS(36) [],
        // Some RESERVED bits here

        TABLE_PRIVILEGED_EXECUTE_NEVER      OFFSET(59)  NUMBITS(1) [],
        TABLE_UNPRIVILEGED_EXECUTE_NEVER    OFFSET(60)  NUMBITS(1) [],
        TABLE_ACCESS_PERMISSION             OFFSET(61)  NUMBITS(2) [
            /// Unprivelaged = nothing, Privelaged = read/write
            UnPrw = 0b00,
            /// Unprivelaged = read/write, Privelaged = read/write
            UrwPrw = 0b01,
            /// Unprivelaged = nothing, Privelaged = read
            UnPr = 0b10,
            /// Unprivelaged = read, Privelaged = read
            UrPr = 0b11,
        ],
        TABLE_NON_SECURE_ACCESS             OFFSET(63)  NUMBITS(1) [],

    ],

    // https://developer.arm.com/documentation/102376/0200/Describing-memory-in-AArch64
    // https://developer.arm.com/documentation/den0024/a/The-Memory-Management-Unit/Translation-tables-in-ARMv8-A/AArch64-descriptor-format
    PageEntry [
        VALID                       OFFSET(0)   NUMBITS(1) [],
        TYPE                        OFFSET(1)   NUMBITS(1) [
            Block = 0,
            Page = 1,
        ],
        ATTRIB_INDEX                OFFSET(2)   NUMBITS(2) [],
        // https://developer.arm.com/documentation/den0024/a/The-Memory-Management-Unit/Security-and-the-MMU
        // Whether page can be accessed in non-secure code
        NON_SECURE_ACCESS           OFFSET(5)   NUMBITS(1) [],
        ACCESS_PERMISSION           OFFSET(6)   NUMBITS(2) [
            /// Unprivelaged = nothing, Privelaged = read/write
            UnPrw = 0b00,
            /// Unprivelaged = read/write, Privelaged = read/write
            UrwPrw = 0b01,
            /// Unprivelaged = nothing, Privelaged = read
            UnPr = 0b10,
            /// Unprivelaged = read, Privelaged = read
            UrPr = 0b11,
        ],
        // https://developer.arm.com/documentation/den0024/a/The-Memory-Management-Unit/Translation-table-configuration
        SHAREABILITY                OFFSET(8)   NUMBITS(2) [
            NonShareable = 0b00,
            Unpredictable = 0b01,
            OuterShareable = 0b10,
            InnerShareable = 0b11,
        ],
        // https://developer.arm.com/documentation/den0024/a/The-Memory-Management-Unit/Operating-system-use-of-translation-table-descriptors
        // 0 when block hasn't been used yet, 1 when it has been used
        // Triggers MMU fault if page accessed while 0
        ACCESS_FLAG                 OFFSET(10)  NUMBITS(1) [],
        // https://developer.arm.com/documentation/den0024/a/The-Memory-Management-Unit/Context-switching
        // Whether page is associated with specific task/application
        // Address Space ID (ASID) is stored in TLB when non-global set
        // TLB will match on ASID when looking up
        NON_GLOBAL                  OFFSET(11)  NUMBITS(1) [],
        // https://armv8-ref.codingbelief.com/en/chapter_d4/d43_2_armv8_translation_table_level_3_descriptor_formats.html
        // Actual offset changes based on page size
        OUTPUT_ADDR                 OFFSET(12)  NUMBITS(36) [],
        // Some RESERVED bits here

        // Indicates if page is dirty
        // Used only if the field HD of TCR_ELx register is set
        DIRTY_BIT_MODIFIER          OFFSET(51)  NUMBITS(1) [],

        // Translation table continuous with prev one for initial lookup
        CONTIGUOUS                  OFFSET(52)  NUMBITS(1) [],
        // https://developer.arm.com/documentation/den0024/a/The-Memory-Management-Unit/Access-permissions
        // Allow (un)priveleged execution level to execute in this memory location
        PRIVILEGED_EXECUTE_NEVER    OFFSET(53)  NUMBITS(1) [],
        UNPRIVILEGED_EXECUTE_NEVER  OFFSET(54)  NUMBITS(1) [],
        OS_USE                      OFFSET(55)  NUMBITS(4) [],
        // For HW use I think?
        IMP_DEF_OR_IGNORED          OFFSET(59)  NUMBITS(3) [],

    ],
}
