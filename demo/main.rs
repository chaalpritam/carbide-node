//! Carbide Network Interactive Demo
//!
//! This is the main entry point for the comprehensive Carbide Network demo.
//! It showcases all components of the decentralized storage marketplace.

use clap::{App, Arg};
use std::time::Duration;
use tokio;

mod r#mod;
use r#mod::*;

mod interactive_demo;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    // Parse command line arguments
    let matches = App::new("Carbide Network Demo")
        .version("1.0")
        .about("Interactive demo of the Carbide decentralized storage marketplace")
        .arg(
            Arg::with_name("providers")
                .short("p")
                .long("providers")
                .value_name("COUNT")
                .help("Number of providers to simulate")
                .takes_value(true)
                .default_value("5"),
        )
        .arg(
            Arg::with_name("clients")
                .short("c")
                .long("clients")
                .value_name("COUNT")
                .help("Number of clients to simulate")
                .takes_value(true)
                .default_value("3"),
        )
        .arg(
            Arg::with_name("duration")
                .short("d")
                .long("duration")
                .value_name("SECONDS")
                .help("Demo duration in seconds")
                .takes_value(true)
                .default_value("60"),
        )
        .arg(
            Arg::with_name("verbose")
                .short("v")
                .long("verbose")
                .help("Enable verbose output")
                .takes_value(false),
        )
        .arg(
            Arg::with_name("quick")
                .short("q")
                .long("quick")
                .help("Run a quick demo (30 seconds)")
                .takes_value(false),
        )
        .get_matches();

    // Parse configuration from arguments
    let provider_count = matches
        .value_of("providers")
        .unwrap()
        .parse::<usize>()
        .unwrap_or(5);
    
    let client_count = matches
        .value_of("clients")
        .unwrap()
        .parse::<usize>()
        .unwrap_or(3);
    
    let duration_secs = if matches.is_present("quick") {
        30
    } else {
        matches
            .value_of("duration")
            .unwrap()
            .parse::<u64>()
            .unwrap_or(60)
    };
    
    let verbose = matches.is_present("verbose");

    // Create demo configuration
    let config = DemoConfig {
        provider_count,
        client_count,
        demo_duration: Duration::from_secs(duration_secs),
        data_dir: std::env::temp_dir().join("carbide_demo"),
        verbose,
        network_config: if matches.is_present("quick") {
            NetworkConfig {
                latency_range: (10, 50),
                packet_loss: 0.0,
                failure_rate: 0.02,
                bandwidth_limit: None,
            }
        } else {
            NetworkConfig::default()
        },
    };

    // Welcome message
    println!("🌟 Welcome to the Carbide Network Demo! 🌟");
    println!("=========================================");
    println!();
    println!("This demo showcases a decentralized storage marketplace where:");
    println!("• Multiple providers offer storage with different tiers and pricing");
    println!("• Clients can discover and select providers based on their needs");
    println!("• A reputation system tracks provider performance and reliability");
    println!("• Market dynamics drive competition and quality improvement");
    println!();
    println!("Demo Configuration:");
    println!("  Providers: {}", config.provider_count);
    println!("  Clients: {}", config.client_count);
    println!("  Duration: {} seconds", duration_secs);
    println!("  Verbose: {}", config.verbose);
    println!();
    
    // Ask for user confirmation
    print!("Press Enter to start the demo, or Ctrl+C to exit...");
    std::io::stdin().read_line(&mut String::new()).unwrap();
    
    // Clear screen and start demo
    if !config.verbose {
        print!("\x1B[2J\x1B[1;1H"); // Clear screen
    }

    // Create and run the demo
    let mut demo_runner = DemoRunner::new(config).await?;
    
    match demo_runner.run_demo().await {
        Ok(results) => {
            println!("\n🎉 Demo completed successfully!");
            
            // Save results to file
            let results_file = std::env::temp_dir().join("carbide_demo_results.txt");
            std::fs::write(&results_file, results.generate_report())?;
            println!("📄 Detailed results saved to: {}", results_file.display());
            
            // Offer to run again
            println!("\nWould you like to:");
            println!("1. Run the demo again");
            println!("2. Exit");
            print!("Enter your choice (1 or 2): ");
            
            let mut input = String::new();
            std::io::stdin().read_line(&mut input).unwrap();
            
            if input.trim() == "1" {
                println!("\nRestarting demo...\n");
                // Recursively restart
                let args: Vec<String> = std::env::args().collect();
                let mut cmd = std::process::Command::new(&args[0]);
                for arg in &args[1..] {
                    cmd.arg(arg);
                }
                cmd.status().expect("Failed to restart demo");
            } else {
                println!("Thank you for trying the Carbide Network demo! 👋");
            }
        }
        Err(e) => {
            eprintln!("❌ Demo failed: {}", e);
            std::process::exit(1);
        }
    }

    Ok(())
}

