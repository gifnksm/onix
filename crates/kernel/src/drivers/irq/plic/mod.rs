use alloc::{boxed::Box, collections::btree_map::BTreeMap, string::String, sync::Arc, vec::Vec};
use core::{ops::Range, ptr};

use devicetree::parsed::Devicetree;
use platform_cast::CastFrom as _;
use snafu::{ResultExt as _, Snafu};
use snafu_utils::Location;
use spin::Once;
use sv39::MapPageFlags;

use self::parse::ParseDevicetreeError;
use crate::{
    cpu::Cpuid,
    interrupt,
    memory::kernel_space::{self, IdentityMapError},
    sync::spinlock::SpinMutex,
};

mod parse;

static PLIC_DEVICES: Once<Vec<Arc<Plic>>> = Once::new();

#[derive(Debug, Snafu)]
pub enum PlicInitError {
    #[snafu(display("failed to parse devicetree"))]
    #[snafu(provide(ref, priority, Location => location))]
    ParseDevicetree {
        #[snafu(source)]
        source: Box<ParseDevicetreeError>,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("identity map error"))]
    #[snafu(provide(ref, priority, Location => location))]
    MapPage {
        #[snafu(source)]
        source: IdentityMapError,
        #[snafu(implicit)]
        location: Location,
    },
}

pub fn init(dtree: &Devicetree) -> Result<(), Box<PlicInitError>> {
    let plic_devices = parse::parse(dtree).context(ParseDevicetreeSnafu)?;
    for plic in &plic_devices {
        let mmio = plic.mmio.lock();
        kernel_space::identity_map_range(mmio.range(), MapPageFlags::RW).context(MapPageSnafu)?;
    }
    PLIC_DEVICES.call_once(|| plic_devices);
    Ok(())
}

pub fn find_plic_context_for_cpu(cpuid: Cpuid) -> Option<(Arc<Plic>, PlicContext)> {
    let plic_devices = PLIC_DEVICES.get()?;
    for plic in plic_devices {
        if let Some(context) = plic.find_context_for_cpu(cpuid) {
            return Some((Arc::clone(plic), context));
        }
    }
    None
}

pub fn find_plic_by_dtree_path(path: &str) -> Option<Arc<Plic>> {
    PLIC_DEVICES
        .get()?
        .iter()
        .find(|plic| plic.path == path)
        .cloned()
}

pub type PlicCallback = Arc<dyn Fn(PlicContext) + Send + Sync>;

