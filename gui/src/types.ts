export interface ProviderStatus {
  running: boolean;
  port?: number;
  name: string;
  storage_used_gb: number;
  storage_total_gb: number;
  earnings_today: number;
  earnings_month: number;
  uptime_hours: number;
  connections: number;
  reputation_score: number;
}

export interface SystemMetrics {
  cpu_usage: number;
  memory_usage: number;
  disk_usage: number;
  network_in: number;
  network_out: number;
}

export interface InstallProgress {
  step: string;
  progress: number;
  message: string;
  completed: boolean;
  error?: string;
}

export interface ProviderConfig {
  provider: {
    name: string;
    tier: string;
    region: string;
    port: number;
    storage_path: string;
    max_storage_gb: number;
  };
  network: {
    discovery_endpoint: string;
    advertise_address: string;
  };
  pricing: {
    price_per_gb_month: number;
  };
  logging: {
    level: string;
    file: string;
  };
  reputation: {
    enable_reporting: boolean;
    health_check_interval: number;
  };
}