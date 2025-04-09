// Clap is the command line parser
use clap::Parser;
use landlord::{HitPolicy, Item, Landlord, TiebreakingPolicy};
use serde::Deserialize;
use std::collections::VecDeque;
// We need to include the logger to do cost and pressure logging
use logger::{PrettyTracker, Tracker};
// We need ordered floats to keep them properly in our cache map
// Io and path are required for writing to our output file and getting our path buffer input.
use std::io::Write;
use std::path::PathBuf;
// File system is required to actually read and write toml files. Env is required to read command
// line arguments.
use std::fs::{self, File};

pub mod landlord;
pub mod logger;

// STRUCTS
// ----------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct TraceInfo {
    items: Vec<Item>,
    trace: Vec<String>,
}

#[derive(Parser, Debug)]
#[command(name = "csim")]
#[command(version = "1.0")]
#[command(about = "A simple cache simulator for the Landlord cache replacement policy")]
pub struct Args {
    /// The path to the input TOML file
    #[arg(short, long, value_name = "INPUT FILE")]
    in_path: PathBuf,

    /// The path to the TOML file we are saving to
    #[arg(short, long, value_name = "OUTPUT FILE")]
    out_path: String,

    /// The size of the caches we are running
    #[arg(short, long, value_name = "CACHE SIZE")]
    size: u32,

    /// The location in our trace where we should split prefix from suffix
    #[arg(short, long, value_name = "PREFIX/SUFFIX DIVISION")]
    div: u32,

    /// The hit and tiebreaking policies for our caches
    #[arg(short, long, num_args = 2, value_name = "HIT/TIEBREAKING POLICY")]
    policies: Vec<String>,
}

// This is the data structure that serde will deserialize the items.toml file into. The items must
// be an exhaustive list of the costs and sizes of the items requested in our trace. Meanwhile, the
// trace is just a vector of strings where each string is an item's label.

// Converts our deserialized trace of strings into a trace of items
fn strings_to_items(trace: &TraceInfo) -> VecDeque<&Item> {
    let mut requests = VecDeque::new();
    let mut counter = 0;
    for request in trace.trace.iter() {
        for item in trace.items.iter() {
            if *item.get_label() == *request {
                requests.push_back(item);
                counter += 1;
            }
        }
    }
    if counter as usize != trace.trace.len() {
        panic!("Invalid trace generation");
    }
    requests
}

fn main() {
    let args = Args::parse();
    // Parsing our data into a string
    let data: &str = &fs::read_to_string(args.in_path).expect("Could not read file");
    // Converting our string into a trace struct with the TOML crate
    let raw_trace: TraceInfo = toml::from_str(data).expect("Could not convert TOML file");
    // Performing some input sanitzation to ensure we don't have any items too large to accomodate
    for item in raw_trace.items.iter() {
        if item.get_size() > args.size {
            println!(
                "Item {} has size {} exceeding cache size of {}",
                item.get_label(),
                item.get_size(),
                args.size
            );
            return;
        }
    }
    // Converting strings into items with our utility function
    let item_trace = strings_to_items(&raw_trace);
    // Creating our two caches
    if args.policies.len() > 2 {
        println!("Could not parse policy input");
        return;
    }
    // Generating our hit policy from the input
    let hit_policy = match args.policies[0].to_ascii_uppercase().as_str() {
        "LRU" => HitPolicy::Lru,
        "FIFO" => HitPolicy::Fifo,
        "RAND" => HitPolicy::Rand,
        "HALF" => HitPolicy::Half,
        _ => {
            println!("Invalid hit policy; select one of: {{LRU, FIFO, RAND, HALF}}");
            return;
        }
    };
    // Generating our tiebreaking policy from the input
    let tiebreaking_policy = match args.policies[1].to_ascii_uppercase().as_str() {
        "LRU" => TiebreakingPolicy::Lru,
        "FIFO" => TiebreakingPolicy::Fifo,
        "RAND" => TiebreakingPolicy::Rand,
        _ => {
            println!("Invalid tiebreaking policy; select one of: {{LRU, FIFO, RAND}}");
            return;
        }
    };
    // Creating our Landlord instances
    let s = Landlord::new(args.size, tiebreaking_policy, hit_policy);
    let f = Landlord::new(args.size, tiebreaking_policy, hit_policy);
    // Creating our tracker
    let mut tracker = Tracker::new(&item_trace);
    // Running the caches on our trace with the tracker
    Landlord::run(item_trace, args.div, s, f, &mut tracker);
    // Creating a pretty tracker instance for serialization
    let display = PrettyTracker::new(tracker);
    // Serializing our pretty tracker into a string
    let output = display.ser_logger();
    // Creating the output file
    let out_file = File::create(args.out_path);
    // If we get an error, the output path was already taken or we do not have permission.
    if out_file.is_err() {
        println!("Output file path already taken.");
        return;
    }
    // Unwrapping the file if we passed the error testing.
    let mut out_file = out_file.unwrap();
    // Writing our serialized data structure into the file.
    let _ = out_file.write_all(output.as_bytes());
}
