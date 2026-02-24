//! # Carbide Network Marketplace Demo
//!
//! This example demonstrates how the core data structures work together
//! to create a decentralized storage marketplace scenario.

use carbide_core::*;
use rust_decimal::Decimal;

fn main() {
    println!("🚀 Carbide Network Marketplace Demo");
    println!("===================================\n");

    // 1. Create files to store with different characteristics
    let small_file = File::new(
        "document.txt".to_string(),
        b"This is a small text document with important content.".to_vec(),
        "text/plain".to_string(),
    );

    let large_file = File::new(
        "video.mp4".to_string(),
        vec![0u8; 100 * 1024 * 1024], // 100MB video file
        "video/mp4".to_string(),
    );

    println!("📄 Files to Store:");
    println!(
        "  Small file: {} ({} bytes, ID: {})",
        small_file.name, small_file.size, small_file.id
    );
    println!(
        "  Large file: {} ({} bytes, ID: {}, needs chunking: {})",
        large_file.name,
        large_file.size,
        large_file.id,
        large_file.needs_chunking()
    );
    println!();

    // 2. Create diverse storage providers
    let providers = create_sample_providers();

    println!("🏪 Available Storage Providers:");
    for provider in &providers {
        println!(
            "  {} ({:?} tier, {:.3}$/GB/month, {}GB capacity, reputation: {:.1})",
            provider.name,
            provider.tier,
            provider.price_per_gb_month,
            provider.total_capacity / (1024 * 1024 * 1024), // Convert to GB
            provider.reputation.overall
        );
    }
    println!();

    // 3. Demonstrate different storage scenarios
    demonstrate_storage_scenarios(&small_file, &large_file, &providers);

    // 4. Show marketplace economics
    demonstrate_marketplace_economics(&providers);

    println!("\n✅ Demo completed! This shows how Carbide Network enables:");
    println!("   • Content-addressed storage (files identified by hash)");
    println!("   • User-configurable replication (1-10 copies)");
    println!("   • Multi-tier provider ecosystem (Home to Enterprise)");
    println!("   • Economic incentives for providers and users");
    println!("   • Reputation-based trust system");
}

fn create_sample_providers() -> Vec<Provider> {
    let mut providers = Vec::new();

    // Home provider - Alice
    let mut alice = Provider::new(
        "Alice's Home Storage".to_string(),
        ProviderTier::Home,
        Region::NorthAmerica,
        "https://alice.example.com:8080".to_string(),
        2 * 1024 * 1024 * 1024, // 2GB capacity
        Decimal::new(2, 3),     // $0.002/GB/month
    );
    alice.reputation.overall = Decimal::new(75, 2); // 0.75 reputation
    providers.push(alice);

    // Professional provider - Bob
    let mut bob = Provider::new(
        "Bob's Business Storage".to_string(),
        ProviderTier::Professional,
        Region::Europe,
        "https://bob-storage.com".to_string(),
        10 * 1024 * 1024 * 1024, // 10GB capacity
        Decimal::new(4, 3),      // $0.004/GB/month
    );
    bob.reputation.overall = Decimal::new(90, 2); // 0.90 reputation
    providers.push(bob);

    // Enterprise provider - CloudCorp
    let mut cloudcorp = Provider::new(
        "CloudCorp Enterprise".to_string(),
        ProviderTier::Enterprise,
        Region::Asia,
        "https://api.cloudcorp.com".to_string(),
        100 * 1024 * 1024 * 1024, // 100GB capacity
        Decimal::new(8, 3),       // $0.008/GB/month
    );
    cloudcorp.reputation.overall = Decimal::new(95, 2); // 0.95 reputation
    providers.push(cloudcorp);

    // Global CDN provider - GlobalEdge
    let mut globaledge = Provider::new(
        "GlobalEdge CDN".to_string(),
        ProviderTier::GlobalCDN,
        Region::NorthAmerica,
        "https://edge.globalcdn.net".to_string(),
        500 * 1024 * 1024 * 1024, // 500GB capacity
        Decimal::new(12, 3),      // $0.012/GB/month
    );
    globaledge.reputation.overall = Decimal::new(98, 2); // 0.98 reputation
    providers.push(globaledge);

    providers
}

