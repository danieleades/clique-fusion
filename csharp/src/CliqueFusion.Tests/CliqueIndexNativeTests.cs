using System;
using System.Runtime.InteropServices;
using Xunit;
using CliqueFusion.Native;

namespace CliqueFusion.Tests
{
    public class CliqueIndexNativeTests
    {
        private static CliqueIndexNative.ObservationC CreateObservation(Guid id, double x, double y, double cov_xx, double cov_xy, double cov_yy, Guid? context = null)
        {
            return new CliqueIndexNative.ObservationC(id, x, y, cov_xx, cov_xy, cov_yy, context);
        }

        private static IntPtr ToNativePointer<T>(T managed) where T : struct
        {
            IntPtr ptr = Marshal.AllocHGlobal(Marshal.SizeOf<T>());
            Marshal.StructureToPtr(managed, ptr, false);
            return ptr;
        }

        [Fact]
        public void CanCreateEmptyIndex()
        {
            IntPtr index = CliqueIndexNative.CliqueIndex_new(5.99);
            Assert.NotEqual(IntPtr.Zero, index);
            CliqueIndexNative.CliqueIndex_free(index);
        }

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

        [Fact]
        public void CanCreateIndexFromMultipleObservations()
        {
            var obs1 = CreateObservation(Guid.Parse("f01073e1-ebff-4417-a082-2279043a44a7"), 1.0, 2.0, 1.0, 0.0, 1.0);
            var obs2 = CreateObservation(Guid.Parse("91ed9e59-60f9-4c3c-a1fa-21d644e78b4b"), 1.2, 2.1, 1.0, 0.0, 1.0);
            var obs3 = CreateObservation(Guid.Parse("08f35c48-525b-4076-bf4a-6e8943bc3c4b"), 5.0, 5.0, 1.0, 0.0, 1.0);

            int size = Marshal.SizeOf<CliqueIndexNative.ObservationC>();
            IntPtr arrayPtr = Marshal.AllocHGlobal(size * 3);
            Marshal.StructureToPtr(obs1, arrayPtr + size * 0, false);
            Marshal.StructureToPtr(obs2, arrayPtr + size * 1, false);
            Marshal.StructureToPtr(obs3, arrayPtr + size * 2, false);

            IntPtr index = CliqueIndexNative.CliqueIndex_from_observations(5.99, arrayPtr, (UIntPtr)3);
            Assert.NotEqual(IntPtr.Zero, index);

            IntPtr cliques = CliqueIndexNative.CliqueIndex_cliques(index);
            Assert.NotEqual(IntPtr.Zero, cliques);

            CliqueIndexNative.CliqueSetC_free(cliques);
            CliqueIndexNative.CliqueIndex_free(index);
            Marshal.FreeHGlobal(arrayPtr);
        }

        [Fact]
        public void FreeingNullCliqueSetDoesNotCrash()
        {
            // Should be safe (no-op)
            CliqueIndexNative.CliqueSetC_free(IntPtr.Zero);
        }

        [Fact]
        public void FreeingNullIndexDoesNotCrash()
        {
            // Should be safe (no-op)
            CliqueIndexNative.CliqueIndex_free(IntPtr.Zero);
        }
    }
}
