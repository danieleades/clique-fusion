use std::ops::Add;

use super::CHI2_2D_CONFIDENCE_95;
use nalgebra::Matrix2;

/// Relative error to use for checking matrices are positive semi-definite
const PSD_EPS_REL: f64 = 1e-12;

/// A covariance matrix, used to represent the positional error ellipse of an observation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CovarianceMatrix(Matrix2<f64>);

impl CovarianceMatrix {
    /// construct a new covariance matrix from its components.
    ///
    /// for trusted and correct input, [`Self::new_unchecked`] is marginally more performant.
    ///
    /// # Errors
    ///
    /// Returns an error if the given values do not describe a positive semi-definite covariance matrix.
    ///
    /// This requires that `xx >= 0.0 && yy >= 0.0 && det >= 0.0`
    /// where `det = xx * yy - xy * xy`.
    ///
    /// It also requires that the inputs be finite.
    pub fn new(xx: f64, yy: f64, xy: f64) -> Result<Self, InvalidCovarianceMatrix> {
        // 1) Check for NaN or infinite values first
        if !xx.is_finite() || !yy.is_finite() || !xy.is_finite() {
            return Err(InvalidCovarianceMatrix { xx, yy, xy });
        }

        // 2) Scale-aware tolerances (dimensionally consistent)
        //    - diagonals have units of variance
        //    - determinant has units of variance^2
        let scale = xx.abs().max(yy.abs()).max(xy.abs());
        // if scale == 0, matrix must be exactly zero to be valid; tolerances collapse to 0
        let diag_tol = PSD_EPS_REL * scale;
        let det_tol = PSD_EPS_REL * scale * scale;

        let det = xx.mul_add(yy, -(xy * xy));

        let diag_ok = xx >= -diag_tol && yy >= -diag_tol;
        let det_ok = det >= -det_tol;

        if diag_ok && det_ok {
            Ok(Self(Matrix2::new(xx, xy, xy, yy)))
        } else {
            Err(InvalidCovarianceMatrix { xx, yy, xy })
        }
    }

    /// construct a new covariance matrix from its components, without checking the input.
    ///
    /// BEWARE: use only for trusted, correct input.
    ///
    /// # Panics
    ///
    /// This method panics in debug builds if [`Self::new`] would have returned an error.
    ///
    /// In release builds no checking is done.
    #[must_use]
    pub fn new_unchecked(xx: f64, yy: f64, xy: f64) -> Self {
        if cfg!(debug_assertions) {
            Self::new(xx, yy, xy).unwrap()
        } else {
            Self(Matrix2::new(xx, xy, xy, yy))
        }
    }

    /// Return the variance of the error in the x direction
    ///
    /// This is guaranteed to be >= 0.0
    #[must_use]
    pub fn xx(&self) -> f64 {
        self.0[(0, 0)]
    }

    /// Return the variance of the error in the y direction
    ///
    /// This is guaranteed to be >= 0.0
    #[must_use]
    pub fn yy(&self) -> f64 {
        self.0[(1, 1)]
    }

    /// Return the covariance between the x and y directions
    ///
    /// note that xy == yx (covariance matrices are symmetric).
    #[must_use]
    pub fn xy(&self) -> f64 {
        self.0[(0, 1)]
    }

    /// The determinant of the covariance matrix
    #[must_use]
    pub fn determinant(&self) -> f64 {
        self.0.determinant()
    }

    /// The identity matrix
    #[must_use]
    pub fn identity() -> Self {
        Self(Matrix2::identity())
    }

    /// Create a covariance matrix for a circular 95% confidence interval with given radius.
    ///
    /// This is a legacy compatibility constructor that creates an isotropic covariance matrix
    /// where the 95% confidence ellipse is a circle with the specified radius.
    ///
    /// # Arguments
    /// * `radius` - The radius of the 95% confidence circle
    ///
    /// # Returns
    /// A covariance matrix representing circular uncertainty with the given radius
    ///
    /// # Errors
    ///
    /// Returns an error if the radius is less than 0.
    pub fn from_circular_95_confidence(radius: f64) -> Result<Self, InvalidRadius> {
        if !radius.is_finite() || radius < 0.0 {
            return Err(InvalidRadius(radius));
        }

        // For a circular confidence region: radius = sqrt(chi2 * sigma^2)
        // Therefore: sigma^2 = radius^2 / chi2
        let variance = (radius * radius) / CHI2_2D_CONFIDENCE_95;

        // Create isotropic covariance matrix [σ², 0; 0, σ²]
        Ok(Self(Matrix2::from_diagonal_element(variance)))
    }

