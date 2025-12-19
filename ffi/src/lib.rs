//! C FFI bindings for the `clique_fusion` crate.

use clique_fusion::{
    CHI2_2D_CONFIDENCE_90, CHI2_2D_CONFIDENCE_95, CHI2_2D_CONFIDENCE_99, CliqueIndex,
    CovarianceMatrix, Observation, Unique,
};
use uuid::Uuid;

#[unsafe(no_mangle)]
/// Returns the chi-squared confidence threshold at 90% for 2D observations.
pub const extern "C" fn CliqueIndex_chi2_confidence_90() -> f64 {
    CHI2_2D_CONFIDENCE_90
}

#[unsafe(no_mangle)]
/// Returns the chi-squared confidence threshold at 95% for 2D observations.
pub const extern "C" fn CliqueIndex_chi2_confidence_95() -> f64 {
    CHI2_2D_CONFIDENCE_95
}

#[unsafe(no_mangle)]
/// Returns the chi-squared confidence threshold at 99% for 2D observations.
pub const extern "C" fn CliqueIndex_chi2_confidence_99() -> f64 {
    CHI2_2D_CONFIDENCE_99
}

type UuidC = [u8; 16];

#[derive(Debug, Clone)]
#[repr(C)]
/// C-compatible observation data with covariance and optional context.
pub struct ObservationC {
    /// Observation UUID (16 bytes).
    pub id: UuidC,
    /// X coordinate.
    pub x: f64,
    /// Y coordinate.
    pub y: f64,
    /// Covariance XX term.
    pub cov_xx: f64,
    /// Covariance XY term.
    pub cov_xy: f64,
    /// Covariance YY term.
    pub cov_yy: f64,
    /// Optional context UUID; a nil UUID is treated as no context.
    pub context: UuidC,
}

const fn parse_uuid(bytes: UuidC) -> Option<Uuid> {
    let uuid = Uuid::from_bytes(bytes);
    if uuid.is_nil() { None } else { Some(uuid) }
}

impl From<ObservationC> for Unique<Observation, Uuid> {
    fn from(obs_c: ObservationC) -> Self {
        let id = Uuid::from_bytes(obs_c.id);
        let error = CovarianceMatrix::new_unchecked(obs_c.cov_xx, obs_c.cov_yy, obs_c.cov_xy);

        let mut observation_builder = Observation::builder(obs_c.x, obs_c.y).error(error);
        if let Some(context) = parse_uuid(obs_c.context) {
            observation_builder = observation_builder.context(context);
        }
        Self {
            id,
            data: observation_builder.build(),
        }
    }
}

/// Initialise a new [`CliqueIndex`].
#[unsafe(no_mangle)]
pub extern "C" fn CliqueIndex_new(chi2: f64) -> *mut CliqueIndex<Uuid> {
    Box::into_raw(Box::new(CliqueIndex::new(chi2)))
}

/// Initialise a new [`CliqueIndex`] from a list of observations.
///
/// This is faster than creating an empty index and adding the observations one at a time.
///
/// # Safety
///
/// - `observations` must be a valid pointer to `len` contiguous `ObservationC` structs.
/// - `observations` must not be null unless `len == 0`.
/// - The memory referenced by `observations` must remain valid for the duration of the call.
/// - The returned pointer must be freed with `CliqueIndex_free` when no longer needed.
///
/// # Errors
///
/// - If `observations` is null and `len > 0`, this function returns a null pointer.
///   The caller should check the return value before using it.
///
/// # Example
/// ```c
/// CliqueIndex* idx = CliqueIndex_from_observations(chi2, obs_array, len);
/// if (idx == NULL) {
///     // Handle error
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn CliqueIndex_from_observations(
    chi2: f64,
    observations: *const ObservationC,
    len: usize,
) -> *mut CliqueIndex<Uuid> {
    if observations.is_null() {
        return std::ptr::null_mut();
    }
    let obs_slice = unsafe { std::slice::from_raw_parts(observations, len) };
    let rust_obs = obs_slice
        .iter()
        .cloned()
        .map(Unique::<Observation, Uuid>::from)
        .collect();
    Box::into_raw(Box::new(CliqueIndex::from_observations(rust_obs, chi2)))
}

