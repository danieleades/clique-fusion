using System;
using System.Collections.Generic;
using System.Linq;
using System.Runtime.InteropServices;
using CliqueFusion.Native;

namespace CliqueFusion
{
    /// <summary>
    /// Provides predefined chi-squared thresholds for common confidence levels,
    /// useful when constructing a <see cref="CliqueIndex"/>.
    /// </summary>
    public static class CliqueThresholds
    {
        public static double Confidence90 => CliqueIndexNative.CliqueIndex_chi2_confidence_90();
        public static double Confidence95 => CliqueIndexNative.CliqueIndex_chi2_confidence_95();
        public static double Confidence99 => CliqueIndexNative.CliqueIndex_chi2_confidence_99();
    }

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

    /// <summary>
    /// Represents a maximal clique — a group of observation IDs that have been clustered together
    /// as mutually compatible under the chi-squared threshold.
    /// </summary>
    public record Clique(IReadOnlyList<Guid> ObservationIds);

    /// <summary>
    /// An index that accumulates observations and extracts maximal cliques from them.
    /// This class wraps the native Rust-based clique-fusion algorithm via FFI.
    /// </summary>
    public sealed class CliqueIndex : IDisposable
    {
        private IntPtr _handle;
        private bool _disposed;

        private static readonly int ObservationSize = Marshal.SizeOf<CliqueIndexNative.ObservationC>();

        /// <summary>
        /// Constructs an empty clique index with a given chi-squared threshold.
        /// </summary>
        public CliqueIndex(double chi2Threshold)
        {
            _handle = CliqueIndexNative.CliqueIndex_new(chi2Threshold);
            if (_handle == IntPtr.Zero)
                throw new InvalidOperationException("Failed to create CliqueIndex");
        }

        /// <summary>
        /// Constructs a clique index from an initial batch of observations.
        /// This is more efficient than inserting observations one at a time.
        /// </summary>
        public CliqueIndex(IEnumerable<Observation> observations, double chi2Threshold)
        {
            if (observations is null)
                throw new ArgumentNullException(nameof(observations));

            var observationList = observations.ToList();
            if (observationList.Count == 0)
            {
                _handle = CliqueIndexNative.CliqueIndex_new(chi2Threshold);
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

                    _handle = CliqueIndexNative.CliqueIndex_from_observations(
                        chi2Threshold, arrayPtr, (UIntPtr)nativeObs.Length);
                }
                finally
                {
                    Marshal.FreeHGlobal(arrayPtr);
                }
            }

            if (_handle == IntPtr.Zero)
                throw new InvalidOperationException("Failed to create CliqueIndex from observations");
        }

        /// <summary>
        /// Inserts a new observation into the index.
        /// </summary>
        public void Insert(Observation observation)
        {
            ThrowIfDisposed();

            var nativeObs = ToNative(observation);
            var obsPtr = Marshal.AllocHGlobal(ObservationSize);
            try
            {
                Marshal.StructureToPtr(nativeObs, obsPtr, false);
                CliqueIndexNative.CliqueIndex_insert(_handle, obsPtr);
            }
            finally
            {
                Marshal.FreeHGlobal(obsPtr);
            }
        }

        /// <summary>
        /// Retrieves the current set of maximal cliques, as grouped by the index.
        /// </summary>
        public IReadOnlyList<Clique> GetCliques()
        {
            ThrowIfDisposed();

            var cliquesPtr = CliqueIndexNative.CliqueIndex_cliques(_handle);
            if (cliquesPtr == IntPtr.Zero)
                return Array.Empty<Clique>();

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

        private static CliqueIndexNative.ObservationC ToNative(Observation o) =>
            new(o.Id, o.X, o.Y, o.CovarianceXX, o.CovarianceXY, o.CovarianceYY, o.Context);

        private void ThrowIfDisposed()
        {
            if (_disposed)
                throw new ObjectDisposedException(nameof(CliqueIndex));
        }

        /// <summary>
        /// Frees native resources associated with this instance.
        /// </summary>
        public void Dispose()
        {
            if (!_disposed && _handle != IntPtr.Zero)
            {
                CliqueIndexNative.CliqueIndex_free(_handle);
                _handle = IntPtr.Zero;
                _disposed = true;
            }
        }
    }
}
