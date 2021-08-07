//! A slab, essentially the same as the slab crate, but with two key
//! differences: the actual items are stored contiguously instead of behind an
//! enum, and the handles returned use NonzeroU32 to allow for the niche
//! optimization and to save significant space.
//!
//! In debug builds, this slab additionally checks that handles are always used
//! with the right slab, and that double frees never occur.
use std::mem::MaybeUninit;
use std::num::NonZeroU32;

#[derive(Debug)]
pub struct SlabHandle<T> {
    slot: NonZeroU32,
    #[cfg(any(debug_assertions, test))]
    slab_tag: usize,
    #[cfg(any(debug_assertions, test))]
    slab_version: usize,
    _data: std::marker::PhantomData<*const T>,
}

#[cfg(any(debug_assertions, test))]
impl<T> Clone for SlabHandle<T> {
    fn clone(&self) -> SlabHandle<T> {
        SlabHandle {
            slot: self.slot,
            slab_tag: self.slab_tag,
            slab_version: self.slab_version,
            _data: std::marker::PhantomData,
        }
    }
}

#[cfg(not(any(debug_assertions, test)))]
impl<T> Clone for SlabHandle<T> {
    fn clone(&self) -> SlabHandle<T> {
        SlabHandle {
            slot: self.slot,
            _data: std::marker::PhantomData,
        }
    }
}

/// A slab consists of a data vector whose first slot is never used, and a stack
/// containing free entries.
///
// Other fields are included for debugging/tracking purposes; in debug and test
// builds, the slab will do sanity checks at a large performance and size cost.
pub struct Slab<T> {
    data: Vec<MaybeUninit<T>>,
    free_slots: Vec<u32>,
    // Initialized to 0, incremented on every free of the slot.
    #[cfg(any(debug_assertions, test))]
    versions: Vec<usize>,
    // Initialized as a counter, used to detect slab mismatches.
    #[cfg(any(debug_assertions, test))]
    slab_tag: usize,
}

#[cfg(any(debug_assertions, test))]
fn get_slab_tag() -> usize {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    COUNTER.fetch_add(1, Ordering::Relaxed)
}

impl<T> Slab<T> {
    #[cfg(any(debug_assertions, test))]
    pub fn new() -> Slab<T> {
        Slab {
            data: vec![MaybeUninit::uninit()],
            free_slots: vec![],
            versions: vec![0],
            slab_tag: get_slab_tag(),
        }
    }

    #[cfg(all(not(debug_assertions), not(test)))]
    pub fn new() -> Slab<T> {
        Slab {
            data: vec![MaybeUninit::uninit()],
            free_slots: Vec::new(),
        }
    }

    #[cfg(any(debug_assertions, test))]
    fn check_handle(&self, h: &SlabHandle<T>) {
        assert_eq!(
            h.slab_tag, self.slab_tag,
            "Attempt to use handle with a different slab"
        );
        assert_eq!(
            h.slab_version,
            self.versions[h.slot.get() as usize],
            "Use after free"
        );
    }

    #[cfg(all(not(debug_assertions), not(test)))]
    fn check_handle(&self, _handle: &SlabHandle<T>) {}

    /// Read a value from the slab, returning an immutable reference.
    pub fn get(&self, handle: &SlabHandle<T>) -> &T {
        self.check_handle(handle);
        unsafe { &*self.data[handle.slot.get() as usize].as_ptr() }
    }

    /// Read a handle, getting a mutable reference to the element.
    pub fn get_mut(&mut self, handle: &SlabHandle<T>) -> &mut T {
        self.check_handle(handle);
        unsafe { &mut *self.data[handle.slot.get() as usize].as_mut_ptr() }
    }

    /// Find an empty slot, returning the index, or grow the slab as needed.  In debug builds, also bump the version of the newly returned slot.
    fn allocate_empty_slot(&mut self) -> NonZeroU32 {
        let ret = match self.free_slots.pop() {
            Some(x) => x,
            None => {
                self.data.push(MaybeUninit::uninit());
                #[cfg(any(debug_assertions, test))]
                {
                    self.versions.push(0);
                }
                (self.data.len() - 1) as u32
            }
        };
        debug_assert_ne!(ret, 0);
        unsafe { NonZeroU32::new_unchecked(ret) }
    }

    #[cfg(any(debug_assertions, test))]
    fn allocate_handle(&mut self) -> SlabHandle<T> {
        let slot = self.allocate_empty_slot();
        let slot_u = slot.get() as usize;
        SlabHandle {
            slot,
            slab_version: self.versions[slot_u],
            slab_tag: self.slab_tag,
            _data: std::marker::PhantomData,
        }
    }

    #[cfg(not(any(debug_assertions, test)))]
    fn allocate_handle(&mut self) -> SlabHandle<T> {
        let slot = self.allocate_empty_slot();
        SlabHandle {
            slot,
            _data: std::marker::PhantomData,
        }
    }

    /// Insert an item into the slab, growing it as needed.
    #[must_use = "Failure to use returned handles permanently leaks data"]
    pub fn insert(&mut self, value: T) -> SlabHandle<T> {
        let new_handle = self.allocate_handle();
        self.data[new_handle.slot.get() as usize] = MaybeUninit::new(value);
        new_handle
    }

    /// Remove an item from the slab.  Doesn't shrink the slab.
    pub fn remove(&mut self, handle: SlabHandle<T>) {
        self.check_handle(&handle);
        let ptr = self.data[handle.slot.get() as usize].as_mut_ptr();
        unsafe { std::ptr::drop_in_place(ptr) };
        self.free_slots.push(handle.slot.get());
        #[cfg(any(debug_assertions, test))]
        {
            self.versions[handle.slot.get() as usize] += 1;
            self.data[handle.slot.get() as usize] = MaybeUninit::zeroed();
        }
    }

