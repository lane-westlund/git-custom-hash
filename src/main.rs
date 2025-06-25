use std::env;
use std::sync::{Arc};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::Duration;
use git2::{Repository, Commit, Signature};
use sha1::{Sha1, Digest};
use num_cpus;
use regex::Regex;

fn check_commit_with_nonce(
    raw_header: &str,
    raw_message: &str,
    committer_name: &str,
    nonce: u64,
    desired_hash_start: &str,
    hidden_message: Option<&str>, // New parameter for hidden message
) -> bool {
    let mut hasher = Sha1::new();

    // Find the position of the committer name in the raw header
    if let Some(pos) = raw_header.rfind(committer_name) {
        // Preallocate a buffer for the modified committer
        let mut modified_committer = String::with_capacity(16 + committer_name.len());
        modified_committer.push_str(&format!("{:x}_", nonce)); // Append the nonce in hex format
        modified_committer.push_str(committer_name);          // Append the committer name

        // Build the new commit header by reusing slices and avoiding unnecessary allocations
        let mut new_commit_header = String::with_capacity(raw_header.len() + modified_committer.len());
        new_commit_header.push_str(&raw_header[..pos]);       // Part before the committer name
        new_commit_header.push_str(&modified_committer);      // Modified committer
        new_commit_header.push_str(&raw_header[pos + committer_name.len()..]); // Part after the committer name
        new_commit_header.push('\n');                         // Append newline

        // Build the full commit string
        let raw_commit = format!("{}{}", new_commit_header, raw_message);
        let commit_preface = format!("commit {}{}", raw_commit.len(), '\0');
        let full_commit = commit_preface + &raw_commit;

        // Hash the full commit string
        hasher.update(full_commit);
        let result = hasher.finalize();

        // Convert the hash to a hexadecimal string
        let hash_hex = format!("{:x}", result);

        // Check for both conditions if both are provided
        if let Some(hidden) = hidden_message {
            hash_hex.starts_with(desired_hash_start) && hash_hex.contains(hidden)
        } else {
            hash_hex.starts_with(desired_hash_start) // Default: check for prefix only
        }
    } else {
        false
    }
}

fn build_commit_with_nonce(commit: &Commit, nonce: u64) -> Result<(), git2::Error> {
    let committer = commit.committer();
    let committer_name_raw = committer.name().unwrap_or("Unknown");
    let committer_name = sanitize_committer_name(committer_name_raw); // Sanitize the committer name

    let modified_committer = format!("{:x}_{}", nonce, committer_name);

    let new_committer = Signature::new(
        &modified_committer,
        committer.email().unwrap_or(""),
        &committer.when(),
    )?;

    commit.amend(
        Some("HEAD"),          // Reference to update (HEAD in this case)
        None,                  // Keep the original author
        Some(&new_committer),  // Update the committer
        None,                  // Keep the original commit message encoding
        None,                  // Keep the original commit message
        None,                  // Keep the original tree
    )?;

    Ok(())
}

fn thread_logic(
    raw_header: &str,
    raw_message: &str,
    committer_name: &str,
    number: Arc<AtomicU64>,
    desired_hex_value: &str,
    shared_result: Arc<AtomicU64>,
    hidden_message: Option<&str>, // Pass hidden message to thread logic
) {
    loop {
        // Check if another thread has already populated the result
        if shared_result.load(Ordering::SeqCst) != 0 {
            return; // Exit early if the result is already set
        }

        // Call the function
        let number_under_test = number.fetch_add(100, Ordering::SeqCst);
        for i in 0..99 {
            if check_commit_with_nonce(
                raw_header,
                raw_message,
                committer_name,
                number_under_test + i,
                desired_hex_value,
                hidden_message,
            ) {
                let _ = shared_result.compare_exchange(
                    0,
                    number_under_test + i,
                    Ordering::SeqCst,
                    Ordering::SeqCst,
                ).is_ok(); // If successful, exit the thread
                return;
            }
        }
    }
}

fn is_valid_hex(value: &str) -> bool {
    !value.is_empty() && value.chars().all(|c| c.is_digit(16))
}

fn get_argument_value(args: &[String], flag: &str) -> Option<String> {
    if let Some(index) = args.iter().position(|arg| arg == flag) {
        args.get(index + 1).cloned()
    } else {
        None
    }
}

fn sanitize_committer_name(committer_name: &str) -> String {
    // Use a regex to remove a leading hexadecimal number followed by an underscore
    let re = Regex::new(r"^[0-9a-fA-F]+_").unwrap();
    re.replace(committer_name, "").to_string()
}

fn sanitize_raw_header(raw_header: &str, committer_name_raw: &str, committer_name: &str) -> String {
    // Replace the unsanitized committer name with the sanitized one in the raw header
    raw_header.replace(committer_name_raw, committer_name)
}

