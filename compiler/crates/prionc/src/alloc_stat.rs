




use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};


static 次数: AtomicUsize = AtomicUsize::new(0);

static 字节: AtomicUsize = AtomicUsize::new(0);

static 释放: AtomicUsize = AtomicUsize::new(0);


pub struct 计数;

unsafe impl GlobalAlloc for 计数 {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        次数.fetch_add(1, Ordering::Relaxed);
        字节.fetch_add(l.size(), Ordering::Relaxed);
        System.alloc(l)
    }

    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        释放.fetch_add(1, Ordering::Relaxed);
        System.dealloc(p, l)
    }

    unsafe fn realloc(&self, p: *mut u8, l: Layout, 新: usize) -> *mut u8 {
        次数.fetch_add(1, Ordering::Relaxed);
        字节.fetch_add(新, Ordering::Relaxed);
        System.realloc(p, l, 新)
    }
}


pub fn 报告() {
    if std::env::var("PRION_ALLOC_STAT").is_err() {
        return;
    }
    let a = 次数.load(Ordering::Relaxed);
    let b = 字节.load(Ordering::Relaxed);
    let d = 释放.load(Ordering::Relaxed);
    eprintln!(
        "[分配统计] 次数 {}，字节 {}（{:.1} MB），释放 {}",
        a,
        b,
        b as f64 / 1048576.0,
        d
    );
}
