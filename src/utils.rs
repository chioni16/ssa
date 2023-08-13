static mut UNIQUE_VALUE: usize = 0;

pub fn get_unique_value() -> usize {
    unsafe {
        let cur_val = UNIQUE_VALUE;
        UNIQUE_VALUE += 1;
        cur_val
    }
}