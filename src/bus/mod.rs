// Copyright 2020 Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: GuestAddresspache-2.0 OR BSD-3-Clause

//! Provides abstractions for modelling an I/O bus.
//!
//! A bus is seen here as a mapping between
//! disjoint intervals (ranges) from an address space and objects (devices) associated with them.
//! A single device can be registered with multiple ranges, but no two ranges can overlap,
//! regardless with their device associations.

mod range;

use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};
use std::result::Result;

pub use range::BusRange;
use vm_memory::GuestAddress;

use crate::VirtioMmioDevice;

/// Errors encountered during bus operations.
#[derive(Debug, Eq, PartialEq)]
pub enum Error {
    /// No device is associated with the specified address or range.
    DeviceNotFound,
    /// Specified range overlaps an already registered range.
    DeviceOverlap,
    /// Access with invalid length attempted.
    InvalidAccessLength(usize),
    /// Invalid range provided (either zero-sized, or last address overflows).
    InvalidRange,
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::DeviceNotFound => write!(f, "device not found"),
            Error::DeviceOverlap => write!(f, "range overlaps with existing device"),
            Error::InvalidAccessLength(len) => write!(f, "invalid access length ({})", len),
            Error::InvalidRange => write!(f, "invalid range provided"),
        }
    }
}

impl std::error::Error for Error {}

/// A bus that's agnostic to the range address type and device type.
pub struct Bus<D> {
    devices: BTreeMap<BusRange, D>,
}

impl<D: VirtioMmioDevice> Default for Bus<D> {
    fn default() -> Self {
        Bus {
            devices: BTreeMap::new(),
        }
    }
}

impl<D: VirtioMmioDevice> Bus<D> {
    /// Create an empty bus.
    pub fn new() -> Self {
        Self::default()
    }

    /// Return the registered range and device associated with `addr`.
    pub fn device(&self, addr: GuestAddress) -> Option<(&BusRange, &D)> {
        // The range is returned as an optimization because the caller
        // might need both the device and its associated bus range.
        // The same goes for the device_mut() method.
        self.devices
            .range(..=BusRange::unit(addr))
            .nth_back(0)
            .filter(|pair| pair.0.last() >= addr)
    }

    /// Return the registered range and a mutable reference to the device
    /// associated with `addr`.
    pub fn device_mut(&mut self, addr: GuestAddress) -> Option<(&BusRange, &mut D)> {
        self.devices
            .range_mut(..=BusRange::unit(addr))
            .nth_back(0)
            .filter(|pair| pair.0.last() >= addr)
    }

    /// Register a device with the provided range.
    pub fn register(&mut self, range: BusRange, device: D) -> Result<(), Error> {
        for r in self.devices.keys() {
            if range.overlaps(r) {
                return Err(Error::DeviceOverlap);
            }
        }

        self.devices.insert(range, device);

        Ok(())
    }

    /// Deregister the device associated with `addr`.
    pub fn deregister(&mut self, addr: GuestAddress) -> Option<(BusRange, D)> {
        let range = self.device(addr).map(|(range, _)| *range)?;
        self.devices.remove(&range).map(|device| (range, device))
    }

    /// Verify whether an access starting at `addr` with length `len` fits within any of
    /// the registered ranges. Return the range and a handle to the device when present.
    pub fn check_access(&self, addr: GuestAddress, len: usize) -> Result<(&BusRange, &D), Error> {
        let access_range = BusRange::new(addr, len as u64).map_err(|_| Error::InvalidRange)?;
        self.device(addr)
            .filter(|(range, _)| range.last() >= access_range.last())
            .ok_or(Error::DeviceNotFound)
    }
}

/// Helper trait that can be implemented by types which hold one or more buses.
pub trait BusManager {
    /// Type of the objects held by the bus.
    type D;

    /// Return a reference to the bus.
    fn bus(&self) -> &Bus<Self::D>;

    /// Return a mutable reference to the bus.
    fn bus_mut(&mut self) -> &mut Bus<Self::D>;
}
