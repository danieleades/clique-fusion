// <copyright file="CliqueThresholds.cs" company="Daniel Eades">
// Copyright (c) Daniel Eades. All rights reserved.
// </copyright>

namespace CliqueFusion
{
    using CliqueFusion.Native;

    /// <summary>
    /// Chi-squared thresholds for common confidence levels.
    /// </summary>
    public static class CliqueThresholds
    {
        /// <summary>
        /// Gets the chi-squared threshold corresponding to 90% confidence.
        /// </summary>
        public static double Confidence90 => CliqueIndexNative.CliqueIndex_chi2_confidence_90();

        /// <summary>
        /// Gets the chi-squared threshold corresponding to 95% confidence.
        /// </summary>
        public static double Confidence95 => CliqueIndexNative.CliqueIndex_chi2_confidence_95();

        /// <summary>
        /// Gets the chi-squared threshold corresponding to 99% confidence.
        /// </summary>
        public static double Confidence99 => CliqueIndexNative.CliqueIndex_chi2_confidence_99();
    }
}
