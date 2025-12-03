// <copyright file="CliqueIndex.cs" company="Daniel Eades">
// Copyright (c) Daniel Eades. All rights reserved.
// </copyright>

namespace CliqueFusion
{
    using System;
    using System.Collections.Generic;
    using System.Linq;
    using System.Runtime.InteropServices;
    using CliqueFusion.Native;

    /// <summary>
    /// An index that accumulates observations and extracts maximal cliques from them.
    /// This class wraps the native Rust-based clique-fusion algorithm via FFI.
    /// </summary>
    public sealed class CliqueIndex : IDisposable
    {
        private static readonly int ObservationSize = Marshal.SizeOf<CliqueIndexNative.ObservationC>();
        private IntPtr handle;
        private bool disposed;

        /// <summary>
        /// Initializes a new instance of the <see cref="CliqueIndex"/> class
        /// with a specified chi-squared threshold.
        /// </summary>
        /// <param name="chi2Threshold">The chi-squared threshold used for clique compatibility.</param>
        public CliqueIndex(double chi2Threshold)
        {
            this.handle = CliqueIndexNative.CliqueIndex_new(chi2Threshold);
            if (this.handle == IntPtr.Zero)
            {
                throw new InvalidOperationException("Failed to create CliqueIndex");
            }
        }

        /// <summary>
        /// Initializes a new instance of the <see cref="CliqueIndex"/> class
        /// using an initial batch of observations.
        /// </summary>
        /// <param name="observations">The observations to initialize the index with.</param>
        /// <param name="chi2Threshold">The chi-squared threshold used for clique compatibility.</param>
        public CliqueIndex(IEnumerable<Observation> observations, double chi2Threshold)
        {
            if (observations is null)
            {
                throw new ArgumentNullException(nameof(observations));
            }

            var observationList = observations.ToList();
            if (observationList.Count == 0)
            {
                this.handle = CliqueIndexNative.CliqueIndex_new(chi2Threshold);
            }
            else
            {
                var nativeObs = observationList.Select(ToNative).ToArray();
                var arrayPtr = Marshal.AllocHGlobal(ObservationSize * nativeObs.Length);

                try
                {
                    for (int i = 0; i < nativeObs.Length; i++)
                    {
                        var ptr = IntPtr.Add(arrayPtr, i * ObservationSize);
                        Marshal.StructureToPtr(nativeObs[i], ptr, false);
                    }

                    this.handle = CliqueIndexNative.CliqueIndex_from_observations(
                        chi2Threshold, arrayPtr, (UIntPtr)nativeObs.Length);
                }
                finally
                {
                    Marshal.FreeHGlobal(arrayPtr);
                }
            }

            if (this.handle == IntPtr.Zero)
            {
                throw new InvalidOperationException("Failed to create CliqueIndex from observations");
            }
        }

        /// <summary>
        /// Inserts a new observation into the index.
        /// </summary>
        /// <param name="observation">The observation to insert.</param>
        public void Insert(Observation observation)
        {
            this.ThrowIfDisposed();

            var nativeObs = ToNative(observation);
            var obsPtr = Marshal.AllocHGlobal(ObservationSize);
            try
            {
                Marshal.StructureToPtr(nativeObs, obsPtr, false);
                CliqueIndexNative.CliqueIndex_insert(this.handle, obsPtr);
            }
            finally
            {
                Marshal.FreeHGlobal(obsPtr);
            }
        }

        /// <summary>
        /// Retrieves the current set of maximal cliques.
        /// </summary>
        /// <returns>A list of cliques containing observation IDs.</returns>
        public IReadOnlyList<Clique> GetCliques()
        {
            this.ThrowIfDisposed();

            var cliquesPtr = CliqueIndexNative.CliqueIndex_cliques(this.handle);
            if (cliquesPtr == IntPtr.Zero)
            {
                return Array.Empty<Clique>();
            }

            try
            {
                var cliqueSet = Marshal.PtrToStructure<CliqueIndexNative.CliqueSetC>(cliquesPtr);
                var cliques = new List<Clique>();

                for (int i = 0; i < (int)cliqueSet.len; i++)
                {
                    var cliquePtr = IntPtr.Add(cliqueSet.cliques, i * Marshal.SizeOf<CliqueIndexNative.CliqueC>());
                    var clique = Marshal.PtrToStructure<CliqueIndexNative.CliqueC>(cliquePtr);

                    var ids = new List<Guid>();
                    for (int j = 0; j < (int)clique.len; j++)
                    {
                        var uuidPtr = IntPtr.Add(clique.uuids, j * 16);
                        ids.Add(Marshal.PtrToStructure<Guid>(uuidPtr));
                    }

                    cliques.Add(new Clique(ids));
                }

                return cliques;
            }
            finally
            {
                CliqueIndexNative.CliqueSetC_free(cliquesPtr);
            }
        }

        /// <summary>
        /// Releases all native resources associated with this instance.
        /// </summary>
        public void Dispose()
        {
            if (!this.disposed && this.handle != IntPtr.Zero)
            {
                CliqueIndexNative.CliqueIndex_free(this.handle);
                this.handle = IntPtr.Zero;
                this.disposed = true;
            }
        }

        private static CliqueIndexNative.ObservationC ToNative(Observation o) =>
            new(o.Id, o.X, o.Y, o.CovarianceXX, o.CovarianceXY, o.CovarianceYY, o.Context);

        private void ThrowIfDisposed()
        {
            if (this.disposed)
            {
                throw new ObjectDisposedException(nameof(CliqueIndex));
            }
        }
    }
}