fn demonstrate_storage_scenarios(small_file: &File, large_file: &File, providers: &[Provider]) {
    println!("💾 Storage Scenarios:");

    // Scenario 1: Critical data - need high reliability
    let critical_request = StorageRequest::new(
        small_file.id,
        5,                   // 5 copies for safety
        Decimal::new(10, 3), // Willing to pay $0.010/GB/month
        ProviderRequirements::critical(),
    )
    .expect("Should create critical request");

    println!("\n  📊 Scenario 1: Critical Business Data");
    println!("     File: {} ({} bytes)", small_file.name, small_file.size);
    println!(
        "     Requirements: {} copies, max ${}/GB/month",
        critical_request.replication_factor, critical_request.max_price_per_gb_month
    );
    println!("     Requirements: Exclude home providers, require backup power, 99.9%+ uptime");

    let suitable_providers = find_suitable_providers(&critical_request, providers);
    println!("     Suitable providers: {}", suitable_providers.len());
    for provider in &suitable_providers {
        let file_size_gb = Decimal::new(small_file.size as i64, 9);
        let cost: Decimal = provider.calculate_monthly_cost(file_size_gb);
        println!(
            "       • {} (${}/month, {:.1}% reputation)",
            provider.name,
            cost,
            provider.reputation.overall * Decimal::new(100, 0)
        );
    }

    // Scenario 2: Backup data - optimize for cost
    let backup_request = StorageRequest::new(
        large_file.id,
        2,                  // 2 copies sufficient
        Decimal::new(3, 3), // Budget: $0.003/GB/month
        ProviderRequirements::backup(),
    )
    .expect("Should create backup request");

    println!("\n  💰 Scenario 2: Cost-Optimized Backup");
    println!(
        "     File: {} ({:.1} MB)",
        large_file.name,
        large_file.size as f64 / (1024.0 * 1024.0)
    );
    println!(
        "     Requirements: {} copies, max ${}/GB/month",
        backup_request.replication_factor, backup_request.max_price_per_gb_month
    );
    println!("     Requirements: Home providers OK, 90%+ uptime");

    let suitable_providers = find_suitable_providers(&backup_request, providers);
    println!("     Suitable providers: {}", suitable_providers.len());
    for provider in &suitable_providers {
        let file_size_gb = Decimal::new(large_file.size as i64, 9);
        let cost: Decimal = provider.calculate_monthly_cost(file_size_gb);
        println!(
            "       • {} (${:.4}/month, {:.1}% reputation)",
            provider.name,
            cost,
            provider.reputation.overall * Decimal::new(100, 0)
        );
    }

    // Calculate total costs
    let critical_file_size_gb = Decimal::new(small_file.size as i64, 9);
    let backup_file_size_gb = Decimal::new(large_file.size as i64, 9);

    let critical_budget = critical_request.calculate_monthly_budget(critical_file_size_gb);
    let backup_budget = backup_request.calculate_monthly_budget(backup_file_size_gb);

    println!("\n     💵 Cost Analysis:");
    println!(
        "       Critical file total budget: ${}/month",
        critical_budget
    );
    println!("       Backup file total budget: ${}/month", backup_budget);
    println!(
        "       Total monthly storage cost: ${}/month",
        critical_budget + backup_budget
    );
}

fn find_suitable_providers<'a>(
    request: &StorageRequest,
    providers: &'a [Provider],
) -> Vec<&'a Provider> {
    providers
        .iter()
        .filter(|provider| {
            // Check if provider meets requirements
            let meets_uptime = provider.reputation.uptime >= request.requirements.min_uptime;
            let meets_reputation =
                provider.reputation.overall >= request.requirements.min_reputation;
            let within_budget = provider.price_per_gb_month <= request.max_price_per_gb_month;
            let home_provider_ok =
                !request.requirements.exclude_home_providers || provider.tier != ProviderTier::Home;

            meets_uptime && meets_reputation && within_budget && home_provider_ok
        })
        .collect()
}

fn demonstrate_marketplace_economics(providers: &[Provider]) {
    println!("\n💰 Marketplace Economics:");

    // Show pricing comparison
    println!("\n  Provider Pricing Comparison (per GB/month):");
    for provider in providers {
        println!(
            "    {}: ${} ({:?} tier)",
            provider.name, provider.price_per_gb_month, provider.tier
        );
    }

    // Show typical earnings for providers
    println!("\n  Provider Earnings Potential (1TB storage):");
    let one_tb_gb = Decimal::new(1024, 0); // 1024 GB = 1TB
    for provider in providers {
        let monthly_revenue = provider.calculate_monthly_cost(one_tb_gb);
        let yearly_revenue = monthly_revenue * Decimal::new(12, 0);
        println!(
            "    {}: ${}/month, ${}/year",
            provider.name, monthly_revenue, yearly_revenue
        );
    }

    // Show cost comparison with traditional cloud storage
    println!("\n  💲 Cost vs. Traditional Cloud Storage:");
    println!("    Traditional AWS S3: ~$0.023/GB/month");
    println!("    Traditional Google Cloud: ~$0.020/GB/month");
    println!("    Traditional Azure: ~$0.018/GB/month");
    println!();

    for provider in providers {
        let savings_vs_aws = (Decimal::new(23, 3) - provider.price_per_gb_month)
            / Decimal::new(23, 3)
            * Decimal::new(100, 0);
        println!(
            "    {}: {:.0}% cheaper than AWS",
            provider.name, savings_vs_aws
        );
    }

    // Show network effects
    println!("\n  🌐 Network Effects:");
    println!("    • {} providers online", providers.len());
    println!(
        "    • Total network capacity: {:.1} GB",
        providers
            .iter()
            .map(|p| p.total_capacity as f64 / (1024.0 * 1024.0 * 1024.0))
            .sum::<f64>()
    );
    println!(
        "    • Average reputation: {:.2}",
        providers
            .iter()
            .map(|p| p.reputation.overall)
            .sum::<Decimal>()
            / Decimal::new(providers.len() as i64, 0)
    );
    println!(
        "    • Price range: ${:.3} - ${:.3}/GB/month",
        providers
            .iter()
            .map(|p| p.price_per_gb_month)
            .min()
            .unwrap(),
        providers
            .iter()
            .map(|p| p.price_per_gb_month)
            .max()
            .unwrap()
    );
}
