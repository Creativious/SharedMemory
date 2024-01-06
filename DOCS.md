```rust
use shared_memory::shared_memory::SharedMemory;

fn main() {
    let mut shm = SharedMemory::create("test", 1024).unwrap();
    shm.write_string("Hello World!");

    let mut shm_other = SharedMemory::create("test", 1024).unwrap();
    // convert the data to a string and then print it
    println!("{}", shm_other.read_string());

}
```