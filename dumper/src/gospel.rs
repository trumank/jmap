use crate::{
    gospel_bindings::*,
    mem::{Ctx, Ptr},
};
use anyhow::Result;
use gospel_runtime::static_type_wrappers::Ref;
use ue_reflection::EClassCastFlags;

impl FName {
    pub fn to_string<C: Ctx>(self: &Ref<C, FName>) -> Result<String> {
        let memory = self.inner_ptr.opaque_ptr.memory.as_ref();

        let number = self.number().unwrap().read()?;
        let value = self
            .comparison_index()
            .unwrap()
            .cast::<FNameEntryId>()
            .unwrap()
            .value()
            .read()?;

        let case_preserving = memory.case_preserving();

        if memory.ue_version() < (4, 22) {
            // self.get_inner_ptr().metadata

            // let chunks = DynamicPtr {
            //     opaque_ptr: OpaquePtr {
            //         memory: memory.clone(),
            //         address: memory.fnamepool.0 as u64,
            //     },
            //     metadata: self.get_inner_ptr().metadata,
            // };

            // wtf :skull_emoji:
            // let chunks = self
            //     .map(|_| )
            //     .cast::<Ptr<Ptr<Ptr<(), C>, C>, C>>()
            //     .read()?;

            let chunks =
                Ptr::<Ptr<Ptr<Ptr<(), C>, C>, C>, C>::new(memory.fnamepool().0, memory.clone())
                    .read()?;

            let per_chunk = 0x4000;

            let chunk = value / per_chunk;
            let offset = value % per_chunk;

            let chunk = chunks.offset(chunk as usize).read()?;
            let entry = chunk.offset(offset as usize).read()?;

            let index = entry.cast::<u32>().read()?;
            let is_wide = (index & 1) == 1;
            let char_data = entry.byte_offset(0x10);

            let base = if is_wide {
                let mut data = vec![];
                let char_data = char_data.cast::<u16>();
                for i in 0.. {
                    let next = char_data.offset(i).read()?;
                    if next == 0 {
                        break;
                    }
                    data.push(next);
                }
                String::from_utf16(&data)?
            } else {
                let mut data = vec![];
                let char_data = char_data.cast::<u8>();
                for i in 0.. {
                    let next = char_data.offset(i).read()?;
                    if next == 0 {
                        break;
                    }
                    data.push(next);
                }
                String::from_utf8(data)?
            };
            return Ok(if number == 0 {
                base
            } else {
                format!("{base}_{}", number - 1)
            });
        }

        let blocks = Ptr::<Ptr<u8, C>, C>::new(memory.fnamepool().0 + 0x10, memory.clone());

        let block_index = (value >> 16) as usize;
        let offset = if case_preserving {
            (value & 0xffff) as usize * 4 + 4
        } else {
            (value & 0xffff) as usize * 2
        };

        let block = blocks.offset(block_index).read()?;
        let header = block.offset(offset).cast::<u16>().read()?;

        let len = if case_preserving {
            (header >> 1) as usize
        } else {
            (header >> 6) as usize
        };
        let is_wide = header & 1 != 0;

        let base = if is_wide {
            String::from_utf16(
                &block
                    .offset(offset + 2)
                    .read_vec(len * 2)?
                    .chunks(2)
                    .map(|chunk| u16::from_le_bytes(chunk.try_into().unwrap()))
                    .collect::<Vec<_>>(),
            )?
        } else {
            String::from_utf8(block.offset(offset + 2).read_vec(len)?)?
        };
        Ok(if number == 0 {
            base
        } else {
            format!("{base}_{}", number - 1)
        })
    }
}

impl FUObjectArray {
    pub fn num_elements<C: Ctx>(self: &Ref<C, FUObjectArray>) -> Result<usize> {
        println!("{:x}", self.inner_ptr.opaque_ptr.address);
        // let memory = self.get_inner_ptr().opaque_ptr.memory.as_ref();

        println!("{:?}", self.obj_objects_critical());
        println!("{:?}", self.obj_objects());
        // dbg!(self.obj_objects());
        let objects = self.obj_objects().unwrap();
        if let Some(chunked_objects) = objects.cast::<FChunkedFixedUObjectArray>() {
            Ok(chunked_objects.num_elements().read()? as usize)
        } else {
            todo!()
        }
    }
    pub fn read_item_ptr<C: Ctx>(
        self: &Ref<C, FUObjectArray>,
        item: usize,
    ) -> Result<Option<Ref<C, UObject>>> {
        let objects = self.obj_objects().unwrap();
        if let Some(chunked_objects) = objects.cast::<FChunkedFixedUObjectArray>() {
            let max_per_chunk = 64 * 1024;
            let chunk_index = item / max_per_chunk;
            let item_index = item % max_per_chunk;

            let object_item = chunked_objects
                .objects()
                .read()?
                .to_ref_checked()
                .add_unchecked(chunk_index)
                .read()?
                .to_ref_checked()
                .add_unchecked(item_index);

            Ok(object_item.object().read()?.to_ref())
        } else {
            todo!()
        }
    }
}

