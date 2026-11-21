package main

import (
	"fmt"
	"os"
	"strconv"
	"strings"
)

func num(n int64) int64 {
	var s int64
	for i := int64(1); i <= n; i++ {
		s = (s*31 + i) % 1000000007
	}
	return s
}

func fib(n int64) int64 {
	if n < 2 {
		return n
	}
	return fib(n-1) + fib(n-2)
}

func listb(n int64) int64 {
	v := make([]int64, 0, n)
	for i := int64(0); i < n; i++ {
		v = append(v, i)
	}
	var s int64
	for _, x := range v {
		s += x
	}
	return s
}

func indexb(n int64) int64 {
	v := make([]int64, 0, n)
	for i := int64(0); i < n; i++ {
		v = append(v, i)
	}
	var s int64
	for r := 0; r < 3000; r++ {
		for _, x := range v {
			s = (s*31 + x) % 1000000007
		}
	}
	return s
}

func mapb(n int64) int64 {
	m := make(map[int64]int64, n)
	for i := int64(0); i < n; i++ {
		m[i] = i * 2
	}
	var s int64
	for i := int64(0); i < n; i++ {
		s += m[i]
	}
	return s
}

func concatb(n int64) int64 {
	parts := []string{"ab", "cd", "ef"}
	var k int64
	var b strings.Builder
	for i := int64(0); i < n; i++ {
		b.Reset()
		b.Grow(6)
		for _, p := range parts {
			b.WriteString(p)
		}
		k += int64(len(b.String()))
	}
	return k
}

func main() {
	mode := "num"
	if len(os.Args) > 1 {
		mode = os.Args[1]
	}
	var n int64
	if len(os.Args) > 2 {
		n, _ = strconv.ParseInt(os.Args[2], 10, 64)
	}
	if mode == "none" {
		fmt.Println(0)
		return
	}
	var r int64
	switch mode {
	case "num":
		r = num(n)
	case "fib":
		r = fib(n)
	case "list":
		r = listb(n)
	case "index":
		r = indexb(n)
	case "map":
		r = mapb(n)
	case "concat":
		r = concatb(n)
	}
	fmt.Println(r)
}
