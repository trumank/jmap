use crate::mem::Mem;
use anyhow::Result as AnyhowResult;
use rdex::{
    ArgumentType, DumpExec, FunctionExecutor, ModuleInfo, ProcessArchitecture, ProcessTrait,
};
use remu64::error::{EmulatorError, Result};
use remu64::memory::{MemoryRegionMut, MemoryRegionRef, MemoryTrait, Permission};
use std::ops::Range;

/// Memory adapter that implements amd64-emu's MemoryTrait on top of the dumper's Mem trait
pub struct MemoryAdapter<M: Mem> {
    mem: M,
}

impl<M: Mem> MemoryAdapter<M> {
    pub fn new(mem: M) -> Self {
        Self { mem }
    }
    pub fn inner(&self) -> &M {
        &self.mem
    }
}

impl<M: Mem> MemoryTrait for MemoryAdapter<M> {
    fn find_region(&self, _addr: u64) -> Option<MemoryRegionRef<'_>> {
        None
    }
    fn find_region_mut(&mut self, _addr: u64) -> Option<MemoryRegionMut<'_>> {
        None
    }
    fn read(&self, addr: u64, buf: &mut [u8]) -> Result<()> {
        self.mem.read_buf(addr as usize, buf).map_err(|e| {
            EmulatorError::InvalidArgument(format!("Memory read failed at 0x{:x}: {}", addr, e))
        })
    }
    fn write(&mut self, _addr: u64, _data: &[u8]) -> Result<()> {
        unimplemented!()
    }
    fn write_code(&mut self, _addr: u64, _data: &[u8]) -> Result<()> {
        unimplemented!()
    }
    fn map(&mut self, _addr: u64, _size: usize, _perms: Permission) -> Result<()> {
        unimplemented!()
    }
    fn unmap(&mut self, _addr: u64, _size: usize) -> Result<()> {
        unimplemented!()
    }
    fn protect(&mut self, _addr: u64, _size: usize, _perms: Permission) -> Result<()> {
        unimplemented!()
    }
    fn permissions(&self, _addr: u64) -> Result<Permission> {
        Ok(Permission::ALL)
    }
}
#[derive(Clone)]
pub struct MemProcess<M: Mem> {
    mem: M,
}

impl<M: Mem> MemProcess<M> {
    pub fn new(mem: M) -> Self {
        Self { mem }
    }
}

impl<M: Mem + Clone> ProcessTrait for MemProcess<M> {
    type Memory = MemoryAdapter<M>;

    fn get_module_by_name(&self, _name: &str) -> Option<ModuleInfo> {
        None
    }
    fn get_module_base_address(&self, _name: &str) -> Option<u64> {
        None
    }
    fn list_modules(&self) -> Vec<ModuleInfo> {
        vec![]
    }
    fn find_module_for_address(&self, _address: u64) -> Option<(String, u64, u64)> {
        None
    }
    fn create_memory(&self) -> AnyhowResult<Self::Memory> {
        Ok(MemoryAdapter::new(self.mem.clone()))
    }
    fn get_teb_address(&self) -> AnyhowResult<u64> {
        Ok(0x880140d000)
    }
    fn get_architecture(&self) -> ProcessArchitecture {
        ProcessArchitecture::X64
    }
}

pub struct FNameConverter<P: ProcessTrait> {
    fname_tostring_address: u64,
    executor: FunctionExecutor<P>,
}

impl<P: ProcessTrait + Clone> FNameConverter<P> {
    pub fn new(process: P, fname_tostring_address: u64) -> AnyhowResult<Self> {
        let executor = DumpExec::create_executor(process.clone())?;

        Ok(Self {
            fname_tostring_address,
            executor,
        })
    }

    /// Convert single FName to String using raw bytes (optimized for caching)
    pub fn convert_fname_from_bytes(&mut self, fname_bytes: &[u8]) -> AnyhowResult<String> {
        // Reset executor for reuse
        self.executor.reset_for_reuse()?;

        let fname_addr = self.executor.push_bytes_to_stack(fname_bytes)?;
        let fstring_addr = self.executor.push_bytes_to_stack(&[0; 16])?;

        let args = vec![
            ArgumentType::Pointer(fname_addr),
            ArgumentType::Pointer(fstring_addr),
        ];

        self.executor
            .execute_function(self.fname_tostring_address, args)?;

        // Read the resulting FString inline
        let mut fstring_data = [0u8; 16];
        self.executor
            .vm_context
            .engine
            .memory
            .read(fstring_addr, &mut fstring_data)?;

        let data_ptr = u64::from_le_bytes(fstring_data[0..8].try_into().unwrap());
        let num = u32::from_le_bytes(fstring_data[8..12].try_into().unwrap());

        if data_ptr == 0 || num == 0 {
            let empty_result = String::new();
            return Ok(empty_result);
        }

        let mut wide_char_data = vec![0u8; num as usize * 2];
        self.executor
            .vm_context
            .engine
            .memory
            .read(data_ptr, &mut wide_char_data)?;

        let wide_chars: Vec<u16> = wide_char_data
            .chunks_exact(2)
            .map(|chunk| u16::from_le_bytes(chunk.try_into().unwrap()))
            .collect();

        let len = wide_chars
            .iter()
            .position(|&c| c == 0)
            .unwrap_or(wide_chars.len());

        Ok(String::from_utf16(&wide_chars[..len])?)
    }
}
