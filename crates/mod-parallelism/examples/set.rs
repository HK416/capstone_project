use std::sync::Arc;

use mod_parallelism::collections::Set;
use mod_parallelism::collections::set_thread_local_id;


const NUM_TEST: i32 = 4000000;
const KEY_RANGE: u32 = 1000;
const MAX_THREADS: i32 = 16;


fn benchmark(set: Arc<Set>, _th_id: i32, num_thread: i32) {
    unsafe {
        set_thread_local_id(_th_id);
    }

    for _ in 0..(NUM_TEST / num_thread) {
        match rand::random::<u32>() % 3 {
            0 => {
                let key = rand::random::<u32>() % KEY_RANGE;
                set.add(key as i32);
            }
            1 => {
                let key = rand::random::<u32>() % KEY_RANGE;
                set.remove(key as i32);
            }
            2 => {
                let key = rand::random::<u32>() % KEY_RANGE;
                set.contains(key as i32);
            }
            _ => {
                println!("Error");
                std::process::exit(-1);
            }
        }
    }
}


fn main() {
    let my_set = Arc::new(Set::new());
    let mut num_thread: i32 = 1;
    while num_thread <= MAX_THREADS {
        my_set.clear();
        let mut tv = Vec::new();
        let start_t = std::time::Instant::now();
        for i in 0..num_thread {
            let my_set = Arc::clone(&my_set);
            tv.push(std::thread::spawn(move || benchmark(my_set, i, num_thread)));
        }
        for th in tv {
            th.join().unwrap();
        }
        let exec_t = start_t.elapsed();
        let ms = exec_t.as_millis();
        println!("{} Threads, {}ms.", num_thread, ms);

        num_thread = num_thread * 2;
    }
}
