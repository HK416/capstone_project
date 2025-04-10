use mod_parallelism::collections::{Set, SetAccessor};


const NUM_TEST: i32 = 4_000_000;
const KEY_RANGE: u32 = 1000;
const MAX_THREADS: i32 = 32;
const NUM_THREADS_SET: [i32; 6] = [1, 2, 3, 4, 6, 12];


fn benchmark(set: SetAccessor, num_thread: i32) {
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
    let my_set = Set::new(MAX_THREADS);
    let mut num_thread: i32 = 1;
    while num_thread <= MAX_THREADS {
        my_set.clear();
        my_set.reset_accessor_counter();
        let mut tv = Vec::new();
        let start_t = std::time::Instant::now();
        for _ in 0..num_thread {
            let my_set_accessor = my_set.new_accessor();
            tv.push(std::thread::spawn(move || benchmark(my_set_accessor, num_thread)));
        }
        for th in tv {
            th.join().unwrap();
        }
        let exec_t = start_t.elapsed();
        let ms = exec_t.as_millis();
        println!("{} Threads, {}ms.", num_thread, ms);

        num_thread = num_thread * 2;
    }

    // 현재 컴퓨터가 6코어 12쓰레드
    for num_thread in NUM_THREADS_SET {
        let my_set = Set::new(num_thread);
        let mut tv = Vec::new();
        let start_t = std::time::Instant::now();
        for _ in 0..num_thread {
            let my_set_accessor = my_set.new_accessor();
            tv.push(std::thread::spawn(move || benchmark(my_set_accessor, num_thread)));
        }
        for th in tv {
            th.join().unwrap();
        }
        let exec_t = start_t.elapsed();
        let ms = exec_t.as_millis();
        println!("{} Threads, {}ms.", num_thread, ms);
    }
}
