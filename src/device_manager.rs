// Copyright © 2019 Intel Corporation. All Rights Reserved.
// SPDX-License-Identifier: (Apache-2.0 OR BSD-3-Clause)

//! System level device management.
//!
//! [`IoManager`] is responsible for managing
//! all devices of virtual machine, registering IO resources callback,
//! deregistering devices and helping VM IO exit handling.
//! It defines two buses, one for PIO and one for MMIO, and provides default
//! implementations of [`PioManager`] and [`MmioManager`].
//!
//! The VMM must first allocate unique resources (such as bus ranges), and then
//! call into the vm-device interface to register the devices with their
//! corresponding resources.
//!
//! # Examples
//!
//! Registering a new device can be done using the register methods of [`PioManager`]
//! and [`MmioManager`] with an appropriate bus range
//! ([`PioRange`](../bus/type.PioRange.html) or [`BusRange`](../bus/type.BusRange.html)).
//! ```
//! # use std::sync::Arc;
//! # use vm_device::bus::{PioAddress, PioAddressOffset, PioRange};
//! # use vm_device::bus::{GuestAddress, GuestAddressOffset, BusRange};
//! # use vm_device::device_manager::{IoManager, PioManager, MmioManager};
//! # use vm_device::{DevicePio, VirtioMmioDevice};
//! struct NoopDevice {}
//!
//! impl DevicePio for NoopDevice {
//!     fn pio_read(&self, base: PioAddress, offset: PioAddressOffset, data: &mut [u8]) {}
//!     fn pio_write(&self, base: PioAddress, offset: PioAddressOffset, data: &[u8]) {}
//! }
//!
//! impl VirtioMmioDevice for NoopDevice {
//!     fn mmio_read(&self, base: GuestAddress, offset: GuestAddressOffset, data: &mut [u8]) {}
//!     fn mmio_write(&self, base: GuestAddress, offset: GuestAddressOffset, data: &[u8]) {}
//! }
//!
//! // IoManager implements both PioManager and MmioManager.
//! let mut manager = IoManager::new();
//!
//! // Register the device on the PIO bus.
//! let pio_range = PioRange::new(PioAddress(0), 10).unwrap();
//! manager
//!     .register_pio(pio_range, Arc::new(NoopDevice {}))
//!     .unwrap();
//!
//! // Register the device on the MMIO bus.
//! let mmio_range = BusRange::new(GuestAddress(0), 10).unwrap();
//! manager
//!     .register_mmio(mmio_range, Arc::new(NoopDevice {}))
//!     .unwrap();
//!
//! // Dispatch I/O on the PIO bus.
//! manager.pio_write(PioAddress(0), &vec![b'o', b'k']).unwrap();
//!
//! // Dispatch I/O on the MMIO bus.
//! manager
//!     .mmio_write(GuestAddress(0), &vec![b'o', b'k'])
//!     .unwrap();
//! ```
//!
//! An alternative way would be to use [`resources`](../resources/index.html) and the
//! resources registration methods of [`IoManager`]:
//! * [`register_pio_resources`](struct.IoManager.html#method.register_pio_resources)
//! * [`register_mmio_resources`](struct.IoManager.html#method.register_mmio_resources)
//! * or generic [`register_resources`](struct.IoManager.html#method.register_resources)
//! ```
//! # use std::sync::Arc;
//! # use vm_device::bus::{PioAddress, PioAddressOffset, PioRange};
//! # use vm_device::bus::{GuestAddress, GuestAddressOffset, BusRange};
//! # use vm_device::device_manager::{IoManager, PioManager, MmioManager};
//! # use vm_device::{DevicePio, VirtioMmioDevice};
//! # use vm_device::resources::Resource;
//! # struct NoopDevice {}
//! #
//! # impl DevicePio for NoopDevice {
//! #    fn pio_read(&self, base: PioAddress, offset: PioAddressOffset, data: &mut [u8]) {}
//! #    fn pio_write(&self, base: PioAddress, offset: PioAddressOffset, data: &[u8]) {}
//! # }
//! #
//! # impl VirtioMmioDevice for NoopDevice {
//! #    fn mmio_read(&self, base: GuestAddress, offset: GuestAddressOffset, data: &mut [u8]) {}
//! #    fn mmio_write(&self, base: GuestAddress, offset: GuestAddressOffset, data: &[u8]) {}
//! # }
//! // Use the same NoopDevice defined above.
//!
//! let mut manager = IoManager::new();
//!
//! // Define a PIO address range resource.
//! let pio = Resource::PioAddressRange {
//!    base: 0,
//!    size: 10,
//! };
//!
//! // Define a MMIO address range resource.
//! let mmio = Resource::GuestAddressRange {
//!    base: 0,
//!    size: 10,
//! };
//!
//! // Register the PIO resource.
//! manager
//!     .register_pio_resources(Arc::new(NoopDevice {}), &vec![pio])
//!     .unwrap();
//!
//! // Register the MMIO resource.
//! manager
//!     .register_mmio_resources(Arc::new(NoopDevice {}), &vec![mmio])
//!     .unwrap();
//!
//! // Dispatching I/O is the same.
//! manager.pio_write(PioAddress(0), &vec![b'o', b'k']).unwrap();
//! manager.mmio_write(GuestAddress(0), &vec![b'o', b'k']).unwrap();
//! ```

