import java.util.HashMap;

public class J {
    static long num(long n) {
        long s = 0;
        for (long i = 1; i <= n; i++) s = (s * 31 + i) % 1000000007L;
        return s;
    }

    static long fib(long n) {
        return n < 2 ? n : fib(n - 1) + fib(n - 2);
    }

    static long listb(long n) {
        long[] v = new long[(int) n];
        for (int i = 0; i < n; i++) v[i] = i;
        long s = 0;
        for (long x : v) s += x;
        return s;
    }

    static long indexb(long n) {
        long[] v = new long[(int) n];
        for (int i = 0; i < n; i++) v[i] = i;
        long s = 0;
        for (int r = 0; r < 3000; r++)
            for (long x : v) s = (s * 31 + x) % 1000000007L;
        return s;
    }

    static long mapb(long n) {
        HashMap<Long, Long> m = new HashMap<>((int) (n * 2));
        for (long i = 0; i < n; i++) m.put(i, i * 2);
        long s = 0;
        for (long i = 0; i < n; i++) s += m.get(i);
        return s;
    }

    static long concatb(long n) {
        String[] parts = { "ab", "cd", "ef" };
        long k = 0;
        for (long i = 0; i < n; i++) {
            StringBuilder sb = new StringBuilder(6);
            for (String p : parts) sb.append(p);
            k += sb.length();
        }
        return k;
    }

    public static void main(String[] args) {
        String mode = args.length > 0 ? args[0] : "num";
        long n = args.length > 1 ? Long.parseLong(args[1]) : 0;
        if (mode.equals("none")) {
            System.out.println(0);
            return;
        }
        long r = 0;
        if (mode.equals("num")) r = num(n);
        else if (mode.equals("fib")) r = fib(n);
        else if (mode.equals("list")) r = listb(n);
        else if (mode.equals("index")) r = indexb(n);
        else if (mode.equals("map")) r = mapb(n);
        else if (mode.equals("concat")) r = concatb(n);
        System.out.println(r);
    }
}