fn main() {
    let args: Vec<String> = env::args().collect();

    // Parse the -h parameter for the desired hash prefix
    let hex_parameter = get_argument_value(&args, "-h").filter(|valid_hex| is_valid_hex(valid_hex));

    // Parse the -m parameter for the hidden message
    let hidden_message = get_argument_value(&args, "-m").filter(|hidden| is_valid_hex(hidden));

    // Parse the -n parameter for the starting nonce (hexadecimal only, default to 1 if not provided)
    let starting_nonce = get_argument_value(&args, "-n")
        .map(|n| u64::from_str_radix(&n, 16).unwrap_or_else(|_| {
            println!("Error: Invalid -n parameter. Please provide a valid hexadecimal value.");
            println!("Usage: cargo run --release -- [-h <hash_prefix>] [-m <hidden_message>] [-n <starting_nonce>] [-j <num_threads>]");
            std::process::exit(1);
        }))
        .unwrap_or(1); // Default to 1 if -n is not provided

    // Ensure at least one of -h or -m is provided
    if hex_parameter.is_none() && hidden_message.is_none() {
        println!("Error: You must provide at least one of -h (hash prefix) or -m (hidden message).");
        println!("Usage: cargo run --release -- [-h <hash_prefix>] [-m <hidden_message>] [-n <starting_nonce>] [-j <num_threads>]");
        return;
    }

    // Parse the -j parameter for the number of threads
    let num_threads = if let Some(thread_count) = get_argument_value(&args, "-j") {
        thread_count.parse::<usize>().unwrap_or_else(|_| {
            println!("Invalid -j parameter. Using default number of threads.");
            num_cpus::get()
        })
    } else {
        num_cpus::get()
    };

    if let Some(ref hex) = hex_parameter {
        println!("Searching for hash starting with: {}", hex);
    }
    if let Some(ref hidden) = hidden_message {
        println!("Searching for hidden message: {}", hidden);
    }
    println!("Starting nonce: {:X}", starting_nonce); // Display the starting nonce in hex
    println!("Using {} threads.", num_threads);

    let hex_value = Arc::new(hex_parameter.unwrap_or_default());
    let hidden_message = Arc::new(hidden_message); // Share hidden message across threads
    let nonce = Arc::new(AtomicU64::new(starting_nonce)); // Use the starting nonce
    let repo = Repository::open(".").expect("Failed to open Git repository");
    let head = repo.head().expect("Failed to get HEAD reference");
    let commit = head.peel_to_commit().expect("Failed to resolve HEAD to commit");
    let raw_header_raw = commit.raw_header().unwrap_or("No RAW bytes").to_string();
    let raw_message = Arc::new(commit.message_raw().unwrap_or("No RAW bytes").to_string());
    let committer = commit.committer();
    let committer_name_raw = committer.name().unwrap_or("Unknown").to_string();
    let committer_name = Arc::new(sanitize_committer_name(&committer_name_raw));
    let raw_header = Arc::new(sanitize_raw_header(&raw_header_raw, &committer_name_raw, &committer_name));
    let shared_result = Arc::new(AtomicU64::new(0)); // Now an AtomicU64

    // Start a monitoring thread to display nonce increase per 5 seconds
    let nonce_clone = Arc::clone(&nonce);
    thread::spawn(move || {
        let mut previous_nonce = starting_nonce;
        loop {
            thread::sleep(Duration::from_secs(5));
            let current_nonce = nonce_clone.load(Ordering::SeqCst);
            let hashes_per_second = (current_nonce - previous_nonce) / 5; // Average over 5 seconds
            previous_nonce = current_nonce;

            // Round to the nearest thousand and format with "K"
            let hashes_per_thousand = (hashes_per_second + 500) / 1000; // Round to nearest thousand
            println!("Hashes per second: {}K\tMost recent nonce: {:X}", hashes_per_thousand, current_nonce);
        }
    });

    let mut handles = vec![];
    for _ in 0..num_threads {
        let hex_value_clone = Arc::clone(&hex_value);
        let hidden_message_clone = Arc::clone(&hidden_message);
        let shared_result_clone = Arc::clone(&shared_result);
        let raw_header_clone = Arc::clone(&raw_header);
        let raw_message_clone = Arc::clone(&raw_message);
        let committer_name_clone = Arc::clone(&committer_name);
        let nonce_clone = Arc::clone(&nonce);

        let handle = thread::spawn(move || {
            thread_logic(
                &raw_header_clone,
                &raw_message_clone,
                &committer_name_clone,
                nonce_clone,
                &hex_value_clone,
                shared_result_clone,
                hidden_message_clone.as_deref(), // Pass hidden message
            );
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    let final_result = shared_result.load(Ordering::SeqCst);
    if final_result != 0 {
        println!("A thread found: {:X}", final_result);
    } else {
        println!("No thread returned a result.");
    }

    let _ = build_commit_with_nonce(&commit, final_result);
}
