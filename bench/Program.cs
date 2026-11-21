using System;
using System.Collections.Generic;
using System.Text;

class Program
{
    static long Num(long n)
    {
        long s = 0;
        for (long i = 1; i <= n; i++) s = (s * 31 + i) % 1000000007L;
        return s;
    }

    static long Fib(long n)
    {
        return n < 2 ? n : Fib(n - 1) + Fib(n - 2);
    }

    static long ListB(long n)
    {
        var v = new List<long>((int)n);
        for (long i = 0; i < n; i++) v.Add(i);
        long s = 0;
        foreach (long x in v) s += x;
        return s;
    }

    static long IndexB(long n)
    {
        var v = new List<long>((int)n);
        for (long i = 0; i < n; i++) v.Add(i);
        long s = 0;
        for (long r = 0; r < 3000; r++)
            foreach (long x in v) s = (s * 31 + x) % 1000000007L;
        return s;
    }

    static long MapB(long n)
    {
        var m = new Dictionary<long, long>((int)(n * 2));
        for (long i = 0; i < n; i++) m[i] = i * 2;
        long s = 0;
        for (long i = 0; i < n; i++) s += m[i];
        return s;
    }

    static long ConcatB(long n)
    {
        string[] parts = { "ab", "cd", "ef" };
        long k = 0;
        for (long i = 0; i < n; i++)
        {
            var sb = new StringBuilder(6);
            foreach (string p in parts) sb.Append(p);
            k += sb.Length;
        }
        return k;
    }

    static void Main(string[] args)
    {
        string mode = args.Length > 0 ? args[0] : "num";
        long n = args.Length > 1 ? long.Parse(args[1]) : 0;
        if (mode == "none")
        {
            Console.WriteLine(0);
            return;
        }
        long r = 0;
        if (mode == "num") r = Num(n);
        else if (mode == "fib") r = Fib(n);
        else if (mode == "list") r = ListB(n);
        else if (mode == "index") r = IndexB(n);
        else if (mode == "map") r = MapB(n);
        else if (mode == "concat") r = ConcatB(n);
        Console.WriteLine(r);
    }
}