use std::fmt::{Display, Formatter};
use std::result::Result;
use std::sync::Arc;

use vm_memory::GuestAddress;

use crate::bus::{self, Bus, BusManager, BusRange};
use crate::resources::Resource;
use crate::{VirtioMmioDevice, VirtioMmioOffset};

/// Error type for [IoManager] usage.
#[derive(Debug)]
pub enum Error {
    /// Error during bus operation.
    Bus(bus::Error),
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Bus(_) => write!(f, "device_manager: bus error"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Bus(e) => Some(e),
        }
    }
}

/// Represents an object that provides MMIO manager operations.
pub trait MmioManager {
    /// Type of the objects that can be registered with this `MmioManager`.
    type D: VirtioMmioDevice;

    /// Return a reference to the device registered at `addr`, together with the associated
    /// range, if available.
    fn mmio_device(&self, addr: GuestAddress) -> Option<(&BusRange, &Self::D)>;

    /// Dispatch a read operation to the device registered at `addr`.
    fn mmio_read(&self, addr: GuestAddress, data: &mut [u8]) -> Result<(), bus::Error>;

    /// Dispatch a write operation to the device registered at `addr`.
    fn mmio_write(&self, addr: GuestAddress, data: &[u8]) -> Result<(), bus::Error>;

    /// Register the provided device with the specified range.
    fn register_mmio(&mut self, range: BusRange, device: Self::D) -> Result<(), bus::Error>;

    /// Deregister the device currently registered at `addr` together with the
    /// associated range.
    fn deregister_mmio(&mut self, addr: GuestAddress) -> Option<(BusRange, Self::D)>;
}

// This automatically provides a `MmioManager` implementation for types that already implement
// `BusManager` if their inner associated type implements `VirtioMmioDevice` as well.
impl<T> MmioManager for T
where
    T: BusManager,
    T::D: VirtioMmioDevice,
{
    type D = <Self as BusManager>::D;

    fn mmio_device(&self, addr: GuestAddress) -> Option<(&BusRange, &Self::D)> {
        self.bus().device(addr)
    }

    fn mmio_read(&self, addr: GuestAddress, data: &mut [u8]) -> Result<(), bus::Error> {
        self.bus()
            .check_access(addr, data.len())
            .map(|(range, device)| {
                device.virtio_mmio_read(
                    range.base(),
                    VirtioMmioOffset::from(addr.0 - range.base().0),
                    data,
                )
            })
    }

    fn mmio_write(&self, addr: GuestAddress, data: &[u8]) -> Result<(), bus::Error> {
        self.bus()
            .check_access(addr, data.len())
            .map(|(range, device)| {
                device.virtio_mmio_write(
                    range.base(),
                    VirtioMmioOffset::from(addr.0 - range.base().0),
                    data,
                )
            })
    }

    fn register_mmio(&mut self, range: BusRange, device: Self::D) -> Result<(), bus::Error> {
        self.bus_mut().register(range, device)
    }

    fn deregister_mmio(&mut self, addr: GuestAddress) -> Option<(BusRange, Self::D)> {
        self.bus_mut().deregister(addr)
    }
}

