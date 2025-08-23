// <copyright file="Observation.cs" company="Daniel Eades">
// Copyright (c) Daniel Eades. All rights reserved.
// </copyright>

namespace CliqueFusion
{
    using System;

    /// <summary>
    /// 2D observation with position, covariance and optional context.
    ///
    /// Observations sharing the same context are never fused into a clique.
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