    /// The maximum eigenvalue of the covariance matrix
    #[must_use]
    pub fn max_variance(&self) -> f64 {
        let trace = self.0.trace();
        let det = self.determinant();
        let discrim = trace.mul_add(trace, -(4.0 * det)).max(0.0).sqrt(); // Clamp to avoid sqrt of -ε
        0.5 * (trace + discrim)
    }

    /// Safely compute the inverse of the covariance matrix, handling different cases gracefully
    ///
    /// # Returns
    /// - `Some(CovarianceMatrix)` for non-zero matrices
    /// - `None` for zero matrices
    ///
    ///
    /// # Examples
    /// ```
    /// use clique_fusion::CovarianceMatrix;
    /// let cov = CovarianceMatrix::new(4.0, 1.0, 0.5).unwrap();
    /// let inv = cov.safe_inverse().unwrap();
    /// ```
    #[allow(clippy::missing_panics_doc)]
    #[must_use]
    pub fn safe_inverse(&self) -> Option<Matrix2<f64>> {
        let m = self.0;

        if m.norm() < 1e-15 {
            return None;
        }

        if let Some(inv) = m.try_inverse() {
            return Some(inv);
        }

        let svd = m.svd(true, true);

        Some(
            svd.pseudo_inverse(1e-12)
                .expect("unable to calculate pseudo-inverse"),
        )
    }
}

#[derive(Debug, thiserror::Error, Clone, Copy)]
#[error("radius must be >=0.0 (got {0})")]
pub struct InvalidRadius(f64);

/// The error returned when the given variances do not form a valid covariance matrix
#[derive(Debug, thiserror::Error, Clone, Copy)]
#[error("not a valid positive semi-definite matrix (xx: {xx}, yy: {yy}, xy: {xy})")]
pub struct InvalidCovarianceMatrix {
    xx: f64,
    yy: f64,
    xy: f64,
}

impl From<CovarianceMatrix> for Matrix2<f64> {
    fn from(covariance_matrix: CovarianceMatrix) -> Self {
        covariance_matrix.0
    }
}