impl UClass {
    fn class_cast_flags_enum<C: Ctx>(self: &Ref<C, UClass>) -> Result<EClassCastFlags> {
        Ok(EClassCastFlags::from_bits(self.class_cast_flags().read()?).unwrap())
    }
}

impl UObject {
    pub fn path<C: Ctx>(self: &Ref<C, UObject>) -> Result<String> {
        let mut objects = vec![self.clone()];

        let mut obj = self.clone();
        while let Some(outer) = obj.outer_private().read()?.to_ref() {
            objects.push(outer.clone());
            obj = outer;
        }

        let mut path = String::new();
        let mut prev: Option<Ref<C, UObject>> = None;
        for obj in objects.iter().rev() {
            if let Some(prev) = prev {
                let sep = if prev
                    .class_private()
                    .read()?
                    .to_ref_checked()
                    .class_cast_flags_enum()?
                    .contains(EClassCastFlags::CASTCLASS_UPackage)
                {
                    '.'
                } else {
                    ':'
                };
                path.push(sep);
            }
            path.push_str(&obj.name_private().to_string()?);
            prev = Some(obj.clone());
        }

        Ok(path)
    }
}

impl UStruct {
    pub fn properties<C: Ctx>(
        self: &Ref<C, UStruct>,
        recurse_parents: bool,
    ) -> PropertyIterator<C> {
        PropertyIterator {
            current_struct: Some(self.clone()),
            current_field: None,
            recurse_parents,
        }
    }
    pub fn child_fields<C: Ctx>(self: &Ref<C, UStruct>) -> Result<Option<Ref<C, ZField>>> {
        let children = if let Some(children) = self.child_properties() {
            children.read()?.cast_checked::<ZField>()
        } else {
            self.children().read()?.cast_checked::<ZField>()
        };
        Ok(children.to_ref())
    }
    pub fn get_min_alignment<C: Ctx>(self: &Ref<C, UStruct>) -> Result<u32> {
        let align = self.min_alignment().unwrap();
        Ok(if let Some(n) = align.cast::<i32>() {
            n.read()? as u32
        } else if let Some(n) = align.cast::<i16>() {
            n.read()? as u32
        } else {
            unreachable!()
        })
    }
}

impl ZField {
    pub fn name<C: Ctx>(self: &Ref<C, ZField>) -> Result<String> {
        if let Some(base) = self.cast::<UObject>() {
            base.name_private().to_string()
        } else if let Some(base) = self.cast::<FField>() {
            base.name_private().to_string()
        } else {
            unreachable!()
        }
    }
    pub fn cast_flags<C: Ctx>(self: &Ref<C, ZField>) -> Result<EClassCastFlags> {
        let cast_flags = if let Some(base) = self.cast::<UObject>() {
            base.class_private()
                .read()?
                .to_ref_checked()
                .class_cast_flags()
                .read()?
        } else if let Some(base) = self.cast::<FField>() {
            base.class_private()
                .read()?
                .to_ref_checked()
                .cast_flags()
                .read()?
        } else {
            unreachable!()
        };
        Ok(EClassCastFlags::from_bits(cast_flags).unwrap())
    }
    pub fn next<C: Ctx>(self: &Ref<C, ZField>) -> Result<Option<Ref<C, ZField>>> {
        Ok(if let Some(base) = self.cast::<UField>() {
            base.next().read()?.cast_checked().to_ref()
        } else if let Some(base) = self.cast::<FField>() {
            base.next().read()?.cast_checked().to_ref()
        } else {
            unreachable!()
        })
    }
}

#[derive(Clone)]
pub struct PropertyIterator<C: Ctx> {
    current_struct: Option<Ref<C, UStruct>>,
    current_field: Option<Ref<C, ZField>>,
    recurse_parents: bool,
}

impl<C: Ctx> Iterator for PropertyIterator<C> {
    type Item = Result<Ref<C, ZProperty>>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(current) = self.current_field.take() {
                let is_property = match current.cast_flags() {
                    Ok(flags) if flags.contains(EClassCastFlags::CASTCLASS_FProperty) => true,
                    Ok(_) => false,
                    Err(e) => return Some(Err(e)),
                };

                let next = current.next();
                self.current_field = match next {
                    Ok(next) => next,
                    Err(e) => return Some(Err(e)),
                };

                if is_property {
                    return Some(Ok(current.cast_checked::<ZProperty>()));
                }
            } else if let Some(current) = self.current_struct.take() {
                self.current_field = match current.child_fields() {
                    Ok(children) => children,
                    Err(e) => return Some(Err(e)),
                };

                if self.recurse_parents {
                    self.current_struct = match current.super_struct().read() {
                        Ok(super_struct) => super_struct.to_ref(),
                        Err(e) => return Some(Err(e)),
                    };
                }
            } else {
                return None;
            }
        }
    }
}
