use std::collections::HashMap;
use std::marker::PhantomData;
use std::mem::MaybeUninit;
use std::sync::{Arc, Mutex};

use anyhow::Result;
use derive_where::derive_where;
use serde::Serialize;

use alloc::*;

#[repr(C)]
pub struct ExternalPtr<T> {
    address: usize,
    _type: PhantomData<T>,
}
impl<T> Copy for ExternalPtr<T> {}
impl<T> Clone for ExternalPtr<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> std::fmt::Debug for ExternalPtr<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ExternalPtr(0x{:x})", self.address)
    }
}
impl<T> ExternalPtr<T> {
    pub fn new(address: usize) -> Self {
        Self {
            address,
            _type: Default::default(),
        }
    }
    pub fn is_null(self) -> bool {
        self.address == 0
    }
    pub fn cast<O>(self) -> ExternalPtr<O> {
        ExternalPtr::new(self.address)
    }
    pub fn offset(&self, n: usize) -> Self {
        Self::new(self.address + n * std::mem::size_of::<T>())
    }
    pub fn read(&self, mem: &impl Mem) -> Result<T> {
        mem.read(self.address)
    }
    pub fn read_opt(&self, mem: &impl Mem) -> Result<Option<T>> {
        Ok(if self.is_null() {
            None
        } else {
            Some(mem.read(self.address)?)
        })
    }
    pub fn read_vec(&self, mem: &impl Mem, count: usize) -> Result<Vec<T>> {
        mem.read_vec(self.address, count)
    }
}

#[derive(Debug)]
pub enum FlaggedPtr<T> {
    Local(*const T),
    Remote(ExternalPtr<T>),
}
impl<T> Copy for FlaggedPtr<T> {}
impl<T> Clone for FlaggedPtr<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> FlaggedPtr<T> {
    pub fn is_null(self) -> bool {
        match self {
            FlaggedPtr::Local(ptr) => ptr.is_null(),
            FlaggedPtr::Remote(ptr) => ptr.is_null(),
        }
    }
}
impl<T: Clone> FlaggedPtr<T> {
    pub fn read(self, mem: &impl Mem) -> Result<T> {
        Ok(match self {
            FlaggedPtr::Local(ptr) => unsafe { ptr.read() },
            FlaggedPtr::Remote(ptr) => ptr.read(mem)?,
        })
    }
    pub fn read_vec(self, mem: &impl Mem, count: usize) -> Result<Vec<T>> {
        Ok(if self.is_null() {
            vec![]
        } else {
            match self {
                FlaggedPtr::Local(ptr) => unsafe {
                    std::slice::from_raw_parts(ptr, count).to_vec()
                },
                FlaggedPtr::Remote(ptr) => ptr.read_vec(mem, count)?,
            }
        })
    }
}

type FString = TArray<u16>;

#[derive_where(Debug, Clone, Copy; T, A::ForElementType<T>)]
#[repr(C)]
pub struct TArray<T, A: TAlloc = TSizedHeapAllocator<32>> {
    pub data: A::ForElementType<T>,
    pub num: u32,
    pub max: u32,
}
impl<T: Clone> TArray<T> {
    pub fn read(&self, mem: &impl Mem) -> Result<Vec<T>> {
        self.data.data().read_vec(mem, self.num as usize)
    }
}

#[derive_where(Debug, Clone, Copy; A::ForElementType<u32>)]
#[repr(C)]
struct TBitArray<A: TAlloc> {
    pub allocator_instance: A::ForElementType<u32>,
    pub num_bits: i32,
    pub max_bits: i32,
}

#[derive_where(Debug, Clone, Copy; T, <A::ElementAllocator as TAlloc>::ForElementType<T>, <A::BitArrayAllocator as TAlloc>::ForElementType<u32>)]
#[repr(C)]
pub struct TSparseArray<T, A: TSparseAlloc = FDefaultSparseArrayAllocator> {
    // TArray<TSparseArrayElementOrFreeListLink<TAlignedBytes<32,8> >,TSizedDefaultAllocator<32> >
    pub data: TArray<T, A::ElementAllocator>,
    // TBitArray<FDefaultBitArrayAllocator>
    pub allocation_flags: TBitArray<A::BitArrayAllocator>,
    pub first_free_index: i32,
    pub num_free_indices: i32,
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct TMap<K, V> {
    pub base: TSortableMapBase<K, V>,
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct TSortableMapBase<K, V> {
    pub base: TMapBase<K, V>,
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct TMapBase<K, V> {
    pub pairs: TSet<TTuple<K, V>>,
}
//TSet<TTuple<FName,FString>,TDefaultMapHashableKeyFuncs<FName,FString,0>,FDefaultSetAllocator>

#[derive_where(Debug, Clone, Copy; T,
    <<<A as TSetAlloc>::SparseArrayAllocator as TSparseAlloc>::BitArrayAllocator as TAlloc>::ForElementType<u32>,
    <<<A as TSetAlloc>::SparseArrayAllocator as TSparseAlloc>::ElementAllocator as TAlloc>::ForElementType<TSetElement<T>>,
    <<A as TSetAlloc>::HashAllocator as TAlloc>::ForElementType<FSetElementId>,
)]
#[repr(C)]
pub struct TSet<T, A: TSetAlloc = FDefaultSetAllocator> {
    // TODO hash functions
    pub elements: TSparseArray<TSetElement<T>, <A as TSetAlloc>::SparseArrayAllocator>,
    pub hash: <<A as TSetAlloc>::HashAllocator as TAlloc>::ForElementType<FSetElementId>,
    pub hash_size: i32,
}

const ASDF2: [u8; 0x50] = [0; std::mem::size_of::<TSet<TTuple<FName, ExternalPtr<()>>>>()];

//#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct TSparseArray_TBaseIterator<const N: usize, T, A: TSparseAlloc> {
    pub array: ExternalPtr<TSparseArray<T, A>>,
    pub bit_array_it: TConstSetBitIterator<A::BitArrayAllocator>,
}

