// Read-only Windows host for the exact ASR reader used by the WASM build.
// No timer functions and no WriteProcessMemory access are provided.
#[cfg(windows)]
mod host;

#[cfg(windows)]
fn main() {
    use escape_simulator_autosplitter::reader::Reader;
    let pid = std::env::args()
        .nth(1)
        .expect("Usage: probe <game PID> [seconds]")
        .parse()
        .expect("PID");
    let seconds: u64 = std::env::args()
        .nth(2)
        .unwrap_or("5".into())
        .parse()
        .expect("seconds");
    let process =
        asr::Process::attach_by_pid(asr::ProcessId(pid)).expect("Cannot open game for reading");
    for name in ["mono-2.0-bdwgc.dll", "UnityPlayer.dll"] {
        let range = process.get_module_range(name);
        println!("Module {name}: {range:?}");
        if let Ok((address, _)) = range {
            println!(
                "Machine: {:?}; version: {:?}",
                asr::file_format::pe::MachineType::read(&process, address),
                asr::file_format::pe::FileVersion::read(&process, address)
            );
        }
    }
    let mut reader = match Reader::attach(&process) {
        Ok(reader) => reader,
        Err(error) => {
            eprintln!("BIND ERROR: {error}");
            std::process::exit(1);
        }
    };
    println!("Bound Mono and game fields successfully.");
    let mut last = String::new();
    let mut tracker = escape_simulator_autosplitter::logic::Tracker::default();
    let options = escape_simulator_autosplitter::logic::Options {
        il: true,
        ..Default::default()
    };
    let mut simulated_timer = escape_simulator_autosplitter::logic::Timer::Idle;
    let mut last_decision = String::new();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(seconds);
    while std::time::Instant::now() < deadline && process.is_open() {
        let sample = reader.sample(&process);
        let state = format!("{sample:?}");
        let decision = match sample {
            Ok(s) => tracker.step(s, simulated_timer, &options),
            Err(_) => tracker.unavailable(simulated_timer),
        };
        if decision.reset {
            simulated_timer = escape_simulator_autosplitter::logic::Timer::Idle;
        }
        if decision.start {
            simulated_timer = escape_simulator_autosplitter::logic::Timer::Running;
        }
        let description = format!("SIMULATED timer: {simulated_timer:?}, {decision:?}");
        if description != last_decision {
            println!("{description}");
            last_decision = description;
        }
        if state != last {
            println!("{state}");
            last = state;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}

#[cfg(not(windows))]
fn main() {
    eprintln!("The native probe requires Windows.");
}
