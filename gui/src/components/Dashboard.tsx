import React, { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/tauri";
import { ProviderStatus, SystemMetrics } from "../types";
import StatusCard from "./StatusCard";
import EarningsChart from "./EarningsChart";
import SystemMetricsCard from "./SystemMetricsCard";
import StorageCard from "./StorageCard";
import LogsPanel from "./LogsPanel";
import SettingsPanel from "./SettingsPanel";
import { 
  Play, 
  Pause, 
  Settings, 
  BarChart3, 
  HardDrive, 
  Activity,
  DollarSign,
  Users,
  Wifi,
  AlertTriangle
} from "lucide-react";

const Dashboard: React.FC = () => {
  const [status, setStatus] = useState<ProviderStatus | null>(null);
  const [metrics, setMetrics] = useState<SystemMetrics | null>(null);
  const [activeTab, setActiveTab] = useState<'overview' | 'settings' | 'logs'>('overview');
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    loadData();
    
    // Set up periodic updates
    const interval = setInterval(loadData, 5000); // Update every 5 seconds
    
    return () => clearInterval(interval);
  }, []);

  const loadData = async () => {
    try {
      const [providerStatus, systemMetrics] = await Promise.all([
        invoke<ProviderStatus>("get_provider_status"),
        invoke<SystemMetrics>("get_system_metrics")
      ]);
      
      setStatus(providerStatus);
      setMetrics(systemMetrics);
      setError(null);
    } catch (error) {
      console.error("Failed to load data:", error);
      setError(String(error));
    } finally {
      setIsLoading(false);
    }
  };

  const handleStartStop = async () => {
    if (!status) return;
    
    try {
      if (status.running) {
        await invoke("stop_provider");
      } else {
        await invoke("start_provider");
      }
      
      // Reload status after action
      setTimeout(loadData, 1000);
    } catch (error) {
      console.error("Failed to start/stop provider:", error);
      setError(String(error));
    }
  };

  const openStorageFolder = async () => {
    try {
      await invoke("open_storage_folder");
    } catch (error) {
      console.error("Failed to open storage folder:", error);
    }
  };

  if (isLoading) {
    return (
      <div className="min-h-screen bg-gray-50 flex items-center justify-center">
        <div className="text-center">
          <div className="w-12 h-12 border-4 border-carbide-500 border-t-transparent rounded-full animate-spin mx-auto mb-4"></div>
          <p className="text-gray-600 font-medium">Loading dashboard...</p>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="min-h-screen bg-gray-50 flex items-center justify-center">
        <div className="text-center max-w-md">
          <AlertTriangle className="w-12 h-12 text-red-500 mx-auto mb-4" />
          <h2 className="text-xl font-semibold text-gray-900 mb-2">Connection Error</h2>
          <p className="text-gray-600 mb-4">{error}</p>
          <button
            onClick={loadData}
            className="bg-carbide-500 hover:bg-carbide-600 text-white px-4 py-2 rounded-lg"
          >
            Retry
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-gray-50">
      {/* Header */}
      <div className="bg-white border-b border-gray-200">
        <div className="px-6 py-4">
          <div className="flex items-center justify-between">
            <div className="flex items-center space-x-4">
              <div className="w-10 h-10 bg-carbide-500 rounded-xl flex items-center justify-center">
                <span className="text-white font-bold">C</span>
              </div>
              <div>
                <h1 className="text-xl font-semibold text-gray-900">
                  {status?.name || 'Carbide Provider'}
                </h1>
                <div className="flex items-center space-x-2 text-sm">
                  <div className={`w-2 h-2 rounded-full ${
                    status?.running ? 'bg-green-500 pulse-green' : 'bg-gray-400'
                  }`} />
                  <span className="text-gray-600">
                    {status?.running ? 'Online' : 'Offline'}
                  </span>
                  {status?.port && (
                    <>
                      <span className="text-gray-400">•</span>
                      <span className="text-gray-600">Port {status.port}</span>
                    </>
                  )}
                </div>
              </div>
            </div>
            
            <div className="flex items-center space-x-3">
              {/* Tab Navigation */}
              <div className="flex bg-gray-100 rounded-lg p-1">
                <button
                  onClick={() => setActiveTab('overview')}
                  className={`px-4 py-2 rounded-md text-sm font-medium transition-colors ${
                    activeTab === 'overview'
                      ? 'bg-white text-gray-900 shadow-sm'
                      : 'text-gray-600 hover:text-gray-900'
                  }`}
                >
                  Overview
                </button>
                <button
                  onClick={() => setActiveTab('settings')}
                  className={`px-4 py-2 rounded-md text-sm font-medium transition-colors ${
                    activeTab === 'settings'
                      ? 'bg-white text-gray-900 shadow-sm'
                      : 'text-gray-600 hover:text-gray-900'
                  }`}
                >
                  Settings
                </button>
                <button
                  onClick={() => setActiveTab('logs')}
                  className={`px-4 py-2 rounded-md text-sm font-medium transition-colors ${
                    activeTab === 'logs'
                      ? 'bg-white text-gray-900 shadow-sm'
                      : 'text-gray-600 hover:text-gray-900'
                  }`}
                >
                  Logs
                </button>
              </div>
              
              {/* Action Button */}
              <button
                onClick={handleStartStop}
                className={`flex items-center space-x-2 px-4 py-2 rounded-lg font-medium transition-colors ${
                  status?.running
                    ? 'bg-red-500 hover:bg-red-600 text-white'
                    : 'bg-green-500 hover:bg-green-600 text-white'
                }`}
              >
                {status?.running ? (
                  <>
                    <Pause className="w-4 h-4" />
                    <span>Stop</span>
                  </>
                ) : (
                  <>
                    <Play className="w-4 h-4" />
                    <span>Start</span>
                  </>
                )}
              </button>
            </div>
          </div>
        </div>
      </div>

      {/* Content */}
      <div className="p-6">
        {activeTab === 'overview' && (
          <div className="space-y-6">
            {/* Quick Stats */}
            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
              <StatusCard
                title="Status"
                value={status?.running ? 'Online' : 'Offline'}
                icon={<Activity className="w-5 h-5" />}
                status={status?.running ? 'success' : 'error'}
              />
              
              <StatusCard
                title="Connections"
                value={status?.connections.toString() || '0'}
                icon={<Wifi className="w-5 h-5" />}
                status={status?.connections ? 'success' : 'warning'}
              />
              
              <StatusCard
                title="Today's Earnings"
                value={`$${status?.earnings_today.toFixed(4) || '0.0000'}`}
                icon={<DollarSign className="w-5 h-5" />}
                status="info"
              />
              
              <StatusCard
                title="Reputation"
                value={`${((status?.reputation_score || 0) * 100).toFixed(1)}%`}
                icon={<BarChart3 className="w-5 h-5" />}
                status="info"
              />
            </div>

            {/* Main Content Grid */}
            <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
              {/* Storage Usage */}
              <StorageCard 
                used={status?.storage_used_gb || 0}
                total={status?.storage_total_gb || 25}
                onOpenFolder={openStorageFolder}
              />
              
              {/* Earnings Chart */}
              <EarningsChart 
                dailyEarnings={status?.earnings_today || 0}
                monthlyEarnings={status?.earnings_month || 0}
                reputation={status?.reputation_score || 0}
              />
            </div>

            {/* System Metrics */}
            <SystemMetricsCard metrics={metrics} />
          </div>
        )}

        {activeTab === 'settings' && (
          <SettingsPanel onSave={() => loadData()} />
        )}

        {activeTab === 'logs' && (
          <LogsPanel />
        )}
      </div>
    </div>
  );
};

export default Dashboard;