mod alloc {
    use std::marker::PhantomData;

    use super::{ExternalPtr, FlaggedPtr};

    pub type FDefaultAllocator = TSizedDefaultAllocator<32>;
    pub type TSizedDefaultAllocator<const P: usize> = TSizedHeapAllocator<P>;
    pub type FDefaultBitArrayAllocator = TInlineAllocator<4, FDefaultAllocator>;
    pub type FDefaultSetAllocator = TSetAllocator;

    pub trait TAlloc {
        type ForElementType<T>: TAllocImpl<T>;
    }
    pub trait TAllocImpl<T> {
        fn data(&self) -> FlaggedPtr<T>;
    }

    #[derive(Debug, Clone, Copy)]
    pub struct TInlineAllocator<const N: usize, A: TAlloc = FDefaultAllocator>(PhantomData<A>);
    impl<const N: usize, A: TAlloc> TAlloc for TInlineAllocator<N, A> {
        type ForElementType<T> = TInlineAlloc_ForElementType<N, T, A>;
    }
    #[derive(Debug, Clone, Copy)]
    #[repr(C)]
    pub struct TInlineAlloc_ForElementType<const N: usize, T, A: TAlloc> {
        inline_data: [T; N],
        secondary_data: A::ForElementType<T>,
    }
    impl<const N: usize, T, A: TAlloc> TAllocImpl<T> for TInlineAlloc_ForElementType<N, T, A> {
        fn data(&self) -> FlaggedPtr<T> {
            let second = self.secondary_data.data();
            if second.is_null() {
                FlaggedPtr::Local(self.inline_data.as_ptr())
            } else {
                second
            }
        }
    }

    #[derive(Debug, Clone, Copy)]
    pub struct TSizedHeapAllocator<const N: usize>;
    impl<const N: usize> TAlloc for TSizedHeapAllocator<N> {
        type ForElementType<T> = THeapAlloc_ForElementType<N, T>;
    }
    #[derive(Debug, Clone, Copy)]
    #[repr(C)]
    pub struct THeapAlloc_ForElementType<const N: usize, T> {
        data: ExternalPtr<T>,
    }
    impl<const N: usize, T> TAllocImpl<T> for THeapAlloc_ForElementType<N, T> {
        fn data(&self) -> FlaggedPtr<T> {
            FlaggedPtr::Remote(self.data)
        }
    }

    pub trait TSparseAlloc {
        type ElementAllocator: TAlloc;
        type BitArrayAllocator: TAlloc;
    }
    pub struct FDefaultSparseArrayAllocator;
    impl TSparseAlloc for FDefaultSparseArrayAllocator {
        type ElementAllocator = FDefaultAllocator;
        type BitArrayAllocator = FDefaultBitArrayAllocator;
    }

