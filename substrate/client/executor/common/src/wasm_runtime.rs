// This file is part of Substrate.

// Copyright (C) Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! Definitions for a wasm runtime.

use crate::error::Error;

pub use sc_allocator::AllocationStats;
pub use sp_core::traits::CallContext;

/// Default heap allocation strategy for onchain execution.
pub const DEFAULT_HEAP_ALLOC_STRATEGY: HeapAllocStrategy = HeapAllocStrategy::Static {
	extra_pages: DEFAULT_HEAP_ALLOC_PAGES,
	offchain_heap_max_allocation: None,
};
/// Default heap allocation strategy for offchain execution.
pub const DEFAULT_OFFCHAIN_HEAP_ALLOC_STRATEGY: HeapAllocStrategy = HeapAllocStrategy::Static {
	extra_pages: DEFAULT_OFFCHAIN_HEAP_PAGES,
	offchain_heap_max_allocation: Some(DEFAULT_OFFCHAIN_HEAP_MAX_ALLOCATION),
};

/// Default heap allocation pages.
pub const DEFAULT_HEAP_ALLOC_PAGES: u32 = 2048;
/// The default extra heap pages for offchain execution, which is 30x the default extra pages for
/// onchain execution.
pub const DEFAULT_OFFCHAIN_HEAP_PAGES: u32 = 61440;
/// The default max allocation for offchain execution is 3GiB.
pub const DEFAULT_OFFCHAIN_HEAP_MAX_ALLOCATION: u32 = 3221225472;

/// A trait that defines an abstract WASM runtime module.
///
/// This can be implemented by an execution engine.
pub trait WasmModule: Sync + Send {
	/// Create a new instance.
	fn new_instance(&self) -> Result<Box<dyn WasmInstance>, Error>;
}

/// A trait that defines an abstract wasm module instance.
///
/// This can be implemented by an execution engine.
pub trait WasmInstance: Send {
	/// Call a method on this WASM instance.
	///
	/// Before execution, instance is reset.
	///
	/// Returns the encoded result on success.
	fn call(&mut self, method: &str, data: &[u8], context: CallContext) -> Result<Vec<u8>, Error> {
		// TODO(remove)
		log::info!(target: "wasm-heap", "___ WasmInstance::call with context {:?}", context);
		self.call_with_allocation_stats(method, data, context).0
	}

	/// Call a method on this WASM instance.
	///
	/// Before execution, instance is reset.
	///
	/// Returns the encoded result on success.
	fn call_with_allocation_stats(
		&mut self,
		method: &str,
		data: &[u8],
		context: CallContext,
	) -> (Result<Vec<u8>, Error>, Option<AllocationStats>);

	/// Call an exported method on this WASM instance.
	///
	/// Before execution, instance is reset.
	///
	/// Returns the encoded result on success.
	fn call_export(&mut self, method: &str, data: &[u8], context: CallContext) -> Result<Vec<u8>, Error> {
		self.call(method.into(), data, context)
	}
}

/// Defines the heap pages allocation strategy the wasm runtime should use.
///
/// A heap page is defined as 64KiB of memory.
#[derive(Debug, Copy, Clone, PartialEq, Hash, Eq)]
pub enum HeapAllocStrategy {
	/// Allocate a static number of heap pages.
	///
	/// The total number of allocated heap pages is the initial number of heap pages requested by
	/// the wasm file plus the `extra_pages`.
	Static {
		/// The number of pages that will be added on top of the initial heap pages requested by
		/// the wasm file.
		extra_pages: u32,
		/// Overwrite the maximum possible heap allocation in the offchain context if different
		/// than `None`.
		offchain_heap_max_allocation: Option<u32>,
	},
	/// Allocate the initial heap pages as requested by the wasm file and then allow it to grow
	/// dynamically.
	Dynamic {
		/// The absolute maximum size of the linear memory (in pages).
		///
		/// When `Some(_)` the linear memory will be allowed to grow up to this limit.
		/// When `None` the linear memory will be allowed to grow up to the maximum limit supported
		/// by WASM (4GB).
		maximum_pages: Option<u32>,
		/// Overwrite the maximum possible heap allocation in the offchain context if different
		/// than `None`.
		offchain_heap_max_allocation: Option<u32>,
	},
}
