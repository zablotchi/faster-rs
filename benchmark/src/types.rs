use faster_rs::FasterRmw;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;

// Thread-local RNG for generating random padding
thread_local! {
    static PADDING_RNG: RefCell<rand::rngs::ThreadRng> = RefCell::new(rand::thread_rng());
}

/// 32-byte key matching C++ benchmark.cc implementation
/// First 8 bytes: actual key value
/// Remaining 24 bytes: random padding for realistic memory patterns
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BenchmarkKey {
    #[serde(with = "serde_big_array::BigArray")]
    data: [u8; 32],
}

impl BenchmarkKey {
    pub fn new(key: u64) -> Self {
        let mut data = [0u8; 32];

        // Store the 64-bit key in first 8 bytes (little-endian)
        data[0..8].copy_from_slice(&key.to_le_bytes());

        // Fill remaining 24 bytes with random data
        PADDING_RNG.with(|rng| {
            let mut rng = rng.borrow_mut();
            for i in 8..32 {
                data[i] = rng.gen::<u8>();
            }
        });

        BenchmarkKey { data }
    }

    #[allow(dead_code)]
    pub fn get_key(&self) -> u64 {
        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(&self.data[0..8]);
        u64::from_le_bytes(bytes)
    }
}

/// 512-byte value matching C++ benchmark.cc implementation
/// First 8 bytes: actual value
/// Remaining 504 bytes: random padding for realistic memory patterns
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BenchmarkValue {
    #[serde(with = "serde_big_array::BigArray")]
    data: [u8; 512],
}

impl BenchmarkValue {
    pub fn new(value: u64) -> Self {
        let mut data = [0u8; 512];

        // Store the 64-bit value in first 8 bytes (little-endian)
        data[0..8].copy_from_slice(&value.to_le_bytes());

        // Fill remaining 504 bytes with random data
        PADDING_RNG.with(|rng| {
            let mut rng = rng.borrow_mut();
            for i in 8..512 {
                data[i] = rng.gen::<u8>();
            }
        });

        BenchmarkValue { data }
    }

    #[allow(dead_code)]
    pub fn get_value(&self) -> u64 {
        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(&self.data[0..8]);
        u64::from_le_bytes(bytes)
    }
}

/// Implement Read-Modify-Write for BenchmarkValue
/// Extracts first 8 bytes as u64, performs addition, stores result back
impl FasterRmw for BenchmarkValue {
    fn rmw(&self, modification: Self) -> Self {
        // Extract current value from first 8 bytes
        let current_val = self.get_value();

        // Extract modification value from first 8 bytes
        let mod_val = modification.get_value();

        // Add them together
        let new_val = current_val.wrapping_add(mod_val);

        // Create new value with updated first 8 bytes, preserving random padding
        let mut result = self.clone();
        result.data[0..8].copy_from_slice(&new_val.to_le_bytes());

        result
    }
}