#[unsafe(no_mangle)]
/// Insert an observation into an existing [`CliqueIndex`].
///
/// Note that it is quicker to create a [`CliqueIndex`] from a batch of observations using
/// [`CliqueIndex_from_observations`], but this function is useful for incrementally adding observations.
///
/// # Safety
///
/// - `clique_index_ptr` must be a valid, non-null pointer to a `CliqueIndex<Uuid>`.
/// - `observation` must be a valid, non-null pointer to an `ObservationC`.
/// - The caller must ensure that no other references (mutable or immutable) to the `CliqueIndex`
///   exist for the duration of the call (i.e., uphold Rust aliasing rules).
///
/// # Errors
///
/// - If either pointer is null, this function does nothing.
///
/// This function does not take ownership of `clique_index_ptr`; it modifies the pointed-to object
/// in-place. The pointer remains valid after the call.
pub unsafe extern "C" fn CliqueIndex_insert(
    clique_index_ptr: *mut CliqueIndex<Uuid>,
    observation: *const ObservationC,
) {
    if clique_index_ptr.is_null() || observation.is_null() {
        return;
    }

    let clique_index = unsafe { &mut *clique_index_ptr };
    let rust_obs = Unique::<Observation, Uuid>::from(unsafe { (*observation).clone() });
    clique_index.insert(rust_obs);
}

/// A single clique: a set of UUIDs (observations) belonging to the same maximal clique.
///
/// # Fields
/// - `uuids`: A pointer to an array of 16-byte UUIDs. Must be valid for reads.
/// - `len`: The number of UUIDs in this clique.
#[derive(Debug)]
#[repr(C)]
pub struct CliqueC {
    /// Pointer to an array of 16-byte UUIDs.
    pub uuids: *const UuidC,
    /// Number of UUIDs in the clique.
    pub len: usize,
}

/// A set of maximal cliques returned by `CliqueIndex_cliques`.
///
/// # Fields
/// - `cliques`: Pointer to an array of [`CliqueC`] structures.
/// - `len`: Number of cliques in the set.
#[derive(Debug)]
#[repr(C)]
pub struct CliqueSetC {
    /// Pointer to an array of `CliqueC` structures.
    pub cliques: *const CliqueC,
    /// Number of cliques in the set.
    pub len: usize,
}

/// Frees memory previously allocated by `CliqueIndex_cliques`.
///
/// This deallocates all memory owned by the `CliqueSetC`, including:
/// - Each inner list of UUIDs (allocated as `Vec<[u8; 16]>`)
/// - The outer array of `CliqueC`
///
/// # Safety
///
/// - `ptr` must be a valid pointer returned by `CliqueIndex_cliques` and must not be used again after calling this.
/// - The caller must ensure that no aliasing or use-after-free occurs.
/// - This function **must not** be called on any pointer not allocated by the library.
///
/// # Example (C side)
/// ```c
/// CliqueSetC* result = CliqueIndex_cliques(index);
/// // ... use result ...
/// CliqueSetC_free(result);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn CliqueSetC_free(ptr: *mut CliqueSetC) {
    if ptr.is_null() {
        return;
    }

    let boxed = unsafe { Box::from_raw(ptr) };

    // Fully reconstruct the outer Vec<CliqueC>
    let cliques_vec =
        unsafe { Vec::from_raw_parts(boxed.cliques.cast_mut(), boxed.len, boxed.len) };

    for clique in cliques_vec {
        // Reconstruct and drop the inner UUID arrays
        let _ = unsafe { Vec::from_raw_parts(clique.uuids.cast_mut(), clique.len, clique.len) };
    }

    // `boxed` is dropped here, releasing CliqueSetC itself
}