/// Display ASCII art banner
fn display_banner() {
    println!(r#"
    ╔═══════════════════════════════════════════════════════════════╗
    ║                                                               ║
    ║      ██████╗ ██████╗ ██████╗ ██████╗ ██╗██████╗ ███████╗     ║
    ║     ██╔════╝██╔══██╗██╔══██╗██╔══██╗██║██╔══██╗██╔════╝     ║
    ║     ██║     ██████╔╝██████╔╝██████╔╝██║██║  ██║█████╗       ║
    ║     ██║     ██╔══██╗██╔══██╗██╔══██╗██║██║  ██║██╔══╝       ║
    ║     ╚██████╗██║  ██║██║  ██║██████╔╝██║██████╔╝███████╗     ║
    ║      ╚═════╝╚═╝  ╚═╝╚═╝  ╚═╝╚═════╝ ╚═╝╚═════╝ ╚══════╝     ║
    ║                                                               ║
    ║              🌐 Decentralized Storage Marketplace 🌐          ║
    ║                                                               ║
    ╚═══════════════════════════════════════════════════════════════╝
    "#);
}

/// Display demo scenarios menu
fn display_scenarios() -> Result<usize, Box<dyn std::error::Error>> {
    println!("\n📋 Available Demo Scenarios:");
    println!("════════════════════════════");
    println!("1. 🚀 Quick Demo (30 seconds) - Fast overview of all features");
    println!("2. 📊 Standard Demo (2 minutes) - Comprehensive demonstration");
    println!("3. 🔬 Extended Demo (5 minutes) - Deep dive with detailed analysis");
    println!("4. 🧪 Custom Demo - Configure your own parameters");
    println!("5. ❌ Exit");
    
    print!("\nEnter your choice (1-5): ");
    
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    
    let choice = input.trim().parse::<usize>().unwrap_or(5);
    
    if choice > 5 {
        println!("Invalid choice. Exiting...");
        std::process::exit(0);
    }
    
    Ok(choice)
}

/// Create demo configuration based on user choice
fn create_demo_config(scenario: usize) -> Result<DemoConfig, Box<dyn std::error::Error>> {
    match scenario {
        1 => Ok(DemoConfig {
            provider_count: 3,
            client_count: 2,
            demo_duration: Duration::from_secs(30),
            verbose: false,
            network_config: NetworkConfig {
                latency_range: (10, 50),
                packet_loss: 0.0,
                failure_rate: 0.01,
                bandwidth_limit: None,
            },
            ..Default::default()
        }),
        2 => Ok(DemoConfig::default()),
        3 => Ok(DemoConfig {
            provider_count: 8,
            client_count: 5,
            demo_duration: Duration::from_secs(300),
            verbose: true,
            ..Default::default()
        }),
        4 => {
            // Custom configuration
            println!("\n⚙️  Custom Demo Configuration");
            println!("─────────────────────────────");
            
            print!("Number of providers (1-10): ");
            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;
            let providers = input.trim().parse::<usize>().unwrap_or(5).min(10).max(1);
            
            print!("Number of clients (1-10): ");
            input.clear();
            std::io::stdin().read_line(&mut input)?;
            let clients = input.trim().parse::<usize>().unwrap_or(3).min(10).max(1);
            
            print!("Demo duration in seconds (30-600): ");
            input.clear();
            std::io::stdin().read_line(&mut input)?;
            let duration = input.trim().parse::<u64>().unwrap_or(120).min(600).max(30);
            
            print!("Enable verbose output? (y/n): ");
            input.clear();
            std::io::stdin().read_line(&mut input)?;
            let verbose = input.trim().to_lowercase().starts_with('y');
            
            Ok(DemoConfig {
                provider_count: providers,
                client_count: clients,
                demo_duration: Duration::from_secs(duration),
                verbose,
                ..Default::default()
            })
        }
        _ => std::process::exit(0),
    }
}