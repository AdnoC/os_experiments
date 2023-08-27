use spin::Mutex;
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

pub mod mmap {
    use crate::units::GIB;

    pub const END_RAM_ADDR: usize = (4 * GIB - 1) as usize;
}


mod page_size_64_kb {
    use crate::units::{TIB, MIB, KIB};

    const SIZE: usize = 64 * KIB as usize;
    // With 64KB ganules we don't use level0
    pub const LEVEL0_TABLE_SIZE: usize = 1;
    pub const LEVEL0_TABLE_COVERAGE: usize = 4 * TIB as usize;
    pub const LEVEL1_TABLE_SIZE: usize = 64;
    pub const LEVEL1_TABLE_COVERAGE: usize = 4 * TIB as usize;
    pub const LEVEL2_TABLE_SIZE: usize = 8192;
    pub const LEVEL2_TABLE_COVERAGE: usize = 512 * MIB as usize;
    pub const LEVEL3_TABLE_SIZE: usize = 8192;
    pub const LEVEL3_TABLE_COVERAGE: usize = 64 * KIB as usize;
}
mod page_size {
    pub use super::page_size_64_kb::*;
}


static TRANLSATION_TABLES: Mutex<TranslationTable> = Mutex::new(TranslationTable::new());


const NUM_LEVEL_0: usize = page_size::LEVEL0_TABLE_SIZE;
const NUM_LEVEL_1: usize = max(mmap::END_RAM_ADDR / page_size::LEVEL1_TABLE_COVERAGE, 1);
const NUM_LEVEL_2: usize = max(mmap::END_RAM_ADDR / page_size::LEVEL2_TABLE_COVERAGE, 1);
const NUM_LEVEL_3: usize = page_size::LEVEL3_TABLE_SIZE;


struct TranslationTable {
    level0: [TableDescriptionR; NUM_LEVEL_0],
    level1: [[TableDescriptionR; NUM_LEVEL_1]; NUM_LEVEL_0],
    level2: [[[TableDescriptionR; NUM_LEVEL_2]; NUM_LEVEL_1]; NUM_LEVEL_0],
    level3: [[[[PageEntryR; NUM_LEVEL_3]; NUM_LEVEL_2]; NUM_LEVEL_1]; NUM_LEVEL_0],
}

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
                level0: transmute([0u64; NUM_LEVEL_0]),
                level1: transmute([[0u64; NUM_LEVEL_1]; NUM_LEVEL_0]),
                level2: transmute([[[0u64; NUM_LEVEL_2]; NUM_LEVEL_1]; NUM_LEVEL_0]),
                level3: transmute([[[[0u64; NUM_LEVEL_3]; NUM_LEVEL_2]; NUM_LEVEL_1]; NUM_LEVEL_0]),
            }
        }
    }
}

pub fn populate_table_entries() {
    let mut table = TRANLSATION_TABLES.lock();
    let table = &mut *table;
    let l0_shift = page_size::LEVEL0_TABLE_COVERAGE.trailing_zeros();
    let l1_shift = page_size::LEVEL1_TABLE_COVERAGE.trailing_zeros();
    let l2_shift = page_size::LEVEL2_TABLE_COVERAGE.trailing_zeros();
    let l3_shift = page_size::LEVEL3_TABLE_COVERAGE.trailing_zeros();
    for (l0_idx, l0_entry) in table.level0.iter_mut().enumerate() {
        let l1_addr = &table.level1[l0_idx] as *const TableDescriptionR as usize as u64;
        let l1_addr = l1_addr << l0_shift;
        l0_entry.write(
            TableDescriptor::VALID::SET +
            TableDescriptor::TYPE::Table +
            TableDescriptor::OUTPUT_ADDR.val(l1_addr) +
            TableDescriptor::TABLE_ACCESS_PERMISSION::UrwPrw
        );

        for (l1_idx, l1_entry) in table.level1[l0_idx].iter_mut().enumerate() {
            let l2_addr = &table.level2[l0_idx][l1_idx] as *const TableDescriptionR as usize as u64;
            let l2_addr = l2_addr << l1_shift;
            l1_entry.write(
                TableDescriptor::VALID::SET +
                TableDescriptor::TYPE::Table +
                TableDescriptor::OUTPUT_ADDR.val(l2_addr) +
                TableDescriptor::TABLE_ACCESS_PERMISSION::UrwPrw
            );

            for (l2_idx, l2_entry) in table.level2[l0_idx][l1_idx].iter_mut().enumerate() {
                let l3_addr = &table.level3[l0_idx][l1_idx][l2_idx] as *const PageEntryR as usize as u64;
                let l3_addr = l3_addr << l2_shift;
                l2_entry.write(
                    TableDescriptor::VALID::SET +
                    TableDescriptor::TYPE::Table +
                    TableDescriptor::OUTPUT_ADDR.val(l3_addr) +
                    TableDescriptor::TABLE_ACCESS_PERMISSION::UrwPrw
                );


                for (l3_idx, l3_entry) in table.level3[l0_idx][l1_idx][l2_idx].iter_mut().enumerate() {
                    let phys_addr = (l0_idx << l0_shift) & (l1_idx << l1_shift) & (l2_idx << l2_shift) & (l3_idx << l3_shift);
                    l3_entry.write(
                        PageEntry::VALID::SET +
                        PageEntry::TYPE::Block +
                        PageEntry::ACCESS_FLAG::SET +
                        PageEntry::OUTPUT_ADDR.val(phys_addr as u64)
                    );
                }
            }
        }
    }
}

pub fn init_virtual_memory() -> Result<(), &'static str> {
    use aarch64_cpu::{
        registers::*,
        asm::barrier,
    };

    populate_table_entries();
    let table_base_addr = &TRANLSATION_TABLES.lock().level1[0] as *const TableDescriptionR as usize as u64;
    // Set the address of the translation tables for lower half of virt address space
    TTBR0_EL1.set_baddr(table_base_addr);
    // Set the address of the translation tables for upper half of virt address space
    TTBR1_EL1.set_baddr(table_base_addr);

    TCR_EL1.write(
        // Our Intermediate Physical Address (IPA) is 4TiB large
        TCR_EL1::IPS::Bits_42 +
        // Inner shareable (idk what this means atm)
        TCR_EL1::SH0::Inner +
        // 64-bit granule size for TTBR0
        TCR_EL1::TG0::KiB_64 +
        // On TLB miss, walk translation table instead of faulting
        TCR_EL1::EPD0::EnableTTBR0Walks +
        TCR_EL1::T0SZ.val(mmap::END_RAM_ADDR.trailing_zeros() as u64) +
        TCR_EL1::IRGN0::WriteBack_ReadAlloc_WriteAlloc_Cacheable +
        TCR_EL1::ORGN0::WriteBack_ReadAlloc_WriteAlloc_Cacheable
     );

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
        OUTPUT_ADDR                         OFFSET(12)  NUMBITS(35) [],
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
            Table = 1,
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
        OUTPUT_ADDR                 OFFSET(12)  NUMBITS(35) [],
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
