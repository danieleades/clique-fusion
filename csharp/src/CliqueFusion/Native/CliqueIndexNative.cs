// <copyright file="CliqueIndexNative.cs" company="Daniel Eades">
// Copyright (c) Daniel Eades. All rights reserved.
// </copyright>

namespace CliqueFusion.Native
{
    using System;
    using System.Diagnostics.CodeAnalysis;
    using System.Runtime.InteropServices;

    /// <summary>
    /// P/Invoke declarations for the clique-fusion FFI library.
    /// </summary>
    internal static class CliqueIndexNative
    {
        private const string DllName = "clique_fusion_ffi";

        /// <summary>
        /// Gets the chi-squared threshold for 90% confidence.
        /// </summary>
        /// <returns>The chi-squared value corresponding to 90% confidence.</returns>
        [DllImport(DllName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern double CliqueIndex_chi2_confidence_90();

        /// <summary>
        /// Gets the chi-squared threshold for 95% confidence.
        /// </summary>
        /// <returns>The chi-squared value corresponding to 95% confidence.</returns>
        [DllImport(DllName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern double CliqueIndex_chi2_confidence_95();

        /// <summary>
        /// Gets the chi-squared threshold for 99% confidence.
        /// </summary>
        /// <returns>The chi-squared value corresponding to 99% confidence.</returns>
        [DllImport(DllName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern double CliqueIndex_chi2_confidence_99();

        /// <summary>
        /// Creates a new clique index.
        /// </summary>
        /// <param name="chi2">Chi-squared threshold for clustering.</param>
        /// <returns>A pointer to the new CliqueIndex instance.</returns>
        [DllImport(DllName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern IntPtr CliqueIndex_new(double chi2);

        /// <summary>
        /// Creates a clique index from an array of observations.
        /// </summary>
        /// <param name="chi2">Chi-squared threshold for clustering.</param>
        /// <param name="observations">Pointer to an array of observations.</param>
        /// <param name="len">Number of observations.</param>
        /// <returns>A pointer to the new CliqueIndex instance.</returns>
        [DllImport(DllName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern IntPtr CliqueIndex_from_observations(
            double chi2,
            IntPtr observations,
            UIntPtr len);

        /// <summary>
        /// Inserts an observation into an existing clique index.
        /// </summary>
        /// <param name="index">Pointer to the clique index.</param>
        /// <param name="observation">Pointer to the observation.</param>
        [DllImport(DllName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern void CliqueIndex_insert(IntPtr index, IntPtr observation);

        /// <summary>
        /// Gets the cliques from a clique index.
        /// </summary>
        /// <param name="index">Pointer to the clique index.</param>
        /// <returns>A pointer to a CliqueSetC struct.</returns>
        [DllImport(DllName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern IntPtr CliqueIndex_cliques(IntPtr index);

        /// <summary>
        /// Frees a clique set returned by the index.
        /// </summary>
        /// <param name="ptr">Pointer to the CliqueSetC.</param>
        [DllImport(DllName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern void CliqueSetC_free(IntPtr ptr);

        /// <summary>
        /// Frees the clique index.
        /// </summary>
        /// <param name="ptr">Pointer to the CliqueIndex.</param>
        [DllImport(DllName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern void CliqueIndex_free(IntPtr ptr);

        /// <summary>
        /// C-compatible struct for observations.
        /// </summary>
        [StructLayout(LayoutKind.Sequential)]
        [SuppressMessage("StyleCop.CSharp.NamingRules", "SA1307", Justification = "Interop naming")]
        [SuppressMessage("StyleCop.CSharp.NamingRules", "SA1310", Justification = "Interop naming")]
        internal struct ObservationC
        {
            /// <summary>Observation UUID (16 bytes).</summary>
            [MarshalAs(UnmanagedType.ByValArray, SizeConst = 16)]
            public byte[] id;

            /// <summary>X position.</summary>
            public double x;

            /// <summary>Y position.</summary>
            public double y;

            /// <summary>X-X covariance.</summary>
            public double cov_xx;

            /// <summary>X-Y covariance.</summary>
            public double cov_xy;

            /// <summary>Y-Y covariance.</summary>
            public double cov_yy;

            /// <summary>Context UUID (16 bytes).</summary>
            [MarshalAs(UnmanagedType.ByValArray, SizeConst = 16)]
            public byte[] context;

            /// <summary>
            /// Initializes a new instance of the <see cref="ObservationC"/> struct.
            /// </summary>
            /// <param name="id">The observation ID.</param>
            /// <param name="x">X position.</param>
            /// <param name="y">Y position.</param>
            /// <param name="cov_xx">Covariance X-X.</param>
            /// <param name="cov_xy">Covariance X-Y.</param>
            /// <param name="cov_yy">Covariance Y-Y.</param>
            /// <param name="context">Optional context ID.</param>
            public ObservationC(Guid id, double x, double y, double cov_xx, double cov_xy, double cov_yy, Guid? context = null)
            {
                this.id = id.ToByteArray();
                this.x = x;
                this.y = y;
                this.cov_xx = cov_xx;
                this.cov_xy = cov_xy;
                this.cov_yy = cov_yy;
                this.context = context?.ToByteArray() ?? new byte[16];
            }
        }

        /// <summary>
        /// C-compatible clique representation.
        /// </summary>
        [StructLayout(LayoutKind.Sequential)]
        [SuppressMessage("StyleCop.CSharp.NamingRules", "SA1307", Justification = "Interop naming")]
        internal struct CliqueC
        {
            /// <summary>Pointer to UUID array.</summary>
            public IntPtr uuids;

            /// <summary>Length of the UUID array.</summary>
            public UIntPtr len;
        }

        /// <summary>
        /// C-compatible clique set representation.
        /// </summary>
        [StructLayout(LayoutKind.Sequential)]
        [SuppressMessage("StyleCop.CSharp.NamingRules", "SA1307", Justification = "Interop naming")]
        internal struct CliqueSetC
        {
            /// <summary>Pointer to clique array.</summary>
            public IntPtr cliques;

            /// <summary>Length of the clique array.</summary>
            public UIntPtr len;
        }
    }
}
