use std::ops::Deref;
use std::sync::{Arc, Mutex};

use vm_memory::GuestAddress;

pub mod bus;
pub mod device_manager;
pub mod resources;
pub mod virtio_mmio;

pub use virtio_mmio::VirtioMmioOffset;

pub trait VirtioMmioDevice {
    fn virtio_mmio_read(&self, base: GuestAddress, offset: VirtioMmioOffset, data: &mut [u8]);
    fn virtio_mmio_write(&self, base: GuestAddress, offset: VirtioMmioOffset, data: &[u8]);
}

pub trait MutVirtioMmioDevice {
    fn virtio_mmio_read(&mut self, base: GuestAddress, offset: VirtioMmioOffset, data: &mut [u8]);
    fn virtio_mmio_write(&mut self, base: GuestAddress, offset: VirtioMmioOffset, data: &[u8]);
}

impl<T: VirtioMmioDevice + ?Sized> VirtioMmioDevice for Arc<T> {
    fn virtio_mmio_read(&self, base: GuestAddress, offset: VirtioMmioOffset, data: &mut [u8]) {
        self.deref().virtio_mmio_read(base, offset, data);
    }

    fn virtio_mmio_write(&self, base: GuestAddress, offset: VirtioMmioOffset, data: &[u8]) {
        self.deref().virtio_mmio_write(base, offset, data);
    }
}

impl<T: MutVirtioMmioDevice + ?Sized> VirtioMmioDevice for Mutex<T> {
    fn virtio_mmio_read(&self, base: GuestAddress, offset: VirtioMmioOffset, data: &mut [u8]) {
        self.lock().unwrap().virtio_mmio_read(base, offset, data)
    }

    fn virtio_mmio_write(&self, base: GuestAddress, offset: VirtioMmioOffset, data: &[u8]) {
        self.lock().unwrap().virtio_mmio_write(base, offset, data)
    }
}
