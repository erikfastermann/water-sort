fn main() {
    println!("cargo:rerun-if-env-changed=WATER_SORT_ITEM_BITS");
    println!("cargo:rerun-if-env-changed=WATER_SORT_BOTTLE_BITS");
    println!("cargo:rerun-if-env-changed=WATER_SORT_COLOR_BITS");
    println!("cargo:rerun-if-env-changed=WATER_SORT_SAFE_COUNTER_BITS");
}
