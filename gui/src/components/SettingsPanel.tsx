import React, { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/tauri";
import { ProviderConfig } from "../types";
import { Settings, Save, RotateCcw, AlertCircle } from "lucide-react";

interface SettingsPanelProps {
  onSave: () => void;
}

const SettingsPanel: React.FC<SettingsPanelProps> = ({ onSave }) => {
  const [config, setConfig] = useState<ProviderConfig | null>(null);
  const [originalConfig, setOriginalConfig] = useState<ProviderConfig | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [isSaving, setIsSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [successMessage, setSuccessMessage] = useState<string | null>(null);

  useEffect(() => {
    loadConfig();
  }, []);

  const loadConfig = async () => {
    try {
      const configData = await invoke<ProviderConfig>("get_config");
      setConfig(configData);
      setOriginalConfig(JSON.parse(JSON.stringify(configData))); // Deep clone
    } catch (error) {
      console.error("Failed to load config:", error);
      setError(String(error));
    } finally {
      setIsLoading(false);
    }
  };

  const saveConfig = async () => {
    if (!config) return;

    setIsSaving(true);
    setError(null);
    
    try {
      await invoke("save_config", { config });
      setOriginalConfig(JSON.parse(JSON.stringify(config))); // Update original
      setSuccessMessage("Configuration saved successfully!");
      setTimeout(() => setSuccessMessage(null), 3000);
      onSave();
    } catch (error) {
      console.error("Failed to save config:", error);
      setError(String(error));
    } finally {
      setIsSaving(false);
    }
  };

  const resetConfig = () => {
    if (originalConfig) {
      setConfig(JSON.parse(JSON.stringify(originalConfig)));
      setError(null);
      setSuccessMessage(null);
    }
  };

  const hasChanges = () => {
    return JSON.stringify(config) !== JSON.stringify(originalConfig);
  };

  const checkPortAvailability = async (port: number) => {
    try {
      const available = await invoke<boolean>("check_port_available", { port });
      return available;
    } catch {
      return false;
    }
  };

  const handlePortChange = async (newPort: number) => {
    if (!config) return;
    
    if (newPort !== config.provider.port) {
      const available = await checkPortAvailability(newPort);
      if (!available && newPort !== originalConfig?.provider.port) {
        setError(`Port ${newPort} is already in use`);
        return;
      }
    }
    
    setError(null);
    setConfig({
      ...config,
      provider: {
        ...config.provider,
        port: newPort
      },
      network: {
        ...config.network,
        advertise_address: `127.0.0.1:${newPort}`
      }
    });
  };

  if (isLoading) {
    return (
      <div className="bg-white rounded-xl border border-gray-200 p-6">
        <div className="text-center py-12">
          <div className="w-8 h-8 border-4 border-carbide-500 border-t-transparent rounded-full animate-spin mx-auto mb-4"></div>
          <p className="text-gray-500">Loading configuration...</p>
        </div>
      </div>
    );
  }

  if (!config) {
    return (
      <div className="bg-white rounded-xl border border-gray-200 p-6">
        <div className="text-center py-12">
          <AlertCircle className="w-12 h-12 text-red-500 mx-auto mb-4" />
          <h3 className="text-lg font-semibold text-gray-900 mb-2">Configuration Error</h3>
          <p className="text-gray-600 mb-4">Failed to load provider configuration</p>
          <button
            onClick={loadConfig}
            className="bg-carbide-500 hover:bg-carbide-600 text-white px-4 py-2 rounded-lg"
          >
            Retry
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="bg-white rounded-xl border border-gray-200">
      {/* Header */}
      <div className="p-6 border-b border-gray-200">
        <div className="flex items-center justify-between">
          <h2 className="text-xl font-semibold text-gray-900 flex items-center">
            <Settings className="w-5 h-5 mr-2" />
            Provider Configuration
          </h2>
          
          <div className="flex items-center space-x-3">
            {hasChanges() && (
              <>
                <button
                  onClick={resetConfig}
                  className="flex items-center space-x-2 px-3 py-2 text-gray-600 hover:text-gray-800 hover:bg-gray-100 rounded-lg transition-colors"
                >
                  <RotateCcw className="w-4 h-4" />
                  <span>Reset</span>
                </button>
                
                <button
                  onClick={saveConfig}
                  disabled={isSaving}
                  className="flex items-center space-x-2 bg-carbide-500 hover:bg-carbide-600 disabled:bg-carbide-300 text-white px-4 py-2 rounded-lg transition-colors"
                >
                  <Save className="w-4 h-4" />
                  <span>{isSaving ? 'Saving...' : 'Save Changes'}</span>
                </button>
              </>
            )}
          </div>
        </div>
      </div>

      {/* Content */}
      <div className="p-6 space-y-8">
        {/* Messages */}
        {error && (
          <div className="bg-red-50 border border-red-200 rounded-lg p-4">
            <div className="flex items-start">
              <AlertCircle className="w-5 h-5 text-red-600 mt-0.5 mr-3" />
              <div>
                <h4 className="text-red-800 font-medium">Configuration Error</h4>
                <p className="text-red-700 text-sm mt-1">{error}</p>
              </div>
            </div>
          </div>
        )}

        {successMessage && (
          <div className="bg-green-50 border border-green-200 rounded-lg p-4">
            <div className="flex items-center">
              <div className="w-5 h-5 text-green-600 mr-3">✓</div>
              <p className="text-green-800 font-medium">{successMessage}</p>
            </div>
          </div>
        )}

        {/* Provider Settings */}
        <div>
          <h3 className="text-lg font-medium text-gray-900 mb-4">Provider Settings</h3>
          <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-2">
                Provider Name
              </label>
              <input
                type="text"
                value={config.provider.name}
                onChange={(e) => setConfig({
                  ...config,
                  provider: { ...config.provider, name: e.target.value }
                })}
                className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-carbide-500 focus:border-carbide-500"
              />
            </div>

            <div>
              <label className="block text-sm font-medium text-gray-700 mb-2">
                Provider Tier
              </label>
              <select
                value={config.provider.tier}
                onChange={(e) => setConfig({
                  ...config,
                  provider: { ...config.provider, tier: e.target.value }
                })}
                className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-carbide-500 focus:border-carbide-500"
              >
                <option value="Home">Home</option>
                <option value="Professional">Professional</option>
                <option value="Enterprise">Enterprise</option>
                <option value="GlobalCDN">Global CDN</option>
              </select>
            </div>

            <div>
              <label className="block text-sm font-medium text-gray-700 mb-2">
                Region
              </label>
              <select
                value={config.provider.region}
                onChange={(e) => setConfig({
                  ...config,
                  provider: { ...config.provider, region: e.target.value }
                })}
                className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-carbide-500 focus:border-carbide-500"
              >
                <option value="NorthAmerica">North America</option>
                <option value="Europe">Europe</option>
                <option value="Asia">Asia</option>
                <option value="SouthAmerica">South America</option>
                <option value="Africa">Africa</option>
                <option value="Oceania">Oceania</option>
              </select>
            </div>

            <div>
              <label className="block text-sm font-medium text-gray-700 mb-2">
                Port
              </label>
              <input
                type="number"
                value={config.provider.port}
                onChange={(e) => handlePortChange(parseInt(e.target.value))}
                min="1024"
                max="65535"
                className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-carbide-500 focus:border-carbide-500"
              />
            </div>

            <div>
              <label className="block text-sm font-medium text-gray-700 mb-2">
                Storage Allocation (GB)
              </label>
              <input
                type="number"
                value={config.provider.max_storage_gb}
                onChange={(e) => setConfig({
                  ...config,
                  provider: { ...config.provider, max_storage_gb: parseInt(e.target.value) }
                })}
                min="1"
                max="1000"
                className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-carbide-500 focus:border-carbide-500"
              />
            </div>

            <div>
              <label className="block text-sm font-medium text-gray-700 mb-2">
                Price per GB per Month (USD)
              </label>
              <div className="relative">
                <span className="absolute left-3 top-2 text-gray-500">$</span>
                <input
                  type="number"
                  value={config.pricing.price_per_gb_month}
                  onChange={(e) => setConfig({
                    ...config,
                    pricing: { ...config.pricing, price_per_gb_month: parseFloat(e.target.value) }
                  })}
                  min="0.001"
                  max="1"
                  step="0.001"
                  className="w-full pl-8 pr-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-carbide-500 focus:border-carbide-500"
                />
              </div>
            </div>
          </div>
        </div>

        {/* Network Settings */}
        <div>
          <h3 className="text-lg font-medium text-gray-900 mb-4">Network Settings</h3>
          <div className="grid grid-cols-1 gap-6">
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-2">
                Discovery Endpoint
              </label>
              <input
                type="url"
                value={config.network.discovery_endpoint}
                onChange={(e) => setConfig({
                  ...config,
                  network: { ...config.network, discovery_endpoint: e.target.value }
                })}
                className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-carbide-500 focus:border-carbide-500"
                placeholder="http://discovery.carbide.network"
              />
            </div>

            <div>
              <label className="block text-sm font-medium text-gray-700 mb-2">
                Advertise Address
              </label>
              <input
                type="text"
                value={config.network.advertise_address}
                onChange={(e) => setConfig({
                  ...config,
                  network: { ...config.network, advertise_address: e.target.value }
                })}
                className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-carbide-500 focus:border-carbide-500"
                placeholder="127.0.0.1:8080"
              />
            </div>
          </div>
        </div>

        {/* Advanced Settings */}
        <div>
          <h3 className="text-lg font-medium text-gray-900 mb-4">Advanced Settings</h3>
          <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-2">
                Log Level
              </label>
              <select
                value={config.logging.level}
                onChange={(e) => setConfig({
                  ...config,
                  logging: { ...config.logging, level: e.target.value }
                })}
                className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-carbide-500 focus:border-carbide-500"
              >
                <option value="debug">Debug</option>
                <option value="info">Info</option>
                <option value="warn">Warning</option>
                <option value="error">Error</option>
              </select>
            </div>

            <div>
              <label className="block text-sm font-medium text-gray-700 mb-2">
                Health Check Interval (seconds)
              </label>
              <input
                type="number"
                value={config.reputation.health_check_interval}
                onChange={(e) => setConfig({
                  ...config,
                  reputation: { ...config.reputation, health_check_interval: parseInt(e.target.value) }
                })}
                min="30"
                max="3600"
                className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-carbide-500 focus:border-carbide-500"
              />
            </div>

            <div className="md:col-span-2">
              <label className="flex items-center space-x-2">
                <input
                  type="checkbox"
                  checked={config.reputation.enable_reporting}
                  onChange={(e) => setConfig({
                    ...config,
                    reputation: { ...config.reputation, enable_reporting: e.target.checked }
                  })}
                  className="rounded border-gray-300 text-carbide-600 focus:ring-carbide-500"
                />
                <span className="text-sm font-medium text-gray-700">
                  Enable reputation reporting
                </span>
              </label>
              <p className="text-sm text-gray-500 mt-1">
                Allow your provider to report health and performance metrics to the reputation system
              </p>
            </div>
          </div>
        </div>

        {/* Storage Path (read-only) */}
        <div>
          <h3 className="text-lg font-medium text-gray-900 mb-4">Storage Configuration</h3>
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-2">
              Storage Path (read-only)
            </label>
            <input
              type="text"
              value={config.provider.storage_path}
              readOnly
              className="w-full px-3 py-2 border border-gray-300 rounded-lg bg-gray-50 text-gray-600"
            />
          </div>
        </div>
      </div>
    </div>
  );
};

export default SettingsPanel;