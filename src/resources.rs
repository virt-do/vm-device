// Copyright (C) 2019 Alibaba Cloud. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Structs to manage device resources.
//!
//! The high level flow of resource management among the VMM, the device manager, and the device
//! is as below:
//! 1) the VMM creates a new device object.
//! 2) the VMM asks the new device object for its resource constraints.
//! 3) the VMM allocates resources for the device object according to resource constraints.
//! 4) the VMM passes the allocated resources to the device object.
//! 5) the VMM registers the new device onto corresponding device managers according the allocated
//!    resources.

use std::{u16, u32, u64};

/// Enumeration describing a device's resource constraints.
pub enum ResourceConstraint {
    /// Constraint for an IO Port address range.
    PioAddress {
        /// Allocating resource within the range [`min`, `max`] if specified.
        range: Option<(u16, u16)>,
        /// Alignment for the allocated address.
        align: u16,
        /// Size for the allocated address range.
        size: u16,
    },
    /// Constraint for a Memory Mapped IO address range.
    MmioAddress {
        /// Allocating resource within the range [`min`, `max`] if specified.
        range: Option<(u64, u64)>,
        /// Alignment for the allocated address.
        align: u64,
        /// Size for the allocated address range.
        size: u64,
    },
    /// Constraint for a legacy IRQ.
    LegacyIrq {
        /// Reserving the pre-allocated IRQ if it's specified.
        irq: Option<u32>,
    },
    /// Constraint for PCI MSI IRQs.
    PciMsiIrq {
        /// Number of Irqs to allocate.
        size: u32,
    },
    /// Constraint for PCI MSIx IRQs.
    PciMsixIrq {
        /// Number of Irqs to allocate.
        size: u32,
    },
    /// Constraint for generic IRQs.
    GenericIrq {
        /// Number of Irqs to allocate.
        size: u32,
    },
    /// Constraint for KVM mem_slot indexes to map memory into the guest.
    KvmMemSlot {
        /// Allocating kvm memory slots starting from the index `slot` if specified.
        slot: Option<u32>,
        /// Number of slots to allocate.
        size: u32,
    },
}

impl ResourceConstraint {
    /// Create a new PIO address constraint object with default configuration.
    pub fn new_pio(size: u16) -> Self {
        ResourceConstraint::PioAddress {
            range: None,
            align: 0x1,
            size,
        }
    }

    /// Create a new PIO address constraint object.
    pub fn pio_with_constraints(size: u16, range: Option<(u16, u16)>, align: u16) -> Self {
        ResourceConstraint::PioAddress { range, align, size }
    }

    /// Create a new MMIO address constraint object with default configuration.
    pub fn new_mmio(size: u64) -> Self {
        ResourceConstraint::MmioAddress {
            range: None,
            align: 0x1000,
            size,
        }
    }

    /// Create a new MMIO address constraint object.
    pub fn mmio_with_constraints(size: u64, range: Option<(u64, u64)>, align: u64) -> Self {
        ResourceConstraint::MmioAddress { range, align, size }
    }

    /// Create a new legacy IRQ constraint object.
    ///
    /// Allocating the pre-allocated legacy Irq `irq` if specified.
    pub fn new_legacy_irq(irq: Option<u32>) -> Self {
        ResourceConstraint::LegacyIrq { irq }
    }

    /// Create a new KVM memory slot constraint object.
    ///
    /// Allocating kvm memory slots starting from the index `slot` if specified.
    pub fn new_kvm_mem_slot(size: u32, slot: Option<u32>) -> Self {
        ResourceConstraint::KvmMemSlot { slot, size }
    }
}

/// Type of Message Signaled Interrupt
#[derive(Clone, Copy, Eq, PartialEq)]
pub enum MsiIrqType {
    /// PCI MSI IRQ numbers.
    PciMsi,
    /// PCI MSIx IRQ numbers.
    PciMsix,
    /// Generic MSI IRQ numbers.
    GenericMsi,
}

/// Enumeration for device resources.
#[allow(missing_docs)]
#[derive(Clone)]
pub enum Resource {
    /// Memory Mapped IO address range.
    GuestAddressRange { base: u64, size: u64 },
    /// Legacy IRQ number.
    LegacyIrq(u32),
    /// Message Signaled Interrupt
    MsiIrq {
        ty: MsiIrqType,
        base: u32,
        size: u32,
    },
    /// Network Interface Card MAC address.
    MacAddresss(String),
    /// KVM memslot index.
    KvmMemSlot(u32),
}

/// Newtype to store a set of device resources.
#[derive(Default, Clone)]
pub struct DeviceResources(Vec<Resource>);

impl DeviceResources {
    /// Create a container object to store device resources.
    pub fn new() -> Self {
        DeviceResources(Vec::new())
    }

    /// Append a device resource to the container object.
    pub fn append(&mut self, entry: Resource) {
        self.0.push(entry);
    }

    /// Get the Memory Mapped IO address resources.
    pub fn get_mmio_address_ranges(&self) -> Vec<(u64, u64)> {
        let mut vec = Vec::new();
        for entry in self.0.iter().as_ref() {
            if let Resource::GuestAddressRange { base, size } = entry {
                vec.push((*base, *size));
            }
        }
        vec
    }

    /// Get the first legacy interrupt number(IRQ).
    pub fn get_legacy_irq(&self) -> Option<u32> {
        for entry in self.0.iter().as_ref() {
            if let Resource::LegacyIrq(base) = entry {
                return Some(*base);
            }
        }
        None
    }

    /// Get information about the first PCI MSI interrupt resource.
    pub fn get_pci_msi_irqs(&self) -> Option<(u32, u32)> {
        self.get_msi_irqs(MsiIrqType::PciMsi)
    }

    /// Get information about the first PCI MSIx interrupt resource.
    pub fn get_pci_msix_irqs(&self) -> Option<(u32, u32)> {
        self.get_msi_irqs(MsiIrqType::PciMsix)
    }

    /// Get information about the first Generic MSI interrupt resource.
    pub fn get_generic_msi_irqs(&self) -> Option<(u32, u32)> {
        self.get_msi_irqs(MsiIrqType::GenericMsi)
    }

    fn get_msi_irqs(&self, ty: MsiIrqType) -> Option<(u32, u32)> {
        for entry in self.0.iter().as_ref() {
            if let Resource::MsiIrq {
                ty: msi_type,
                base,
                size,
            } = entry
            {
                if ty == *msi_type {
                    return Some((*base, *size));
                }
            }
        }
        None
    }

    /// Get the KVM memory slots to map memory into the guest.
    pub fn get_kvm_mem_slots(&self) -> Vec<u32> {
        let mut vec = Vec::new();
        for entry in self.0.iter().as_ref() {
            if let Resource::KvmMemSlot(index) = entry {
                vec.push(*index);
            }
        }
        vec
    }

    /// Get the first resource information for NIC MAC address.
    pub fn get_mac_address(&self) -> Option<String> {
        for entry in self.0.iter().as_ref() {
            if let Resource::MacAddresss(addr) = entry {
                return Some(addr.clone());
            }
        }
        None
    }

    /// Get immutable reference to all the resources.
    pub fn get_all_resources(&self) -> &[Resource] {
        &self.0
    }
}
