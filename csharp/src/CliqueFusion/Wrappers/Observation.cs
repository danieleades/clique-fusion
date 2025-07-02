// <copyright file="Observation.cs" company="Daniel Eades">
// Copyright (c) Daniel Eades. All rights reserved.
// </copyright>

namespace CliqueFusion
{
    using System;

    /// <summary>
    /// Represents a 2D observation with a unique identifier, position, uncertainty (covariance), and optional context.
    ///
    /// Observations within the same context are never merged into cliques. The context groups
    /// observations that are known to be distinct — for example, simultaneous detections by a single sensor.
    ///
    /// Observations with different contexts, or no context, may be fused into cliques if they are consistent
    /// under the chi-squared test with the given uncertainty model.
    /// </summary>
    public record Observation(
        Guid Id,
        double X,
        double Y,
        double CovarianceXX,
        double CovarianceXY,
        double CovarianceYY,
        Guid? Context = null);
}