    /// Get the capacity of the slab. Like `Vec`.
    pub fn capacity(&self) -> usize {
        self.data.capacity()
    }

    /// How many items of the slab are used?
    pub fn used_count(&self) -> usize {
        // Don't forget that the first element of data is unused.
        self.data.len() - self.free_slots.len() - 1
    }

    /// How many objects can we reallocate before the slab would again expand?
    pub fn available_slots(&self) -> usize {
        // Don't forget that the first element of data is unused.
        let from_cap = self.data.capacity() - self.data.len() - 1;
        let from_free = self.free_slots.len();
        from_cap + from_free
    }
}

impl<T> Drop for Slab<T> {
    fn drop(&mut self) {
        // Slabs can be large, so we don't want to actually end up duplicating them as hashsets. Instead, sort the fre slots and infer the ranges.
        self.free_slots.sort_unstable();
        // Don't start at 0, the first index is always uninitialized.
        let mut last = 1;
        for next_free in self.free_slots.iter().copied() {
            for ind in last..next_free {
                unsafe { std::ptr::drop_in_place(self.data[ind as usize].as_mut_ptr()) };
            }
            last = next_free + 1;
        }
        for ind in last as usize..self.data.len() {
            unsafe { std::ptr::drop_in_place(self.data[ind].as_mut_ptr()) };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::Mutex;

    #[test]
    fn basic() {
        let mut slab = Slab::<u32>::new();
        let h1 = slab.insert(1);
        let h2 = slab.insert(2);
        let h3 = slab.insert(3);
        assert_eq!([*slab.get(&h1), *slab.get(&h2), *slab.get(&h3)], [1, 2, 3]);
        assert_eq!(slab.used_count(), 3);
        assert_eq!(slab.free_slots, vec![]);
    }

    #[test]
    fn basic_free() {
        let mut slab = Slab::<u32>::new();
        let h1 = slab.insert(1);
        let h2 = slab.insert(2);
        let h3 = slab.insert(3);
        slab.remove(h2);
        assert_eq!(slab.free_slots, vec![2]);
        let h4 = slab.insert(4);
        assert_eq!(*slab.get(&h1), 1);
        assert_eq!(*slab.get(&h3), 3);
        assert_eq!(*slab.get(&h4), 4);
        assert_eq!(slab.used_count(), 3);
        assert_eq!(slab.free_slots, vec![]);
        assert_eq!(slab.versions, vec![0, 0, 1, 0]);
    }

    #[test]
    fn no_explosive_growth() {
        let mut slab = Slab::<u32>::new();

        // Add a bunch of handles and free several times, then check that things iddn't go nuts.
        for outer in 0..10u32 {
            let mut handles = vec![];
            for i in 0..10000u32 {
                handles.push((i * outer, slab.insert(i * outer)));
            }
            for (expected, handle) in handles.into_iter() {
                assert_eq!(*slab.get(&handle), expected);
                slab.remove(handle);
            }
            assert_eq!(slab.data.len(), 10001);
            assert_eq!(slab.free_slots.len(), 10000);
        }
    }

    struct Dropper<'a> {
        dest: &'a Mutex<bool>,
    }

    impl<'a> Drop for Dropper<'a> {
        fn drop(&mut self) {
            *self.dest.lock().unwrap() = true;
        }
    }

    #[test]
    fn drop_is_called() {
        let dropped = Mutex::new(false);
        let mut slab = Slab::new();
        let handle = slab.insert(Dropper { dest: &dropped });
        slab.remove(handle);
        std::mem::forget(slab);
        assert!(*dropped.lock().unwrap());
    }

    fn mutex_vec_to_bool(mvec: &Vec<Mutex<bool>>) -> Vec<bool> {
        mvec.iter().map(|i| *i.lock().unwrap()).collect()
    }

    #[test]
    fn dropping_slab_drops_items() {
        let mut drop_arr = vec![];
        for _ in 0..10 {
            drop_arr.push(Mutex::new(false));
        }

        let mut slab = Slab::new();
        let mut handles = vec![];
        for i in 0..10 {
            handles.push(slab.insert(Dropper { dest: &drop_arr[i] }));
        }

        // We want to put some holes in it to see if we can crash the drop impl.
        slab.remove(handles[2].clone());
        slab.remove(handles[3].clone());
        slab.remove(handles[5].clone());
        slab.remove(handles[9].clone());
        assert_eq!(
            mutex_vec_to_bool(&drop_arr),
            vec![false, false, true, true, false, true, false, false, false, true]
        );

        std::mem::drop(slab);
        assert_eq!(mutex_vec_to_bool(&drop_arr), vec![true; 10]);
    }

    #[test]
    #[should_panic]
    fn test_builds_catch_double_free() {
        let mut slab = Slab::<u32>::new();
        let h1 = slab.insert(1);
        slab.remove(h1.clone());
        slab.remove(h1);
    }

    #[test]
    fn slab_tags_are_different() {
        let slab1 = Slab::<u32>::new();
        let slab2 = Slab::<u32>::new();
        assert_ne!(slab1.slab_tag, slab2.slab_tag);
    }

    #[test]
    #[should_panic]
    fn test_builds_catch_slab_handles_used_with_wrong_slab() {
        let mut slab1 = Slab::<u32>::new();
        let slab2 = Slab::<u32>::new();
        let h = slab1.insert(1);
        slab2.get(&h);
    }
}