/// System IO manager serving for all devices management and VM exit handling.
#[derive(Default)]
pub struct IoManager {
    // Range mapping for VM exit mmio operations.
    mmio_bus: Bus<Arc<dyn VirtioMmioDevice + Send + Sync>>,
}

// Enables the automatic implementation of `MmioManager` for `IoManager`.
impl BusManager for IoManager {
    type D = Arc<dyn VirtioMmioDevice + Send + Sync>;

    fn bus(&self) -> &Bus<Arc<dyn VirtioMmioDevice + Send + Sync>> {
        &self.mmio_bus
    }

    fn bus_mut(&mut self) -> &mut Bus<Arc<dyn VirtioMmioDevice + Send + Sync>> {
        &mut self.mmio_bus
    }
}

impl IoManager {
    /// Create an default IoManager with empty IO member.
    pub fn new() -> Self {
        IoManager::default()
    }

    /// Register a new MMIO device with its allocated resources.
    /// VMM is responsible for providing the allocated resources to virtual device.
    ///
    /// # Arguments
    ///
    /// * `device`: device instance object to be registered
    /// * `resources`: resources that this device owns, might include
    ///                port I/O and memory-mapped I/O ranges, irq number, etc.
    pub fn register_mmio_resources(
        &mut self,
        device: Arc<dyn VirtioMmioDevice + Send + Sync>,
        resources: &[Resource],
    ) -> Result<(), Error> {
        // Register and mark device resources
        // The resources addresses being registered are sucessfully allocated before.
        for res in resources.iter() {
            match *res {
                Resource::GuestAddressRange { base, size } => {
                    self.register_mmio(
                        BusRange::new(GuestAddress(base), size).unwrap(),
                        device.clone(),
                    )
                    .map_err(Error::Bus)?;
                }
                _ => continue,
            }
        }
        Ok(())
    }

    /// Register a new MMIO + PIO device with its allocated resources.
    /// VMM is responsible for providing the allocated resources to virtual device.
    ///
    /// # Arguments
    ///
    /// * `device`: device instance object to be registered
    /// * `resources`: resources that this device owns, might include
    ///                port I/O and memory-mapped I/O ranges, irq number, etc.
    pub fn register_resources<T: VirtioMmioDevice + 'static + Send + Sync>(
        &mut self,
        device: Arc<T>,
        resources: &[Resource],
    ) -> Result<(), Error> {
        self.register_mmio_resources(device.clone(), resources)
    }

    /// Deregister a device from `IoManager`, e.g. users specified removing.
    /// VMM pre-fetches the resources e.g. dev.get_assigned_resources()
    /// VMM is responsible for freeing the resources. Returns the number
    /// of deregistered devices.
    ///
    /// # Arguments
    ///
    /// * `resources`: resources that this device owns, might include
    ///                port I/O and memory-mapped I/O ranges, irq number, etc.
    pub fn deregister_resources(&mut self, resources: &[Resource]) -> usize {
        let mut count = 0;
        for res in resources.iter() {
            match *res {
                Resource::GuestAddressRange { base, .. } => {
                    if self.deregister_mmio(GuestAddress(base)).is_some() {
                        count += 1;
                    }
                }
                _ => continue,
            }
        }
        count
    }
}
