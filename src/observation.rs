//! Observations with 2D positional uncertainty and statistical compatibility tests.
//!
//! An [`Observation`] pairs a 2D position with a covariance matrix capturing
//! Gaussian positional uncertainty. Compatibility between two observations is
//! tested via the squared Mahalanobis distance using the **sum** of their
//! covariances, which is optimal under independence:
//! 
//! - Effective covariance: Σ = Σ_A + Σ_B
//! - Distance: d² = (A − B)ᵀ Σ⁻¹ (A − B)
//!
//! Compare `d²` to a chi-square threshold with 2 degrees of freedom
//! (e.g., `CHI2_2D_CONFIDENCE_95 = 5.991464547`) to decide compatibility.
//!
//! The helper [`Observation::max_compatibility_radius`] provides a conservative
//! Euclidean radius for spatial prefiltering, using the spectral bound
//! `λ_max(Σ_A + Σ_B) ≤ λ_max(Σ_A) + λ_max(Σ_B)`.
//!
//! # Examples
//! See the `Observation::builder` examples and `is_compatible_with` docs below.

use nalgebra::{Point2, Vector2};

mod covariance_matrix;
pub use covariance_matrix::CovarianceMatrix;
pub use covariance_matrix::InvalidCovarianceMatrix;
use uuid::Uuid;

use crate::observation::covariance_matrix::InvalidRadius;

/// Chi-squared threshold for 90% confidence in 2D (2 degrees of freedom)
pub const CHI2_2D_CONFIDENCE_90: f64 = 4.605170186;

/// Chi-squared threshold for 95% confidence in 2D (2 degrees of freedom)
pub const CHI2_2D_CONFIDENCE_95: f64 = 5.991464547;

/// Chi-squared threshold for 99% confidence in 2D (2 degrees of freedom)
pub const CHI2_2D_CONFIDENCE_99: f64 = 9.210340372;

#[must_use]
#[derive(Debug)]
pub struct ObservationBuilder<E> {
    position: Point2<f64>,
    error: E,
    context: Option<Uuid>,
}

impl ObservationBuilder<()> {
    const fn new(x: f64, y: f64) -> Self {
        Self {
            position: Point2::new(x, y),
            error: (),
            context: None,
        }
    }

    /// Sets the positional error for the [`Observation`].
    pub const fn error(self, error: CovarianceMatrix) -> ObservationBuilder<CovarianceMatrix> {
        ObservationBuilder {
            position: self.position,
            error,
            context: self.context,
        }
    }

    /// Sets a circular 95% confidence positional error for the [`Observation`].
    ///
    /// Ie. a gaussian error where 95% of the probability mass is contained within the given radius.
    ///
    /// See [`CovarianceMatrix::from_circular_95_confidence`].
    pub fn circular_95_confidence_error(
        self,
        radius: f64,
    ) -> Result<ObservationBuilder<CovarianceMatrix>, InvalidRadius> {
        let error = CovarianceMatrix::from_circular_95_confidence(radius)?;
        Ok(ObservationBuilder {
            position: self.position,
            error,
            context: self.context,
        })
    }
}

impl<E> ObservationBuilder<E> {
    /// Set the 'context' for the [`Observation`].
    ///
    /// See [`Observation::context`].
    pub const fn context(mut self, id: Uuid) -> Self {
        self.context = Some(id);
        self
    }
}

impl ObservationBuilder<CovarianceMatrix> {
    /// Finalise the builder and return an [`Observation`].
    pub const fn build(self) -> Observation {
        Observation {
            position: self.position,
            error: self.error,
            context: self.context,
        }
    }
}

