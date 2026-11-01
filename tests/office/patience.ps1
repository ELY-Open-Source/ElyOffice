# Retries COM calls busy Office rejects, two minutes.
if ([Threading.Thread]::CurrentThread.GetApartmentState() -ne 'STA') { throw 'Office automation needs an STA thread' }
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
[ComImport, Guid("00000016-0000-0000-C000-000000000046"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
interface IOleMessageFilter {
    [PreserveSig] int HandleInComingCall(int type, IntPtr caller, int ticks, IntPtr info);
    [PreserveSig] int RetryRejectedCall(IntPtr callee, int ticks, int reject);
    [PreserveSig] int MessagePending(IntPtr callee, int ticks, int pending);
}
public class Patience : IOleMessageFilter {
    [DllImport("ole32.dll")]
    static extern int CoRegisterMessageFilter(IOleMessageFilter next, out IOleMessageFilter old);
    public static void Register() {
        IOleMessageFilter old;
        Marshal.ThrowExceptionForHR(CoRegisterMessageFilter(new Patience(), out old));
    }
    int IOleMessageFilter.HandleInComingCall(int type, IntPtr caller, int ticks, IntPtr info) { return 0; }
    int IOleMessageFilter.RetryRejectedCall(IntPtr callee, int ticks, int reject) {
        return reject == 2 && ticks < 120000 ? 100 : -1;
    }
    int IOleMessageFilter.MessagePending(IntPtr callee, int ticks, int pending) { return 2; }
}
'@
[Patience]::Register()
