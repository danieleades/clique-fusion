using System;
using System.Runtime.InteropServices;

namespace CliqueFusion.Native
{
    /// <summary>
    /// P/Invoke declarations for the clique-fusion FFI library
    /// </summary>
    internal static class CliqueIndexNative
    {
        private const string DllName = "clique_fusion_ffi";

        [DllImport(DllName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern double CliqueIndex_chi2_confidence_90();

        [DllImport(DllName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern double CliqueIndex_chi2_confidence_95();

        [DllImport(DllName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern double CliqueIndex_chi2_confidence_99();

        [StructLayout(LayoutKind.Sequential)]
        internal struct ObservationC
        {
            [MarshalAs(UnmanagedType.ByValArray, SizeConst = 16)]
            public byte[] id;
            public double x;
            public double y;
            public double cov_xx;
            public double cov_xy;
            public double cov_yy;
            [MarshalAs(UnmanagedType.ByValArray, SizeConst = 16)]
            public byte[] context;

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

        [StructLayout(LayoutKind.Sequential)]
        internal struct CliqueC
        {
            public IntPtr uuids;
            public UIntPtr len;
        }

        [StructLayout(LayoutKind.Sequential)]
        internal struct CliqueSetC
        {
            public IntPtr cliques;
            public UIntPtr len;
        }

        [DllImport(DllName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern IntPtr CliqueIndex_new(double chi2);

        [DllImport(DllName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern IntPtr CliqueIndex_from_observations(
            double chi2,
            IntPtr observations,
            UIntPtr len);

        [DllImport(DllName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern void CliqueIndex_insert(IntPtr index, IntPtr observation);

        [DllImport(DllName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern IntPtr CliqueIndex_cliques(IntPtr index);

        [DllImport(DllName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern void CliqueSetC_free(IntPtr ptr);

        [DllImport(DllName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern void CliqueIndex_free(IntPtr ptr);
    }
}
