use crate::input::{self, Key};
use crate::memory::{Address, MemoryReader};
use parking_lot::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

const BASE_ADDRESS: usize = 0x05A5D218;
const OFFSETS: &[usize] = &[0, 0x230, 0x18, 0x798, 0x48, 0x10, 0x78, 0xC];
const CAST_DELAY: Duration = Duration::from_secs(1);
const INITIAL_DELAY: Duration = Duration::from_secs(5);
const POLL_INTERVAL: Duration = Duration::from_millis(50);

pub struct Fisher {
    memory: Arc<MemoryReader>,
    rod_address: Mutex<Option<Address>>,
    should_stop: AtomicBool,
}

#[derive(PartialEq)]
enum FishingState {
    Casting,
    WaitingForBite,
    Reeling,
}

impl Fisher {
    pub fn new(memory: Arc<MemoryReader>) -> Self {
        Fisher {
            memory,
            rod_address: Mutex::new(None),
            should_stop: AtomicBool::new(false),
        }
    }

    pub fn run(&self, max_casts: Option<i32>, threshold: u32, stop_sender: Sender<()>) {
        self.find_rod_address();
        if self.rod_address.lock().is_none() {
            println!("failed to find fishing rod address");
            return;
        }

        println!("starting fishing loop in 5 seconds...");
        thread::sleep(INITIAL_DELAY);

        println!("fishing started!");
        let mut cast_count = 0;
        while !self.should_stop.load(Ordering::Relaxed) {
            self.fish_cycle(threshold);
            cast_count += 1;

            if let Some(max) = max_casts {
                if cast_count >= max {
                    println!("reached maximum number of casts");
                    break;
                }
            }
        }
        println!("fishing stopped");
        let _ = stop_sender.send(());
    }

    pub fn stop(&self) {
        self.should_stop.store(true, Ordering::Relaxed);
    }

    fn find_rod_address(&self) {
        if let Ok(base) = self.memory.get_module_base("Minecraft.Windows.exe") {
            let absolute_base = base + BASE_ADDRESS;
            if let Ok(address) = self.memory.follow_pointers(absolute_base, OFFSETS) {
                let mut rod_address = self.rod_address.lock();
                *rod_address = Some(address);
                println!("fishing rod address found: 0x{:X}", address);
            } else {
                println!("failed to follow pointers for fishing rod address");
            }
        } else {
            println!("failed to get module base for Minecraft.Windows.exe");
        }
    }

    fn fish_cycle(&self, threshold: u32) {
        let mut state = FishingState::Casting;
        let mut peak_value = 0;
        let mut last_value = 0;
        let mut falling_count = 0;
        let mut post_peak_count = 0;
        let start_time = Instant::now();

        self.cast();
        thread::sleep(CAST_DELAY);

        while state != FishingState::Reeling && !self.should_stop.load(Ordering::Relaxed) {
            if let Some(current_value) = self.get_rod_state() {
                println!("current rod state: {}", current_value);

                state = self.update_fishing_state(
                    state,
                    current_value,
                    &mut peak_value,
                    &mut last_value,
                    &mut falling_count,
                    &mut post_peak_count,
                    threshold,
                );
            }

            if self.is_timeout(start_time) {
                println!("timeout reached, recasting...");
                break;
            }

            thread::sleep(POLL_INTERVAL);
        }

        if state == FishingState::Reeling {
            self.reel();
        }

        thread::sleep(Duration::from_secs(1));
    }

    fn update_fishing_state(
        &self,
        state: FishingState,
        current_value: u32,
        peak_value: &mut u32,
        last_value: &mut u32,
        falling_count: &mut u32,
        post_peak_count: &mut u32,
        threshold: u32,
    ) -> FishingState {
        match state {
            FishingState::Casting => {
                if current_value > 0 {
                    println!("waiting for bite...");
                    FishingState::WaitingForBite
                } else {
                    state
                }
            }
            FishingState::WaitingForBite => self.update_bite_detection(
                current_value,
                peak_value,
                last_value,
                falling_count,
                post_peak_count,
                threshold,
            ),
            FishingState::Reeling => state,
        }
    }

    fn update_bite_detection(
        &self,
        current_value: u32,
        peak_value: &mut u32,
        last_value: &mut u32,
        falling_count: &mut u32,
        post_peak_count: &mut u32,
        threshold: u32,
    ) -> FishingState {
        if current_value > *peak_value {
            *peak_value = current_value;
            *falling_count = 0;
            *post_peak_count = 0;
        } else if current_value < *last_value {
            *falling_count += 1;
            if *falling_count > 2 && *peak_value > 2 {
                *post_peak_count += 1;
                if *post_peak_count > threshold {
                    return FishingState::Reeling;
                }
            }
        } else {
            *falling_count = 0;
        }
        *last_value = current_value;
        FishingState::WaitingForBite
    }

    fn is_timeout(&self, start_time: Instant) -> bool {
        start_time.elapsed() > Duration::from_secs(60)
    }

    fn cast(&self) {
        println!("casting rod...");
        input::send_key(Key::MouseRight);
    }

    fn reel(&self) {
        println!("fish on, reeling in...");
        input::send_key(Key::MouseRight);
    }

    fn get_rod_state(&self) -> Option<u32> {
        self.rod_address
            .lock()
            .and_then(|addr| self.memory.read::<u32>(addr).ok())
    }
}
