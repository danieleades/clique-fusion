// <copyright file="CliqueIndexNativeTests.cs" company="Daniel Eades">
// Copyright (c) Daniel Eades. All rights reserved.
// </copyright>

namespace CliqueFusion.Tests
{
    using System.Runtime.InteropServices;
    using CliqueFusion.Native;
    using Xunit;

    /// <summary>
    /// Tests for the low-level native FFI (foreign function interface) of the CliqueIndex implementation.
    /// These tests validate correct memory handling, pointer marshalling, and basic interop behavior.
    /// </summary>
    public class CliqueIndexNativeTests
    {
        /// <summary>
        /// Verifies that an empty index can be created and is non-null.
        /// </summary>
        [Fact]
        public void CanCreateEmptyIndex()
        {
            IntPtr index = CliqueIndexNative.CliqueIndex_new(5.99);
            Assert.NotEqual(IntPtr.Zero, index);
            CliqueIndexNative.CliqueIndex_free(index);
        }

        /// <summary>
        /// Verifies that a single observation can be inserted into the index and cliques can be retrieved.
        /// </summary>
        [Fact]
        public void CanInsertSingleObservation()
        {
            var obs = CreateObservation(Guid.NewGuid(), 1.0, 2.0, 1.0, 0.0, 1.0);
            var obsPtr = ToNativePointer(obs);

            IntPtr index = CliqueIndexNative.CliqueIndex_new(5.99);
            CliqueIndexNative.CliqueIndex_insert(index, obsPtr);

            IntPtr cliques = CliqueIndexNative.CliqueIndex_cliques(index);
            Assert.NotEqual(IntPtr.Zero, cliques);

            CliqueIndexNative.CliqueSetC_free(cliques);
            CliqueIndexNative.CliqueIndex_free(index);
            Marshal.FreeHGlobal(obsPtr);
        }

        /// <summary>
        /// Verifies that an index can be created from an array of multiple observations,
        /// and that the resulting cliques pointer is valid.
        /// </summary>
        [Fact]
        public void CanCreateIndexFromMultipleObservations()
        {
            var obs1 = CreateObservation(Guid.Parse("f01073e1-ebff-4417-a082-2279043a44a7"), 1.0, 2.0, 1.0, 0.0, 1.0);
            var obs2 = CreateObservation(Guid.Parse("91ed9e59-60f9-4c3c-a1fa-21d644e78b4b"), 1.2, 2.1, 1.0, 0.0, 1.0);
            var obs3 = CreateObservation(Guid.Parse("08f35c48-525b-4076-bf4a-6e8943bc3c4b"), 5.0, 5.0, 1.0, 0.0, 1.0);

            int size = Marshal.SizeOf<CliqueIndexNative.ObservationC>();
            IntPtr arrayPtr = Marshal.AllocHGlobal(size * 3);
            Marshal.StructureToPtr(obs1, arrayPtr + (size * 0), false);
            Marshal.StructureToPtr(obs2, arrayPtr + (size * 1), false);
            Marshal.StructureToPtr(obs3, arrayPtr + (size * 2), false);

            IntPtr index = CliqueIndexNative.CliqueIndex_from_observations(5.99, arrayPtr, (UIntPtr)3);
            Assert.NotEqual(IntPtr.Zero, index);

            IntPtr cliques = CliqueIndexNative.CliqueIndex_cliques(index);
            Assert.NotEqual(IntPtr.Zero, cliques);

            CliqueIndexNative.CliqueSetC_free(cliques);
            CliqueIndexNative.CliqueIndex_free(index);
            Marshal.FreeHGlobal(arrayPtr);
        }

        /// <summary>
        /// Verifies that freeing a null clique set pointer is safe and does not crash.
        /// </summary>
        [Fact]
        public void FreeingNullCliqueSetDoesNotCrash()
        {
            CliqueIndexNative.CliqueSetC_free(IntPtr.Zero);
        }

        /// <summary>
        /// Verifies that freeing a null index pointer is safe and does not crash.
        /// </summary>
        [Fact]
        public void FreeingNullIndexDoesNotCrash()
        {
            CliqueIndexNative.CliqueIndex_free(IntPtr.Zero);
        }

        /// <summary>
        /// Creates a native-compatible observation struct with optional context.
        /// </summary>
        private static CliqueIndexNative.ObservationC CreateObservation(Guid id, double x, double y, double cov_xx, double cov_xy, double cov_yy, Guid? context = null)
        {
            return new CliqueIndexNative.ObservationC(id, x, y, cov_xx, cov_xy, cov_yy, context);
        }

        /// <summary>
        /// Allocates a native pointer for a given struct and copies the managed data to unmanaged memory.
        /// </summary>
        private static IntPtr ToNativePointer<T>(T managed)
            where T : struct
        {
            IntPtr ptr = Marshal.AllocHGlobal(Marshal.SizeOf<T>());
            Marshal.StructureToPtr(managed, ptr, false);
            return ptr;
        }
    }
}