/// Returns the current set of maximal cliques from the [`CliqueIndex`].
///
/// Each clique is represented as a set of UUIDs. The result is wrapped in a
/// [`CliqueSetC`] structure containing an array of [`CliqueC`] entries.
///
/// # Safety
///
/// - `ptr` must be a valid, non-null pointer to a [`CliqueIndex<Uuid>`] allocated by this library.
/// - The caller takes ownership of the returned pointer and is responsible for freeing it using
///   [`CliqueSetC_free`] to avoid memory leaks.
/// - The returned structure points to heap-allocated memory and must not be mutated.
///
/// # Errors
///
/// If `ptr` is null, this function returns a null pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn CliqueIndex_cliques(ptr: *const CliqueIndex<Uuid>) -> *mut CliqueSetC {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }

    // SAFETY: We checked for null above.
    let index = unsafe { &*ptr };
    let cliques = index.cliques();

    // Build a vector of `CliqueC` entries with raw UUID arrays.
    let mut clique_cs: Vec<CliqueC> = cliques
        .iter()
        .map(|clique| {
            let mut uuid_vec: Vec<[u8; 16]> = clique.iter().map(|id| *id.as_bytes()).collect();
            let len = uuid_vec.len();
            let ptr = uuid_vec.as_mut_ptr();
            std::mem::forget(uuid_vec); // Prevent Rust from freeing the UUIDs
            CliqueC { uuids: ptr, len }
        })
        .collect();

    // Get raw pointer to the `CliqueC` array
    let len = clique_cs.len();
    let clique_ptr = clique_cs.as_mut_ptr();
    std::mem::forget(clique_cs); // Prevent Rust from freeing the vector

    // Box and return the outer structure
    let result = Box::new(CliqueSetC {
        cliques: clique_ptr,
        len,
    });

    Box::into_raw(result)
}

/// Free the memory associated with a [`CliqueIndex`].
///
/// # Safety
///
/// `ptr` must have been returned by `CliqueIndex_new` and not already freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn CliqueIndex_free(ptr: *mut CliqueIndex<Uuid>) {
    if !ptr.is_null() {
        unsafe {
            drop(Box::from_raw(ptr));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;
    use clique_fusion::Observation;
    use uuid::Uuid;

    fn sample_uuid() -> Uuid {
        Uuid::parse_str("12345678-1234-5678-1234-567812345678").unwrap()
    }

    fn nil_uuid() -> UuidC {
        [0u8; 16]
    }

    fn uuidc_from_uuid(uuid: Uuid) -> UuidC {
        *uuid.as_bytes()
    }

    #[test]
    fn test_parse_uuid_some() {
        let uuid = sample_uuid();
        let parsed = parse_uuid(uuidc_from_uuid(uuid));
        assert_eq!(parsed, Some(uuid));
    }

    #[test]
    fn test_parse_uuid_none() {
        let parsed = parse_uuid(nil_uuid());
        assert_eq!(parsed, None);
    }

    #[test]
    fn test_observationc_to_unique_without_context() {
        let id = sample_uuid();
        let obs_c = ObservationC {
            id: uuidc_from_uuid(id),
            x: 1.0,
            y: 2.0,
            cov_xx: 0.1,
            cov_xy: 0.01,
            cov_yy: 0.2,
            context: nil_uuid(),
        };

        let unique: Unique<Observation, Uuid> = obs_c.into();

        assert_eq!(unique.id, id);
        assert_relative_eq!(unique.data.x(), 1.0, epsilon = 1e-12);
        assert_relative_eq!(unique.data.y(), 2.0, epsilon = 1e-12);
        assert_eq!(unique.data.context(), None);
    }

    #[test]
    fn test_observationc_to_unique_with_context() {
        let id = sample_uuid();
        let ctx = Uuid::parse_str("abcdefab-cdef-abcd-efab-cdefabcdefab").unwrap();

        let obs_c = ObservationC {
            id: uuidc_from_uuid(id),
            x: 3.0,
            y: 4.0,
            cov_xx: 0.3,
            cov_xy: 0.02,
            cov_yy: 0.4,
            context: uuidc_from_uuid(ctx),
        };

        let unique: Unique<Observation, Uuid> = obs_c.into();

        assert_eq!(unique.id, id);
        assert_relative_eq!(unique.data.x(), 3.0, epsilon = 1e-12);
        assert_relative_eq!(unique.data.y(), 4.0, epsilon = 1e-12);
        assert_eq!(unique.data.context(), Some(ctx));
    }
}
