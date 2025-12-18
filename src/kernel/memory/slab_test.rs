#[cfg(test)]
mod tests {
    use super::super::slab::SlabAllocator;

    #[test]
    fn slab_alloc_free_reuse() {
        let mut slab = SlabAllocator::new();

        unsafe {
            let p1 = slab.alloc(32);
            assert!(!p1.is_null());

            slab.dealloc(p1, 32);

            let p2 = slab.alloc(32);
            assert!(!p2.is_null());

            // Le slab doit réutiliser le même bloc
            assert_eq!(p1, p2);
        }
    }
}
