fn main() {
    #[cfg(all(target_arch = "arm", target_pointer_width = "32"))]
    {
        println!(
            "cargo:warning=write_float64 can cause an illegal instruction error on ARM32 due to a \
             known issue in libdlt"
        );
    }
}