impl Add for CovarianceMatrix {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self(self.0 + other.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    // Existing tests...
    #[test]
    fn covariance_matrix_accepts_spd() {
        assert!(CovarianceMatrix::new(2.0, 1.0, 0.0).is_ok());
    }

    #[test]
    fn covariance_matrix_accepts_singular() {
        assert!(CovarianceMatrix::new(1.0, 0.0, 0.0).is_ok()); // rank-deficient but valid
    }

    #[test]
    fn covariance_matrix_rejects_negative_definite() {
        assert!(CovarianceMatrix::new(-1.0, 1.0, 0.0).is_err());
        assert!(CovarianceMatrix::new(1.0, -1.0, 0.0).is_err());
        assert!(CovarianceMatrix::new(1.0, 1.0, -2.0).is_err());
    }

    #[test]
    fn identity_matrix_is_valid() {
        let id = CovarianceMatrix::identity();
        assert_relative_eq!(id.determinant(), 1.0, epsilon = 1e-12);
    }

    #[test]
    fn max_variance_correct_for_diagonal_matrix() {
        let cov = CovarianceMatrix::new_unchecked(3.0, 2.0, 0.0);
        assert_relative_eq!(cov.max_variance(), 3.0, epsilon = 1e-12);
    }

    #[test]
    fn max_variance_correct_for_off_diagonal_matrix() {
        let cov = CovarianceMatrix::new_unchecked(4.0, 1.0, 1.0);
        let trace: f64 = 4.0 + 1.0;
        let det = 4.0f64.mul_add(1.0, -(1.0 * 1.0));
        let discrim = trace.mul_add(trace, -(4.0 * det)).sqrt();
        let expected = 0.5 * (trace + discrim);
        assert_relative_eq!(cov.max_variance(), expected, epsilon = 1e-12);
    }

    #[test]
    fn safe_inverse_returns_none_for_zero_matrix() {
        let zero = CovarianceMatrix::new_unchecked(0.0, 0.0, 0.0);
        assert!(zero.safe_inverse().is_none());
    }

    #[test]
    fn safe_inverse_returns_some_for_valid_matrix() {
        let cov = CovarianceMatrix::new_unchecked(2.0, 2.0, 0.5);
        let inv = cov.safe_inverse();
        assert!(inv.is_some());
        let inv_m = inv.unwrap();
        let approx_identity = inv_m * Matrix2::from(cov);
        assert_relative_eq!(approx_identity, Matrix2::identity(), epsilon = 1e-8);
    }

    #[test]
    fn safe_inverse_of_rank_deficient_matrix_returns_pseudoinverse() {
        // Matrix: [1, 1; 1, 1] has rank 1 and determinant 0
        let cov = CovarianceMatrix::new_unchecked(1.0, 1.0, 1.0);

        let inv = cov.safe_inverse();
        assert!(
            inv.is_some(),
            "Expected Some(pseudoinverse) for singular but non-zero matrix"
        );

        let inv = inv.unwrap();

        // Expected pseudoinverse should satisfy A @ A⁺ @ A ≈ A
        let a: Matrix2<f64> = cov.into();
        let approx_a = a * inv * a;

        // Compare reconstructed matrix to original
        assert_relative_eq!(approx_a, a, epsilon = 1e-10);
    }

    #[test]
    fn from_circular_95_confidence_accepts_positive_radius() {
        let radius = 2.0;
        let cov = CovarianceMatrix::from_circular_95_confidence(radius).unwrap();
        let expected_variance = (radius * radius) / CHI2_2D_CONFIDENCE_95;
        let expected = Matrix2::new(expected_variance, 0.0, 0.0, expected_variance);
        assert_relative_eq!(Matrix2::from(cov), expected, epsilon = 1e-12);
    }

    #[test]
    fn from_circular_95_confidence_rejects_negative_radius() {
        assert!(CovarianceMatrix::from_circular_95_confidence(-1.0).is_err());
    }

    #[test]
    fn into_matrix2_conversion_is_correct() {
        let cov = CovarianceMatrix::new_unchecked(1.0, 2.0, 0.5);
        let mat: Matrix2<f64> = cov.into();

        assert_relative_eq!(mat[(0, 0)], 1.0, epsilon = 1e-12);
        assert_relative_eq!(mat[(1, 1)], 2.0, epsilon = 1e-12);
        assert_relative_eq!(mat[(0, 1)], 0.5, epsilon = 1e-12);
        assert_relative_eq!(mat[(1, 0)], 0.5, epsilon = 1e-12);
    }

    // NEW EDGE CASE TESTS

    // NaN handling tests
    #[test]
    fn covariance_matrix_rejects_nan_values() {
        assert!(CovarianceMatrix::new(f64::NAN, 1.0, 0.0).is_err());
        assert!(CovarianceMatrix::new(1.0, f64::NAN, 0.0).is_err());
        assert!(CovarianceMatrix::new(1.0, 1.0, f64::NAN).is_err());
        assert!(CovarianceMatrix::new(f64::NAN, f64::NAN, f64::NAN).is_err());
    }

    // Infinity handling tests
    #[test]
    fn covariance_matrix_rejects_infinite_values() {
        assert!(CovarianceMatrix::new(f64::INFINITY, 1.0, 0.0).is_err());
        assert!(CovarianceMatrix::new(1.0, f64::INFINITY, 0.0).is_err());
        assert!(CovarianceMatrix::new(1.0, 1.0, f64::INFINITY).is_err());
        assert!(CovarianceMatrix::new(f64::NEG_INFINITY, 1.0, 0.0).is_err());
        assert!(CovarianceMatrix::new(1.0, f64::NEG_INFINITY, 0.0).is_err());
        assert!(CovarianceMatrix::new(1.0, 1.0, f64::NEG_INFINITY).is_err());
    }

    // Very small values near machine epsilon
    #[test]
    fn covariance_matrix_handles_very_small_values() {
        let epsilon = f64::EPSILON;
        assert!(CovarianceMatrix::new(epsilon, epsilon, 0.0).is_ok());
        assert!(CovarianceMatrix::new(epsilon * 10.0, epsilon * 10.0, epsilon).is_ok());
    }

    // Very large finite values
    #[test]
    fn covariance_matrix_handles_very_large_values() {
        let large = f64::MAX / 4.0; // Avoid overflow in determinant calculation
        assert!(CovarianceMatrix::new(large, large, 0.0).is_ok());
    }

    // Edge cases for determinant calculation
    #[test]
    fn determinant_edge_cases() {
        // Test determinant with values that could cause overflow
        let sqrt_max = f64::MAX.sqrt() / 2.0;
        let cov = CovarianceMatrix::new_unchecked(sqrt_max, sqrt_max, 0.0);
        assert!(cov.determinant().is_finite());

        // Test with very small values
        let tiny = f64::MIN_POSITIVE;
        let cov_small = CovarianceMatrix::new_unchecked(tiny, tiny, 0.0);
        assert!(cov_small.determinant() >= 0.0);
    }

    // Edge cases for max_variance calculation
    #[test]
    fn max_variance_edge_cases() {
        // Test with zero variance in one direction
        let cov = CovarianceMatrix::new_unchecked(5.0, 0.0, 0.0);
        assert_relative_eq!(cov.max_variance(), 5.0, epsilon = 1e-12);

        // Test with equal variances and correlation
        let cov = CovarianceMatrix::new_unchecked(2.0, 2.0, 1.0);
        assert!(cov.max_variance() > 2.0); // Should be larger due to correlation

        // Test numerical stability with very small discriminant
        let cov = CovarianceMatrix::new_unchecked(1.0 + f64::EPSILON, 1.0, 1.0 - f64::EPSILON);
        assert!(cov.max_variance().is_finite());
    }

    // Safe inverse edge cases
    #[test]
    fn safe_inverse_handles_near_zero_norm() {
        // Matrix with extremely small but non-zero values
        let tiny = 1e-16;
        let cov = CovarianceMatrix::new_unchecked(tiny, tiny, 0.0);
        assert!(cov.safe_inverse().is_none()); // Should be treated as zero
    }

    #[test]
    fn safe_inverse_handles_ill_conditioned_matrix() {
        // Create an ill-conditioned matrix (large condition number)
        let large = 1e10;
        let small = 1e-10;
        let cov = CovarianceMatrix::new_unchecked(large, small, 0.0);
        let inv = cov.safe_inverse();
        assert!(inv.is_some());

        // Verify the inverse is reasonable (not containing extreme values)
        let inv_matrix = inv.unwrap();
        assert!(inv_matrix[(0, 0)].is_finite());
        assert!(inv_matrix[(1, 1)].is_finite());
    }

    // Circular confidence constructor edge cases
    #[test]
    fn from_circular_95_confidence_handles_zero_radius() {
        let cov = CovarianceMatrix::from_circular_95_confidence(0.0).unwrap();
        assert_relative_eq!(cov.xx(), 0.0, epsilon = 1e-15);
        assert_relative_eq!(cov.yy(), 0.0, epsilon = 1e-15);
        assert_relative_eq!(cov.xy(), 0.0, epsilon = 1e-15);
    }

    #[test]
    fn from_circular_95_confidence_rejects_nan_radius() {
        assert!(CovarianceMatrix::from_circular_95_confidence(f64::NAN).is_err());
    }

    #[test]
    fn from_circular_95_confidence_rejects_infinite_radius() {
        assert!(CovarianceMatrix::from_circular_95_confidence(f64::INFINITY).is_err());
        assert!(CovarianceMatrix::from_circular_95_confidence(f64::NEG_INFINITY).is_err());
    }

    #[test]
    fn from_circular_95_confidence_handles_very_large_radius() {
        let large_radius = 1e6;
        let cov = CovarianceMatrix::from_circular_95_confidence(large_radius).unwrap();
        assert!(cov.xx().is_finite());
        assert!(cov.yy().is_finite());
        assert!(cov.determinant().is_finite());
    }

    #[test]
    fn from_circular_95_confidence_handles_very_small_radius() {
        let tiny_radius = f64::MIN_POSITIVE;
        let cov = CovarianceMatrix::from_circular_95_confidence(tiny_radius).unwrap();
        assert!(cov.xx() >= 0.0);
        assert!(cov.yy() >= 0.0);
    }

    // Test accessor methods with edge cases
    #[test]
    fn accessor_methods_edge_cases() {
        // Test accessors with various edge case matrices
        let cases = vec![
            (f64::MIN_POSITIVE, f64::MIN_POSITIVE, 0.0),
            (1e-10, 1e10, 0.0),
            (100.0, 100.0, 99.99), // High correlation
        ];

        for (xx, yy, xy) in cases {
            let cov = CovarianceMatrix::new_unchecked(xx, yy, xy);
            assert_relative_eq!(cov.xx(), xx);
            assert_relative_eq!(cov.yy(), yy);
            assert_relative_eq!(cov.xy(), xy);
            assert!(cov.determinant().is_finite());
        }
    }

    // Test numerical stability of calculations
    #[test]
    fn numerical_stability_tests() {
        // Test with values that could cause numerical issues in calculations
        let test_cases = vec![
            // (xx, yy, xy, description)
            (1e-100, 1e-100, 0.0),   // Extremely small values
            (1e100, 1e-100, 0.0),    // Mixed scales
            (1.0, 1.0, 1.0 - 1e-15), // Nearly singular
        ];

        for (xx, yy, xy) in test_cases {
            let cov = CovarianceMatrix::new_unchecked(xx, yy, xy);
            // All calculations should produce finite results
            assert!(cov.determinant().is_finite());
            assert!(cov.max_variance().is_finite());
            assert!(cov.max_variance() >= 0.0);

            // Test safe_inverse doesn't panic
            let _inv = cov.safe_inverse();
        }
    }

    #[test]
    fn constructor_accepts_small_negative_det() {
        let xx: f64 = 4.0;
        let yy: f64 = 9.0;
        let scale: f64 = 9.0;
        let det_tol: f64 = PSD_EPS_REL * scale * scale;

        // Safely within tolerance
        let target_det: f64 = -0.5 * det_tol;

        // xy' so that xx*yy - xy'^2 = target_det
        let xy_pert: f64 = (xx * yy - target_det).sqrt();

        let cov = CovarianceMatrix::new(xx, yy, xy_pert);
        assert!(
            cov.is_ok(),
            "should accept det slightly below zero within tolerance"
        );
    }

    #[test]
    fn constructor_scales_with_magnitude() {
        for &k in &[1e-9_f64, 1.0_f64, 1e9_f64] {
            let scale: f64 = k;
            let diag_tol: f64 = PSD_EPS_REL * scale;

            // Within tolerance (accept)
            let xx_within: f64 = -0.9 * diag_tol;
            let cov_ok = CovarianceMatrix::new(xx_within, scale, 0.0);
            assert!(
                cov_ok.is_ok(),
                "should accept tiny negative diagonal at scale {k}"
            );

            // Beyond tolerance (reject)
            let xx_beyond: f64 = -10.0 * diag_tol;
            let cov_bad = CovarianceMatrix::new(xx_beyond, scale, 0.0);
            assert!(
                cov_bad.is_err(),
                "should reject negative diagonal beyond tolerance at scale {k}"
            );
        }
    }

    #[test]
    fn constructor_rejects_clearly_invalid() {
        // Negative variance far beyond tolerance
        assert!(CovarianceMatrix::new(-1e-3, 1.0, 0.0).is_err());

        // |xy| > sqrt(xx*yy) by a large margin -> strongly negative det
        let xx = 1.0;
        let yy = 1.0;
        let xy = 2.0; // det = 1 - 4 = -3
        assert!(CovarianceMatrix::new(xx, yy, xy).is_err());
    }

    #[test]
    fn constructor_zero_scale_behaviour() {
        // Exactly zero matrix remains valid
        assert!(CovarianceMatrix::new(0.0, 0.0, 0.0).is_ok());

        // With scale = 0, tolerances are zero; any negative diagonal must be rejected
        assert!(CovarianceMatrix::new(-1e-16, 0.0, 0.0).is_err());
    }

    #[test]
    fn tolerance_does_not_break_max_variance_or_inverse() {
        // Nearly singular but within tolerance
        let xx: f64 = 1.0;
        let yy: f64 = 1.0;
        let xy: f64 = (xx * yy - 1e-20).sqrt(); // det ≈ 1e-20
        let cov = CovarianceMatrix::new(xx, yy, xy).unwrap();

        // Numerical functions remain stable
        let mv = cov.max_variance();
        assert!(mv.is_finite() && mv >= 0.0);

        // safe_inverse should produce Some (near singular -> pseudo-inverse path is fine)
        let inv = cov.safe_inverse();
        assert!(inv.is_some());
    }

    #[test]
    fn covariance_matrix_boundary_conditions() {
        // Determinant exactly zero (singular but valid)
        assert!(CovarianceMatrix::new(1.0, 1.0, 1.0).is_ok());

        // One variance zero, other positive
        assert!(CovarianceMatrix::new(0.0, 1.0, 0.0).is_ok());
        assert!(CovarianceMatrix::new(1.0, 0.0, 0.0).is_ok());

        // Boundary correlation (|xy| = sqrt(xx * yy))
        let xx: f64 = 4.0;
        let yy: f64 = 9.0;
        let xy_max: f64 = (xx * yy).sqrt();
        assert!(CovarianceMatrix::new(xx, yy, xy_max).is_ok());
        assert!(CovarianceMatrix::new(xx, yy, -xy_max).is_ok());

        // Exceed boundary by enough to violate the tolerant det check
        let scale: f64 = f64::max(xx.abs(), f64::max(yy.abs(), xy_max.abs())); // 9.0
        let det_tol: f64 = PSD_EPS_REL * scale * scale;
        let delta: f64 = 1.1 * det_tol / (2.0 * xy_max); // ensures det ≈ -2*xy_max*delta < -det_tol

        let xy_over_pos: f64 = xy_max + delta;
        let xy_over_neg: f64 = -xy_max - delta;

        assert!(CovarianceMatrix::new(xx, yy, xy_over_pos).is_err());
        assert!(CovarianceMatrix::new(xx, yy, xy_over_neg).is_err());
    }
}
