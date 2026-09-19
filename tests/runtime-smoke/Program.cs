using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Text;
using System.Threading;
using LiveSplit.AutoSplittingRuntime;

// Use the installed LiveSplit runtime with simulated timer callbacks. The real timer is untouched.
class Program
{
    [DllImport("kernel32", CharSet = CharSet.Unicode, SetLastError = true)]
    static extern IntPtr LoadLibrary(string path);
    static string Text(IntPtr p, UIntPtr len) { byte[] b = new byte[(int)len]; Marshal.Copy(p, b, 0, b.Length); return Encoding.UTF8.GetString(b); }
    static void Main(string[] args)
    {
        if (args.Length < 3) throw new ArgumentException("Usage: RuntimeSmoke <wasm> <seconds> <LiveSplit directory>");
        string nativePath = System.IO.Path.Combine(System.IO.Path.GetFullPath(args[2]), "Components", IntPtr.Size == 8 ? "x64" : "x86", "asr_capi.dll");
#if NETFRAMEWORK
        if (LoadLibrary(nativePath) == IntPtr.Zero) throw new Exception("Cannot load ASR: " + Marshal.GetLastWin32Error());
#else
        NativeLibrary.SetDllImportResolver(typeof(Runtime).Assembly, (name, assembly, path) =>
            name == "asr_capi" ? NativeLibrary.Load(nativePath) : IntPtr.Zero);
#endif
        var vars = new Dictionary<string, string>();
        StateDelegate state = () => 0;
        IndexDelegate index = () => 0;
        SegmentSplittedDelegate segment = i => 0;
        Action start = () => Console.WriteLine("TIMER start (simulated)");
        Action split = () => Console.WriteLine("TIMER split (simulated)");
        Action nop = () => {};
        SetGameTimeDelegate gameTime = t => {};
        SetCustomVariableDelegate variable = (np, nl, vp, vl) => {
            string n = Text(np, nl), v = Text(vp, vl);
            if (!vars.TryGetValue(n, out string old) || old != v) { Console.WriteLine(n + " = " + v); vars[n] = v; }
        };
        LogDelegate log = (p, n) => Console.WriteLine(Text(p, n));
        using (var runtime = new Runtime(System.IO.Path.GetFullPath(args[0]), null, state, index, segment,
            start, split, nop, nop, nop, gameTime, nop, nop, variable, log))
        {
            runtime.SettingsMapSetBool("il_mode", true);
            var clock = Stopwatch.StartNew();
            int seconds = args.Length > 1 ? int.Parse(args[1]) : 5;
            int ticks = 0;
            while (clock.Elapsed.TotalSeconds < seconds)
            {
                if (!runtime.Step()) throw new Exception("WASM runtime Step failed");
                ticks++;
                Thread.Sleep(16);
            }
            Console.WriteLine("Runtime smoke passed: " + ticks + " ticks, " + clock.ElapsedMilliseconds + " ms");
            using (var widgets = runtime.GetSettingsWidgets())
            {
                for (ulong i = 0; i < widgets.GetLength(); i++)
                    Console.WriteLine("Setting [" + widgets.GetType(i) + "] " + widgets.GetKey(i) + ": " + widgets.GetDescription(i));
            }
        }
        GC.KeepAlive(state); GC.KeepAlive(index); GC.KeepAlive(segment); GC.KeepAlive(start);
        GC.KeepAlive(split); GC.KeepAlive(nop); GC.KeepAlive(gameTime); GC.KeepAlive(variable); GC.KeepAlive(log);
    }
}
