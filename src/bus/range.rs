// Copyright 2020 Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR BSD-3-Clause

use std::cmp::Ordering;

use vm_memory::GuestAddress;

use crate::bus::Error;

/// An interval in the address space of a bus.
#[derive(Copy, Clone, Debug)]
pub struct BusRange {
    base: GuestAddress,
    size: u64,
}

impl BusRange {
    /// Create a new range while checking for overflow.
    pub fn new(base: GuestAddress, size: u64) -> Result<Self, Error> {
        // A zero-length range is not valid.
        if size == 0 {
            return Err(Error::InvalidRange);
        }

        // Subtracting one, because a range that ends at the very edge of the address space
        // is still valid.
        base.0.checked_add(size - 1).ok_or(Error::InvalidRange)?;

        Ok(BusRange { base, size })
    }

    /// Create a new unit range (its size equals `1`).
    pub fn unit(base: GuestAddress) -> Self {
        BusRange { base, size: 1 }
    }

    /// Return the base address of this range.
    pub fn base(&self) -> GuestAddress {
        self.base
    }

    /// Return the size of the range.
    pub fn size(&self) -> usize {
        self.size as usize
    }

    /// Return the last bus address that's still part of the range.
    pub fn last(&self) -> GuestAddress {
        GuestAddress(self.base.0 + (self.size - 1))
    }

    /// Check whether `self` and `other` overlap as intervals.
    pub fn overlaps(&self, other: &BusRange) -> bool {
        !(self.base > other.last() || self.last() < other.base)
    }
}

// We need to implement the following traits so we can use `BusRange` values with `BTreeMap`s.
// This usage scenario requires treating ranges as if they supported a total order, but that's
// not really possible with intervals, so we write the implementations as if `BusRange`s were
// solely determined by their base addresses, and apply extra checks in the `Bus` logic.

impl PartialEq for BusRange {
    fn eq(&self, other: &BusRange) -> bool {
        self.base == other.base
    }
}

impl Eq for BusRange {}

impl PartialOrd for BusRange {
    fn partial_cmp(&self, other: &BusRange) -> Option<Ordering> {
        self.base.partial_cmp(&other.base)
    }
}

impl Ord for BusRange {
    fn cmp(&self, other: &BusRange) -> Ordering {
        self.base.cmp(&other.base)
    }
}