#[derive(custom_debug_derive::Debug)]
pub struct Plic {
    path: String,
    mmio: SpinMutex<PlicMmio>,
    context_map: BTreeMap<Cpuid, PlicContext>,
    #[debug(skip)]
    callbacks: SpinMutex<BTreeMap<PlicSource, PlicCallback>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PlicContext {
    id: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PlicSource {
    id: usize,
}

#[derive(custom_debug_derive::Debug)]
struct PlicMmio {
    #[debug(format = "{:#x}")]
    base_addr: usize,
    #[debug(format = "{:#x}")]
    size: usize,
    ndev: usize,
}

impl Plic {
    pub fn find_context_for_cpu(&self, cpuid: Cpuid) -> Option<PlicContext> {
        self.context_map.get(&cpuid).copied()
    }

    pub fn handle_interrupt(&self, context: PlicContext) -> bool {
        let Some(source) = self.mmio.lock().claim(context) else {
            return false;
        };
        let callback = self.callbacks.lock().get(&source).map(Arc::clone);
        if let Some(callback) = callback {
            callback(context);
        } else {
            warn!("no handler for PLIC source {source:?}");
        }
        self.mmio.lock().complete(source, context);
        true
    }

    pub fn translate_interrupt_specifier(&self, specifier: &[u32]) -> PlicSource {
        assert_eq!(specifier.len(), 1, "invalid interrupt specifier");
        let id = usize::cast_from(specifier[0]);
        let source = PlicSource { id };
        assert!(
            self.mmio.lock().is_valid_source(source),
            "invalid interrupt source id"
        );
        source
    }

    pub fn register_callback(&self, source: PlicSource, callback: PlicCallback) {
        let mut callbacks = self.callbacks.lock();
        assert!(
            !callbacks.contains_key(&source),
            "callback already registered for source {source:?}"
        );
        callbacks.insert(source, callback);
    }

    pub fn set_priority(&self, source: PlicSource, priority: u32) {
        self.mmio.lock().set_priority(source, priority);
    }

    pub fn set_priority_threshold(&self, context: PlicContext, threshold: u32) {
        self.mmio.lock().set_priority_threshold(context, threshold);
    }

    pub fn enable_interrupt(&self, source: PlicSource, context: PlicContext) {
        self.mmio.lock().enable_interrupt(source, context);
    }
}

impl PlicMmio {
    fn is_valid_source(&self, source: PlicSource) -> bool {
        (1..=self.ndev).contains(&source.id)
    }

    fn range(&self) -> Range<usize> {
        self.base_addr..self.base_addr + self.size
    }

    fn priority_addr(&self, source: PlicSource) -> *mut u32 {
        assert!(self.is_valid_source(source));
        ptr::with_exposed_provenance_mut(self.base_addr + source.id * 4)
    }

    fn pending_addr_bit(&self, source: PlicSource) -> (*mut u32, usize) {
        assert!(self.is_valid_source(source));
        let base = self.base_addr + 0x00_1000;
        let bit = source.id % 32;
        let word = source.id / 32;
        (ptr::with_exposed_provenance_mut(base + word * 4), bit)
    }

    fn enable_addr_bit(&self, source: PlicSource, context: PlicContext) -> (*mut u32, usize) {
        assert!(self.is_valid_source(source));
        let base = self.base_addr + 0x00_2000 + 0x80 * context.id;
        let bit = source.id % 32;
        let word = source.id / 32;
        (ptr::with_exposed_provenance_mut(base + word * 4), bit)
    }

    fn priority_threshold_addr(&self, context: PlicContext) -> *mut u32 {
        ptr::with_exposed_provenance_mut(self.base_addr + 0x20_0000 + 0x1000 * context.id)
    }

    fn claim_addr(&self, context: PlicContext) -> *mut u32 {
        ptr::with_exposed_provenance_mut(self.base_addr + 0x20_0000 + 0x1000 * context.id + 0x4)
    }

    fn set_priority(&mut self, source: PlicSource, priority: u32) {
        assert!(!interrupt::is_enabled());
        assert!(self.is_valid_source(source));
        unsafe {
            self.priority_addr(source).write_volatile(priority);
        }
    }

    #[expect(dead_code)]
    #[expect(clippy::needless_pass_by_ref_mut)]
    fn is_pending(&mut self, source: PlicSource) -> bool {
        assert!(!interrupt::is_enabled());
        assert!(self.is_valid_source(source));
        let (addr, bit) = self.pending_addr_bit(source);
        unsafe { (addr.read_volatile() & (1 << bit)) != 0 }
    }

    #[expect(clippy::needless_pass_by_ref_mut)]
    fn enable_interrupt(&mut self, source: PlicSource, context: PlicContext) {
        assert!(!interrupt::is_enabled());
        assert!(self.is_valid_source(source));
        let (addr, bit) = self.enable_addr_bit(source, context);
        unsafe {
            let value = addr.read_volatile();
            addr.write_volatile(value | (1 << bit));
        }
    }

    #[expect(dead_code)]
    #[expect(clippy::needless_pass_by_ref_mut)]
    fn disable_interrupt(&mut self, source: PlicSource, context: PlicContext) {
        assert!(!interrupt::is_enabled());
        assert!(self.is_valid_source(source));
        let (addr, bit) = self.enable_addr_bit(source, context);
        unsafe {
            let value = addr.read_volatile();
            addr.write_volatile(value & !(1 << bit));
        }
    }

    #[expect(clippy::needless_pass_by_ref_mut)]
    fn set_priority_threshold(&mut self, context: PlicContext, threshold: u32) {
        assert!(!interrupt::is_enabled());
        let addr = self.priority_threshold_addr(context);
        unsafe {
            addr.write_volatile(threshold);
        }
    }

    fn claim(&mut self, context: PlicContext) -> Option<PlicSource> {
        assert!(!interrupt::is_enabled());
        let source = PlicSource {
            id: usize::cast_from(unsafe { self.claim_addr(context).read_volatile() }),
        };
        self.is_valid_source(source).then_some(source)
    }

    fn complete(&mut self, source: PlicSource, context: PlicContext) {
        assert!(!interrupt::is_enabled());
        assert!(self.is_valid_source(source));
        unsafe {
            self.claim_addr(context)
                .write_volatile(source.id.try_into().unwrap());
        }
    }
}