    pub trait TSetAlloc {
        type SparseArrayAllocator: TSparseAlloc;
        type HashAllocator: TAlloc;
        const AverageNumberOfElementsPerHashBucket: usize;
        const BaseNumberOfHashBuckets: usize;
        const MinNumberOfHashedElements: usize;
    }
    pub struct TSetAllocator<
        S = FDefaultSparseArrayAllocator,
        H = TInlineAllocator<1, FDefaultAllocator>,
        const E: usize = 2,
        const B: usize = 8,
        const M: usize = 4,
    >(PhantomData<S>, PhantomData<H>);
    impl<S: TSparseAlloc, H: TAlloc, const E: usize, const B: usize, const M: usize> TSetAlloc
        for TSetAllocator<S, H, E, B, M>
    {
        type SparseArrayAllocator = S;
        type HashAllocator = H;
        const AverageNumberOfElementsPerHashBucket: usize = E;
        const BaseNumberOfHashBuckets: usize = B;
        const MinNumberOfHashedElements: usize = M;
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct FRelativeBitReference {
    pub DWORDIndex: i32,
    pub Mask: u32,
}

#[derive_where(Debug, Clone, Copy; <A as TAlloc>::ForElementType<u32>)]
#[repr(C)]
pub struct TConstSetBitIterator<A: TAlloc> {
    pub bit_reference: FRelativeBitReference,
    pub array: ExternalPtr<TBitArray<A>>,
    pub UnvisitedBitMask: u32,
    pub CurrentBitIndex: i32,
    pub BaseBitIndex: i32,
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct TSetElement<T> {
    pub inner: TSetElementBase<T, 1>,
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct TTuple<K, V> {
    pub key: K,
    pub value: V,
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct TSetElementBase<T, const N: usize> {
    pub Value: T,
    pub HashNextId: FSetElementId,
    pub HashIndex: i32,
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
struct FSetElementId {
    pub index: i32,
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy)]
    #[repr(C)]
    pub struct EObjectFlags: u32 {
        const RF_NoFlags = 0x0000;
        const RF_Public = 0x0001;
        const RF_Standalone = 0x0002;
        const RF_MarkAsNative = 0x0004;
        const RF_Transactional = 0x0008;
        const RF_ClassDefaultObject = 0x0010;
        const RF_ArchetypeObject = 0x0020;
        const RF_Transient = 0x0040;
        const RF_MarkAsRootSet = 0x0080;
        const RF_TagGarbageTemp = 0x0100;
        const RF_NeedInitialization = 0x0200;
        const RF_NeedLoad = 0x0400;
        const RF_KeepForCooker = 0x0800;
        const RF_NeedPostLoad = 0x1000;
        const RF_NeedPostLoadSubobjects = 0x2000;
        const RF_NewerVersionExists = 0x4000;
        const RF_BeginDestroyed = 0x8000;
        const RF_FinishDestroyed = 0x00010000;
        const RF_BeingRegenerated = 0x00020000;
        const RF_DefaultSubObject = 0x00040000;
        const RF_WasLoaded = 0x00080000;
        const RF_TextExportTransient = 0x00100000;
        const RF_LoadCompleted = 0x00200000;
        const RF_InheritableComponentTemplate = 0x00400000;
        const RF_DuplicateTransient = 0x00800000;
        const RF_StrongRefOnFrame = 0x01000000;
        const RF_NonPIEDuplicateTransient = 0x02000000;
        const RF_Dynamic = 0x04000000;
        const RF_WillBeLoaded = 0x08000000;
    }

    #[derive(Debug, Clone, Copy)]
    #[repr(C)]
    pub struct EFunctionFlags: u32 {
        const FUNC_None = 0x0000;
        const FUNC_Final = 0x0001;
        const FUNC_RequiredAPI = 0x0002;
        const FUNC_BlueprintAuthorityOnly = 0x0004;
        const FUNC_BlueprintCosmetic = 0x0008;
        const FUNC_Net = 0x0040;
        const FUNC_NetReliable = 0x0080;
        const FUNC_NetRequest = 0x0100;
        const FUNC_Exec = 0x0200;
        const FUNC_Native = 0x0400;
        const FUNC_Event = 0x0800;
        const FUNC_NetResponse = 0x1000;
        const FUNC_Static = 0x2000;
        const FUNC_NetMulticast = 0x4000;
        const FUNC_UbergraphFunction = 0x8000;
        const FUNC_MulticastDelegate = 0x00010000;
        const FUNC_Public = 0x00020000;
        const FUNC_Private = 0x00040000;
        const FUNC_Protected = 0x00080000;
        const FUNC_Delegate = 0x00100000;
        const FUNC_NetServer = 0x00200000;
        const FUNC_HasOutParms = 0x00400000;
        const FUNC_HasDefaults = 0x00800000;
        const FUNC_NetClient = 0x01000000;
        const FUNC_DLLImport = 0x02000000;
        const FUNC_BlueprintCallable = 0x04000000;
        const FUNC_BlueprintEvent = 0x08000000;
        const FUNC_BlueprintPure = 0x10000000;
        const FUNC_EditorOnly = 0x20000000;
        const FUNC_Const = 0x40000000;
        const FUNC_NetValidate = 0x80000000;
        const FUNC_AllFlags = 0xffffffff;
    }

    #[derive(Debug, Clone, Copy)]
    #[repr(C)]
    pub struct EClassFlags: i32 {
        const CLASS_None = 0x0000;
        const CLASS_Abstract = 0x0001;
        const CLASS_DefaultConfig = 0x0002;
        const CLASS_Config = 0x0004;
        const CLASS_Transient = 0x0008;
        const CLASS_Parsed = 0x0010;
        const CLASS_MatchedSerializers = 0x0020;
        const CLASS_ProjectUserConfig = 0x0040;
        const CLASS_Native = 0x0080;
        const CLASS_NoExport = 0x0100;
        const CLASS_NotPlaceable = 0x0200;
        const CLASS_PerObjectConfig = 0x0400;
        const CLASS_ReplicationDataIsSetUp = 0x0800;
        const CLASS_EditInlineNew = 0x1000;
        const CLASS_CollapseCategories = 0x2000;
        const CLASS_Interface = 0x4000;
        const CLASS_CustomConstructor = 0x8000;
        const CLASS_Const = 0x00010000;
        const CLASS_LayoutChanging = 0x00020000;
        const CLASS_CompiledFromBlueprint = 0x00040000;
        const CLASS_MinimalAPI = 0x00080000;
        const CLASS_RequiredAPI = 0x00100000;
        const CLASS_DefaultToInstanced = 0x00200000;
        const CLASS_TokenStreamAssembled = 0x00400000;
        const CLASS_HasInstancedReference = 0x00800000;
        const CLASS_Hidden = 0x01000000;
        const CLASS_Deprecated = 0x02000000;
        const CLASS_HideDropDown = 0x04000000;
        const CLASS_GlobalUserConfig = 0x08000000;
        const CLASS_Intrinsic = 0x10000000;
        const CLASS_Constructed = 0x20000000;
        const CLASS_ConfigDoNotCheckDefaults = 0x40000000;
        const CLASS_NewerVersionExists = i32::MIN;
    }


    #[derive(Debug, Clone, Copy)]
    #[repr(C)]
    pub struct EClassCastFlags : u64 {
        const CASTCLASS_None = 0x0000000000000000;

        const CASTCLASS_UField = 0x0000000000000001;
        const CASTCLASS_FInt8Property = 0x0000000000000002;
        const CASTCLASS_UEnum = 0x0000000000000004;
        const CASTCLASS_UStruct = 0x0000000000000008;
        const CASTCLASS_UScriptStruct = 0x0000000000000010;
        const CASTCLASS_UClass = 0x0000000000000020;
        const CASTCLASS_FByteProperty = 0x0000000000000040;
        const CASTCLASS_FIntProperty = 0x0000000000000080;
        const CASTCLASS_FFloatProperty = 0x0000000000000100;
        const CASTCLASS_FUInt64Property = 0x0000000000000200;
        const CASTCLASS_FClassProperty = 0x0000000000000400;
        const CASTCLASS_FUInt32Property = 0x0000000000000800;
        const CASTCLASS_FInterfaceProperty = 0x0000000000001000;
        const CASTCLASS_FNameProperty = 0x0000000000002000;
        const CASTCLASS_FStrProperty = 0x0000000000004000;
        const CASTCLASS_FProperty = 0x0000000000008000;
        const CASTCLASS_FObjectProperty = 0x0000000000010000;
        const CASTCLASS_FBoolProperty = 0x0000000000020000;
        const CASTCLASS_FUInt16Property = 0x0000000000040000;
        const CASTCLASS_UFunction = 0x0000000000080000;
        const CASTCLASS_FStructProperty = 0x0000000000100000;
        const CASTCLASS_FArrayProperty = 0x0000000000200000;
        const CASTCLASS_FInt64Property = 0x0000000000400000;
        const CASTCLASS_FDelegateProperty = 0x0000000000800000;
        const CASTCLASS_FNumericProperty = 0x0000000001000000;
        const CASTCLASS_FMulticastDelegateProperty = 0x0000000002000000;
        const CASTCLASS_FObjectPropertyBase = 0x0000000004000000;
        const CASTCLASS_FWeakObjectProperty = 0x0000000008000000;
        const CASTCLASS_FLazyObjectProperty = 0x0000000010000000;
        const CASTCLASS_FSoftObjectProperty = 0x0000000020000000;
        const CASTCLASS_FTextProperty = 0x0000000040000000;
        const CASTCLASS_FInt16Property = 0x0000000080000000;
        const CASTCLASS_FDoubleProperty = 0x0000000100000000;
        const CASTCLASS_FSoftClassProperty = 0x0000000200000000;
        const CASTCLASS_UPackage = 0x0000000400000000;
        const CASTCLASS_ULevel = 0x0000000800000000;
        const CASTCLASS_AActor = 0x0000001000000000;
        const CASTCLASS_APlayerController = 0x0000002000000000;
        const CASTCLASS_APawn = 0x0000004000000000;
        const CASTCLASS_USceneComponent = 0x0000008000000000;
        const CASTCLASS_UPrimitiveComponent = 0x0000010000000000;
        const CASTCLASS_USkinnedMeshComponent = 0x0000020000000000;
        const CASTCLASS_USkeletalMeshComponent = 0x0000040000000000;
        const CASTCLASS_UBlueprint = 0x0000080000000000;
        const CASTCLASS_UDelegateFunction = 0x0000100000000000;
        const CASTCLASS_UStaticMeshComponent = 0x0000200000000000;
        const CASTCLASS_FMapProperty = 0x0000400000000000;
        const CASTCLASS_FSetProperty = 0x0000800000000000;
        const CASTCLASS_FEnumProperty = 0x0001000000000000;
        const CASTCLASS_USparseDelegateFunction = 0x0002000000000000;
        const CASTCLASS_FMulticastInlineDelegateProperty = 0x0004000000000000;
        const CASTCLASS_FMulticastSparseDelegateProperty = 0x0008000000000000;
        const CASTCLASS_FFieldPathProperty = 0x0010000000000000;
        const CASTCLASS_FLargeWorldCoordinatesRealProperty = 0x0080000000000000;
        const CASTCLASS_FOptionalProperty = 0x0100000000000000;
        const CASTCLASS_FVerseValueProperty = 0x0200000000000000;
        const CASTCLASS_UVerseVMClass = 0x0400000000000000;
    }

    #[derive(Debug, Clone, Copy, Serialize)]
    #[repr(C)]
    pub struct  EPropertyFlags: u64 {
        const CPF_None = 0x0000;
        const CPF_Edit = 0x0001;
        const CPF_ConstParm = 0x0002;
        const CPF_BlueprintVisible = 0x0004;
        const CPF_ExportObject = 0x0008;
        const CPF_BlueprintReadOnly = 0x0010;
        const CPF_Net = 0x0020;
        const CPF_EditFixedSize = 0x0040;
        const CPF_Parm = 0x0080;
        const CPF_OutParm = 0x0100;
        const CPF_ZeroConstructor = 0x0200;
        const CPF_ReturnParm = 0x0400;
        const CPF_DisableEditOnTemplate = 0x0800;
        const CPF_Transient = 0x2000;
        const CPF_Config = 0x4000;
        const CPF_DisableEditOnInstance = 0x00010000;
        const CPF_EditConst = 0x00020000;
        const CPF_GlobalConfig = 0x00040000;
        const CPF_InstancedReference = 0x00080000;
        const CPF_DuplicateTransient = 0x00200000;
        const CPF_SaveGame = 0x01000000;
        const CPF_NoClear = 0x02000000;
        const CPF_ReferenceParm = 0x08000000;
        const CPF_BlueprintAssignable = 0x10000000;
        const CPF_Deprecated = 0x20000000;
        const CPF_IsPlainOldData = 0x40000000;
        const CPF_RepSkip = 0x80000000;
        const CPF_RepNotify = 0x100000000;
        const CPF_Interp = 0x200000000;
        const CPF_NonTransactional = 0x400000000;
        const CPF_EditorOnly = 0x800000000;
        const CPF_NoDestructor = 0x1000000000;
        const CPF_AutoWeak = 0x4000000000;
        const CPF_ContainsInstancedReference = 0x8000000000;
        const CPF_AssetRegistrySearchable = 0x10000000000;
        const CPF_SimpleDisplay = 0x20000000000;
        const CPF_AdvancedDisplay = 0x40000000000;
        const CPF_Protected = 0x80000000000;
        const CPF_BlueprintCallable = 0x100000000000;
        const CPF_BlueprintAuthorityOnly = 0x200000000000;
        const CPF_TextExportTransient = 0x400000000000;
        const CPF_NonPIEDuplicateTransient = 0x800000000000;
        const CPF_ExposeOnSpawn = 0x1000000000000;
        const CPF_PersistentInstance = 0x2000000000000;
        const CPF_UObjectWrapper = 0x4000000000000;
        const CPF_HasGetValueTypeHash = 0x8000000000000;
        const CPF_NativeAccessSpecifierPublic = 0x10000000000000;
        const CPF_NativeAccessSpecifierProtected = 0x20000000000000;
        const CPF_NativeAccessSpecifierPrivate = 0x40000000000000;
        const CPF_SkipSerialization = 0x80000000000000;
    }

    #[derive(Debug, Clone, Copy)]
    #[repr(C)]
    pub struct EInternalObjectFlags: u32 {
        const None = 0x0;
        const ReachableInCluster = 0x800000;
        const ClusterRoot = 0x1000000;
        const Native = 0x2000000;
        const Async = 0x4000000;
        const AsyncLoading = 0x8000000;
        const Unreachable = 0x10000000;
        const PendingKill = 0x20000000;
        const RootSet = 0x40000000;
        const GarbageCollectionKeepFlags = 0xe000000;
        const AllFlags = 0x7f800000;
    }

    #[derive(Debug, Clone, Copy)]
    #[repr(C)]
    pub struct EStructFlags: i32 {
        const STRUCT_NoFlags = 0x0000;
        const STRUCT_Native = 0x0001;
        const STRUCT_IdenticalNative = 0x0002;
        const STRUCT_HasInstancedReference = 0x0004;
        const STRUCT_NoExport = 0x0008;
        const STRUCT_Atomic = 0x0010;
        const STRUCT_Immutable = 0x0020;
        const STRUCT_AddStructReferencedObjects = 0x0040;
        const STRUCT_RequiredAPI = 0x0200;
        const STRUCT_NetSerializeNative = 0x0400;
        const STRUCT_SerializeNative = 0x0800;
        const STRUCT_CopyNative = 0x1000;
        const STRUCT_IsPlainOldData = 0x2000;
        const STRUCT_NoDestructor = 0x4000;
        const STRUCT_ZeroConstructor = 0x8000;
        const STRUCT_ExportTextItemNative = 0x00010000;
        const STRUCT_ImportTextItemNative = 0x00020000;
        const STRUCT_PostSerializeNative = 0x00040000;
        const STRUCT_SerializeFromMismatchedTag = 0x00080000;
        const STRUCT_NetDeltaSerializeNative = 0x00100000;
        const STRUCT_PostScriptConstruct = 0x00200000;
        const STRUCT_NetSharedSerialization = 0x00400000;
        const STRUCT_Trashed = 0x00800000;
        const STRUCT_Inherit = 0x0014;
        const STRUCT_ComputedFlags = 0x007ffc42;
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct FNameEntryId {
    pub Value: u32,
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct FName {
    pub ComparisonIndex: FNameEntryId,
    pub Number: u32,
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct UObject {
    pub vtable: ExternalPtr<usize>,
    pub ObjectFlags: EObjectFlags,
    pub InternalIndex: i32,
    pub ClassPrivate: ExternalPtr<UClass>,
    pub NamePrivate: FName,
    pub OuterPrivate: ExternalPtr<UObject>,
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct UField {
    /* offset 0x000 */ pub uobject: UObject,
    /* offset 0x028 */ pub Next: ExternalPtr<UField>,
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct FStructBaseChain {
    /* offset 0x000 */ pub StructBaseChainArray: ExternalPtr<ExternalPtr<FStructBaseChain>>,
    /* offset 0x008 */ pub NumStructBasesInChainMinusOne: i32,
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct UStruct {
    pub ufield: UField,
    pub base_chain: FStructBaseChain,
    pub SuperStruct: ExternalPtr<UStruct>,
    pub Children: ExternalPtr<UField>,
    pub ChildProperties: ExternalPtr<FField>,
    pub PropertiesSize: i32,
    pub MinAlignment: i32,
    pub Script: TArray<u8>,
    pub PropertyLink: ExternalPtr<FProperty>,
    pub RefLink: ExternalPtr<FProperty>,
    pub DestructorLink: ExternalPtr<FProperty>,
    pub PostConstructLink: ExternalPtr<FProperty>,
    pub ScriptAndPropertyObjectReferences: TArray<ExternalPtr<UObject>>,
    pub UnresolvedScriptProperties: ExternalPtr<()>, // *const TArray<TTuple<TFieldPath<FField>,int>,TSizedDefaultAllocator<32> >,
    pub UnversionedSchema: ExternalPtr<()>,          // *const FUnversionedStructSchema
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct UClass {
    /* offset 0x000 */
    pub ustruct: UStruct,
    /* offset 0x0b0 */
    pub ClassConstructor: ExternalPtr<()>, //extern "system" fn(*const [const] FObjectInitializer),
    /* offset 0x0b8 */
    pub ClassVTableHelperCtorCaller: ExternalPtr<()>, //extern "system" fn(*const FVTableHelper) -> *const UObject,
    /* offset 0x0c0 */
    pub ClassAddReferencedObjects: ExternalPtr<()>, //extern "system" fn(*const UObject, *const FReferenceCollector),
    /* offset 0x0c8 */
    pub ClassUnique_bCooked: u32, /* TODO: figure out how to name it */
    /* offset 0x0cc */ pub ClassFlags: EClassFlags,
    /* offset 0x0d0 */ pub ClassCastFlags: EClassCastFlags,
    /* offset 0x0d8 */ pub ClassWithin: *const UClass,
    /* offset 0x0e0 */ pub ClassGeneratedBy: *const UObject,
    /* offset 0x0e8 */ pub ClassConfigName: FName,
    /* offset 0x0f0 */
    pub ClassReps: TArray<()>, //TArray<FRepRecord,TSizedDefaultAllocator<32> >,
    /* offset 0x100 */ pub NetFields: TArray<ExternalPtr<UField>>,
    /* offset 0x110 */ pub FirstOwnedClassRep: i32,
    /* offset 0x118 */ pub ClassDefaultObject: ExternalPtr<UObject>,
    /* offset 0x120 */ pub SparseClassData: ExternalPtr<()>,
    /* offset 0x128 */
    pub SparseClassDataStruct: ExternalPtr<()>, // *const UScriptStruct
    /* offset 0x130 */
    pub FuncMap: TMap<FName, ExternalPtr<UObject>>, // *const UFunction
    /* offset 0x180 */
    pub SuperFuncMap: TMap<FName, ExternalPtr<UObject>>, //*const UFunction
    /* offset 0x1d0 */ pub SuperFuncMapLock: u64, //FWindowsRWLock,
    /* offset 0x1d8 */
    pub Interfaces: TArray<()>, //TArray<FImplementedInterface,TSizedDefaultAllocator<32> >,
    /* offset 0x1e8 */ pub ReferenceTokenStream: [u64; 2], // FGCReferenceTokenStream,
    /* offset 0x1f8 */
    pub ReferenceTokenStreamCritical: [u64; 5], // FWindowsCriticalSection,
    /* offset 0x220 */
    pub NativeFunctionLookupTable: TArray<()>, //TArray<FNativeFunctionLookup,TSizedDefaultAllocator<32> >,
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct UScriptStruct {
    pub ustruct: UStruct,
    pub StructFlags: EStructFlags,
    pub bPrepareCppStructOpsCompleted: bool,
    pub CppStructOps: ExternalPtr<()>, // UScriptStruct::ICppStructOps
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct UEnum {
    pub ufield: UField,
    pub CppType: FString,
    pub Names: TArray<TTuple<FName, i64>>,
    //CppForm: UEnum::ECppForm,
    //EnumFlags: EEnumFlags,
    //EnumDisplayNameFn: extern "system" fn(i32) -> FText,
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct FField {
    pub vtable: ExternalPtr<()>,
    pub ClassPrivate: ExternalPtr<FFieldClass>,
    pub Owner: FFieldVariant,
    pub Next: ExternalPtr<FField>,
    pub NamePrivate: FName,
    pub FlagsPrivate: EObjectFlags,
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct FFieldClass {
    pub Name: FName,
    pub Id: u64,
    pub CastFlags: EClassCastFlags,
    pub ClassFlags: EClassFlags,
    pub SuperClass: *const FFieldClass,
    pub DefaultObject: *const FField,
    pub ConstructFn: ExternalPtr<()>, //extern "system" fn(*const [const] FFieldVariant, *const [const] FName, EObjectFlags) -> *const FField,
    pub UnqiueNameIndexCounter: FThreadSafeCounter,
}

#[derive(Clone)]
#[repr(C)]
pub struct FFieldVariant {
    pub Container: FFieldVariant_FFieldObjectUnion,
    pub bIsUObject: bool,
}
impl std::fmt::Debug for FFieldVariant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut fmt = f.debug_struct("FFieldVariant");
        match self.bIsUObject {
            true => fmt.field("object", unsafe { &self.Container.object }),
            false => fmt.field("field", unsafe { &self.Container.field }),
        }
        .finish()
    }
}

#[derive(Clone, Copy)]
#[repr(C)]
pub union FFieldVariant_FFieldObjectUnion {
    pub field: ExternalPtr<FField>,
    pub object: ExternalPtr<UObject>,
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct FThreadSafeCounter {
    pub counter: i32,
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct FProperty {
    pub ffield: FField,
    pub ArrayDim: i32,
    pub ElementSize: i32,
    pub PropertyFlags: EPropertyFlags,
    pub RepIndex: u16,
    pub BlueprintReplicationCondition: u8, //TEnumAsByte<enum ELifetimeCondition>,
    pub Offset_Internal: i32,
    pub RepNotifyFunc: FName,
    pub PropertyLinkNext: ExternalPtr<FProperty>,
    pub NextRef: ExternalPtr<FProperty>,
    pub DestructorLinkNext: ExternalPtr<FProperty>,
    pub PostConstructLinkNext: ExternalPtr<FProperty>,
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct FBoolProperty {
    pub fproperty: FProperty,
    pub FieldSize: u8,
    pub ByteOffset: u8,
    pub ByteMask: u8,
    pub FieldMask: u8,
}
#[derive(Debug, Clone)]
#[repr(C)]
pub struct FObjectProperty {
    pub fproperty: FProperty,
    pub property_class: ExternalPtr<UClass>,
}
#[derive(Debug, Clone)]
#[repr(C)]
pub struct FSoftObjectProperty {
    pub fproperty: FProperty,
    pub property_class: ExternalPtr<UClass>,
}
#[derive(Debug, Clone)]
#[repr(C)]
pub struct FWeakObjectProperty {
    pub fproperty: FProperty,
    pub property_class: ExternalPtr<UClass>,
}
#[derive(Debug, Clone)]
#[repr(C)]
pub struct FLazyObjectProperty {
    pub fproperty: FProperty,
    pub property_class: ExternalPtr<UClass>,
}
#[derive(Debug, Clone)]
#[repr(C)]
pub struct FInterfaceProperty {
    pub fproperty: FProperty,
    pub interface_class: ExternalPtr<UClass>,
}
#[derive(Debug, Clone)]
#[repr(C)]
pub struct FArrayProperty {
    pub fproperty: FProperty,
    pub inner: ExternalPtr<FProperty>,
    pub array_flags: u32, //EArrayPropertyFlags,
}
#[derive(Debug, Clone)]
#[repr(C)]
pub struct FStructProperty {
    pub fproperty: FProperty,
    pub struct_: ExternalPtr<UScriptStruct>,
}
#[derive(Debug, Clone)]
#[repr(C)]
pub struct FMapProperty {
    pub fproperty: FProperty,
    pub key_prop: ExternalPtr<FProperty>,
    pub value_prop: ExternalPtr<FProperty>,
    //pub map_layout: FScriptMapLayout,
    //pub map_flags: EMapPropertyFlags,
}
#[derive(Debug, Clone)]
#[repr(C)]
pub struct FSetProperty {
    pub fproperty: FProperty,
    pub element_prop: ExternalPtr<FProperty>,
    //pub set_layout: FScriptSetLayout,
}
#[derive(Debug, Clone)]
#[repr(C)]
pub struct FEnumProperty {
    pub fproperty: FProperty,
    pub underlying_prop: ExternalPtr<FProperty>, // FNumericProperty
    pub enum_: ExternalPtr<UEnum>,               // FNumericProperty
                                                 //pub set_layout: FScriptSetLayout,
}
#[derive(Debug, Clone)]
#[repr(C)]
pub struct FByteProperty {
    pub fproperty: FProperty,
    pub enum_: ExternalPtr<UEnum>,
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct FUObjectItem {
    pub Object: ExternalPtr<UObject>,
    pub Flags: i32,
    pub ClusterRootIndex: i32,
    pub SerialNumber: i32,
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct FChunkedFixedUObjectArray {
    /* offset 0x0000 */ pub Objects: ExternalPtr<ExternalPtr<FUObjectItem>>,
    /* offset 0x0008 */ pub PreAllocatedObjects: ExternalPtr<FUObjectItem>,
    /* offset 0x0010 */ pub MaxElements: i32,
    /* offset 0x0014 */ pub NumElements: i32,
    /* offset 0x0018 */ pub MaxChunks: i32,
    /* offset 0x001c */ pub NumChunks: i32,
}
impl FChunkedFixedUObjectArray {
    pub fn read_item(&self, mem: &impl Mem, item: usize) -> Result<FUObjectItem> {
        let max_per_chunk = 64 * 1024;
        let chunk_index = item / max_per_chunk;

        self.Objects
            .offset(chunk_index)
            .read(mem)?
            .offset(item % max_per_chunk)
            .read(mem)
    }
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct FUObjectArray {
    /* offset 0x0000 */ pub ObjFirstGCIndex: i32,
    /* offset 0x0004 */ pub ObjLastNonGCIndex: i32,
    /* offset 0x0008 */ pub MaxObjectsNotConsideredByGC: i32,
    /* offset 0x000c */ pub OpenForDisregardForGC: bool,
    /* offset 0x0010 */
    pub ObjObjects: FChunkedFixedUObjectArray,
    /* offset 0x0030 */ // FWindowsCriticalSection ObjObjectsCritical;
    /* offset 0x0058 */ // TLockFreePointerListUnordered<int,64> ObjAvailableList;
    /* offset 0x00e0 */ // TArray<FUObjectArray::FUObjectCreateListener *,TSizedDefaultAllocator<32> > UObjectCreateListeners;
    /* offset 0x00f0 */ // TArray<FUObjectArray::FUObjectDeleteListener *,TSizedDefaultAllocator<32> > UObjectDeleteListeners;
    /* offset 0x0100 */ // FWindowsCriticalSection UObjectDeleteListenersCritical;
    /* offset 0x0128 */ // FThreadSafeCounter MasterSerialNumber;
}

#[derive(Debug, Clone)]
#[repr(C)]
struct FNameBlock {
    data: [u8; 0x1_0000],
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct FNameEntryAllocator {
    /* offset 0x0000 */ pub Lock: *const (), //FWindowsRWLock Lock;
    /* offset 0x0008 */ pub CurrentBlock: u32,
    /* offset 0x000c */ pub CurrentByteCursor: u32,
    /* offset 0x0010 */ pub Blocks: [ExternalPtr<[u8; 0x2_0000]>; 0x1_0000],
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct FNamePool {
    /* offset 0x0000 */
    pub Entries: FNameEntryAllocator,
    /* offset 0x10040 */ // FNamePoolShard<1>[1024] ComparisonShards;
    /* offset 0x10440 */ // FNameEntryId[2808] ENameToEntry;
    /* offset 0x10f38 */ // uint32_t LargestEnameUnstableId;
    /* offset 0x10f40 */ // TMap<FNameEntryId,enum EName,TInlineSetAllocator<512,TSetAllocator<TSparseArrayAllocator<TSizedDefaultAllocator<32>,TSizedDefaultAllocator<32> >,TSizedDefaultAllocator<32>,2,8,4>,2,4>,TDefaultMapHashableKeyFuncs<FNameEntryId,enum EName,0> > EntryToEName;
}

#[derive(Debug, Clone, Copy)]
pub struct PtrFNamePool(pub usize);
impl PtrFNamePool {
    pub fn read(self, mem: &impl Mem, name: FName) -> Result<String> {
        let blocks = ExternalPtr::<ExternalPtr<u8>>::new(self.0 + 0x10);

        let block_index = (name.ComparisonIndex.Value >> 16) as usize;
        let offset = (name.ComparisonIndex.Value & 0xffff) as usize * 2;

        let block = blocks.offset(block_index).read(mem)?;

        let header_bytes: [u8; 2] = block.offset(offset).read_vec(mem, 2)?.try_into().unwrap();
        let header: u16 = unsafe { std::mem::transmute_copy(&header_bytes) };

        // TODO depends on case preserving
        let len = (header >> 6) as usize;
        let is_wide = header & 1 != 0;

        Ok(if is_wide {
            String::from_utf16(
                &block
                    .offset(offset + 2)
                    .read_vec(mem, len * 2)?
                    .chunks(2)
                    .map(|chunk| u16::from_le_bytes(chunk.try_into().unwrap()))
                    .collect::<Vec<_>>(),
            )?
        } else {
            String::from_utf8(block.offset(offset + 2).read_vec(mem, len)?)?
        })
    }
}

pub trait Mem {
    fn read_buf(&self, address: usize, buf: &mut [u8]) -> Result<()>;
    fn read<T>(&self, address: usize) -> Result<T> {
        let mut buf = MaybeUninit::<T>::uninit();
        let mut bytes = unsafe {
            std::slice::from_raw_parts_mut(
                buf.as_mut_ptr().cast::<u8>() as _,
                std::mem::size_of::<T>(),
            )
        };
        self.read_buf(address, &mut bytes)?;
        Ok(unsafe { std::mem::transmute_copy(&buf) })
    }

    fn read_vec<T: Sized>(&self, address: usize, count: usize) -> Result<Vec<T>> {
        let size = std::mem::size_of::<T>();

        let mut buf = vec![0u8; count * size];
        self.read_buf(address, &mut buf)?;

        let length = buf.len() / size;
        let capacity = buf.capacity() / size;
        let ptr = buf.as_mut_ptr() as *mut T;

        std::mem::forget(buf);

        Ok(unsafe { Vec::from_raw_parts(ptr, length, capacity) })
    }
}
const PAGE_SIZE: usize = 0x1000;
pub struct MemCache<M> {
    inner: M,
    pages: Arc<Mutex<HashMap<usize, Vec<u8>>>>,
}
impl<M: Mem> MemCache<M> {
    pub fn wrap(inner: M) -> Self {
        Self {
            inner,
            pages: Default::default(),
        }
    }
}
impl<M: Mem> Mem for MemCache<M> {
    fn read_buf(&self, address: usize, buf: &mut [u8]) -> Result<()> {
        let mut remaining = buf.len();
        let mut cur = 0;

        let mut lock = self.pages.lock().unwrap();

        while remaining > 0 {
            let page_start = (address + cur) & !(PAGE_SIZE - 1);
            let page_offset = (address + cur) - page_start;
            let to_copy = remaining.min(PAGE_SIZE - page_offset);

            let buf_region = &mut buf[cur..cur + to_copy];
            let page_range = page_offset..page_offset + to_copy;
            if let Some(page) = lock.get(&page_start) {
                buf_region.copy_from_slice(&page[page_range]);
            } else {
                let mut page = vec![0; PAGE_SIZE];
                self.inner.read_buf(page_start, &mut page)?;
                buf_region.copy_from_slice(&page[page_range]);
                lock.insert(page_start, page);
            }

            remaining -= to_copy;
            cur += to_copy;
        }

        Ok(())
    }
}