/// Represents an observation of an object at a fixed location.
///
/// The observation has some measurement error associated with it.
///
/// # Example
///
/// Creating an observation with a circular 95% confidence error:
///
/// ```
/// use clique_fusion::Observation;
///
/// let obs = Observation::builder(10.0, 20.0)
///     .circular_95_confidence_error(3.0)
///     .unwrap()
///     .build();
///
/// assert_eq!(obs.x(), 10.0);
/// assert_eq!(obs.y(), 20.0);
/// ```
///
/// Creating an observation with a general error ellipse:
///
/// ```
/// use clique_fusion::{Observation, CovarianceMatrix};
///
/// let error = CovarianceMatrix::new(2.0, 1.5, 0.5).unwrap();
/// let obs = Observation::builder(5.0, -3.0)
///     .error(error)
///     .build();
///
/// let error = obs.error_covariance();
/// assert_eq!(error.xx(), 2.0);
/// ```
///
/// Adding context to an observation:
///
/// ```
/// use clique_fusion::{Observation, CovarianceMatrix};
/// use uuid::Uuid;
///
/// let context = Uuid::new_v4();
///
/// let error = CovarianceMatrix::identity();
/// let obs = Observation::builder(1.0, 1.0)
///     .error(error)
///     .context(context)
///     .build();
///
/// assert_eq!(obs.context(), Some(context));
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Observation {
    /// The position in 2D cartesian space of the observation
    position: Point2<f64>,

    /// The covariance matrix of the position error.
    ///
    /// 2x2 symmetric positive-definite
    ///
    /// A covariance matrix is used to express a general error ellipse.
    error: CovarianceMatrix,

    context: Option<Uuid>,
}

impl Observation {
    /// The position of the observation (x, y).
    #[must_use]
    pub fn position(&self) -> (f64, f64) {
        (self.position.x, self.position.y)
    }

    /// The x ordinate of the observation.
    #[must_use]
    pub fn x(&self) -> f64 {
        self.position.x
    }

    /// The y ordinate of the observation.
    #[must_use]
    pub fn y(&self) -> f64 {
        self.position.y
    }

    /// The covariance matrix representing the positional error ellipse associated with the observation.
    #[must_use]
    pub const fn error_covariance(&self) -> CovarianceMatrix {
        self.error
    }

    /// The 'context' for the observation.
    ///
    /// Observations in the same context are considered to have negligible relative error between them.
    ///
    /// For example:
    ///
    /// - separate observations marked in the same image
    /// - observations made within a single straight pass of a sensor on a moving platform
    #[must_use]
    pub const fn context(&self) -> Option<Uuid> {
        self.context
    }

    /// Construct a new observation
    pub const fn builder(x: f64, y: f64) -> ObservationBuilder<()> {
        ObservationBuilder::new(x, y)
    }

    /// Determines whether two observations are statistically compatible under the assumption
    /// that they represent independent measurements of the same underlying object.
    ///
    /// This method computes the squared Mahalanobis distance between the two observation positions,
    /// using the **sum of their covariance matrices** as the effective uncertainty model.
    ///
    /// This is statistically optimal for the case where each observation is modelled as a 2D
    /// Gaussian distribution with independent noise, and you're testing the hypothesis that both
    /// were drawn from the same true (but unknown) location.
    ///
    /// The combined covariance models the uncertainty in the difference between the two observations:
    ///     Cov[A − B] = Cov[A] + Cov[B]
    ///
    /// The Mahalanobis distance is then:
    ///     d² = (A − B)ᵀ ⋅ (Σ_A + Σ_B)⁻¹ ⋅ (A − B)
    ///
    /// If this distance is less than or equal to the given chi-squared threshold (typically based
    /// on 2 degrees of freedom for 2D), the observations are considered compatible.
    ///
    /// # Parameters
    /// - `other`: The observation to compare against.
    /// - `chi2_threshold`: The chi-squared threshold corresponding to the desired confidence level
    ///   (e.g., 5.991 for 95% confidence in 2D).
    ///
    /// # Returns
    /// `true` if the squared Mahalanobis distance between the observations is less than or equal
    /// to the threshold, indicating statistical compatibility; otherwise `false`.
    ///
    /// # See also
    /// - [Mahalanobis distance](https://en.wikipedia.org/wiki/Mahalanobis_distance)
    /// - [Chi-squared distribution](https://en.wikipedia.org/wiki/Chi-squared_distribution)
    #[must_use]
    pub fn is_compatible_with(&self, other: &Self, chi2_threshold: f64) -> bool {
        let delta = self.position - other.position;
        let combined_covariance = self.error + other.error;
        let d2 = mahalanobis_squared(delta, combined_covariance);
        d2 <= chi2_threshold
    }

    /// Computes a conservative maximum radius for spatial filtering to identify potentially
    /// compatible observations under the statistically optimal compatibility test.
    ///
    /// This radius corresponds to the maximum Mahalanobis distance consistent with the given chi-squared
    /// threshold, using the worst-case assumption about the other observation's error.
    ///
    /// The method assumes that the other observation's maximum variance does not exceed `max_other_variance`.
    /// The resulting radius ensures that no compatible observation is missed during spatial indexing.
    ///
    /// # Parameters
    /// - `chi2_threshold`: Chi-squared threshold for compatibility (e.g., 5.991 for 95% confidence in 2D)
    /// - `max_other_variance`: Assumed upper bound on the largest eigenvalue of the candidate observation's covariance
    #[must_use]
    pub(crate) fn max_compatibility_radius(
        &self,
        chi2_threshold: f64,
        max_other_variance: f64,
    ) -> f64 {
        let combined_max_variance = self.error.max_variance() + max_other_variance;
        (chi2_threshold * combined_max_variance).sqrt()
    }
}

/// Compute the squared [Mahalanobis distance](https://en.wikipedia.org/wiki/Mahalanobis_distance) between two points,
/// with covariance given by `covariance`.
fn mahalanobis_squared(delta: Vector2<f64>, covariance: CovarianceMatrix) -> f64 {
    covariance.safe_inverse().map_or(f64::INFINITY, |inv_cov| {
        let result = delta.transpose() * inv_cov * delta;
        result[(0, 0)]
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;
    use nalgebra::Matrix2;

    #[test]
    fn observation_with_circular_error_constructs_correctly() {
        let radius = 3.0;
        let actual_variance: Matrix2<f64> = Observation::builder(1.0, 2.0)
            .circular_95_confidence_error(radius)
            .unwrap()
            .build()
            .error_covariance()
            .into();
        let expected_variance = radius.powi(2) / CHI2_2D_CONFIDENCE_95;
        let expected = Matrix2::new(expected_variance, 0.0, 0.0, expected_variance);
        assert_relative_eq!(actual_variance, expected, epsilon = f64::EPSILON);
    }

    #[test]
    fn mahalanobis_distance_zero_for_same_position() {
        let cov = CovarianceMatrix::new_unchecked(2.0, 1.0, 0.0);
        let delta = Vector2::new(0.0, 0.0);
        let d2 = mahalanobis_squared(delta, cov);
        assert_relative_eq!(d2, 0.0, epsilon = f64::EPSILON);
    }

    #[test]
    fn mutual_compatibility_passes_for_close_points() {
        let cov = CovarianceMatrix::identity();
        let a = Observation::builder(0.0, 0.0).error(cov).build();
        let b = Observation::builder(1.0, 1.0).error(cov).build();

        // Mahalanobis distance squared should be 1 under identity covariances (combined is 2I)
        assert!(a.is_compatible_with(&b, CHI2_2D_CONFIDENCE_95));
    }

    #[test]
    fn mutual_compatibility_fails_for_distant_points() {
        let cov = CovarianceMatrix::identity();
        let a = Observation::builder(0.0, 0.0).error(cov).build();
        let b = Observation::builder(5.0, 5.0).error(cov).build();

        assert!(!a.is_compatible_with(&b, CHI2_2D_CONFIDENCE_95));
    }

    #[test]
    fn is_mutually_compatible_with_is_symmetric() {
        let cov = CovarianceMatrix::identity();

        let obs1 = Observation::builder(0.0, 0.0).error(cov).build();
        let obs2 = Observation::builder(1.0, 0.0).error(cov).build();

        let a_to_b = obs1.is_compatible_with(&obs2, CHI2_2D_CONFIDENCE_95);
        let b_to_a = obs2.is_compatible_with(&obs1, CHI2_2D_CONFIDENCE_95);

        assert_eq!(a_to_b, b_to_a); // function should be symmetric
    }
}
