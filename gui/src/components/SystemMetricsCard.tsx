import React from "react";
import { SystemMetrics } from "../types";
import { Cpu, MemoryStick, HardDrive, Activity } from "lucide-react";

interface SystemMetricsCardProps {
  metrics: SystemMetrics | null;
}

const SystemMetricsCard: React.FC<SystemMetricsCardProps> = ({ metrics }) => {
  if (!metrics) {
    return (
      <div className="bg-white p-6 rounded-xl border border-gray-200">
        <h3 className="text-lg font-semibold text-gray-900 mb-4 flex items-center">
          <Activity className="w-5 h-5 mr-2" />
          System Performance
        </h3>
        <div className="text-center py-8">
          <div className="w-8 h-8 border-4 border-carbide-500 border-t-transparent rounded-full animate-spin mx-auto mb-4"></div>
          <p className="text-gray-500">Loading system metrics...</p>
        </div>
      </div>
    );
  }

  const getUsageColor = (usage: number) => {
    if (usage < 60) return 'text-green-600 bg-green-100';
    if (usage < 80) return 'text-yellow-600 bg-yellow-100';
    return 'text-red-600 bg-red-100';
  };

  const getProgressColor = (usage: number) => {
    if (usage < 60) return 'bg-green-500';
    if (usage < 80) return 'bg-yellow-500';
    return 'bg-red-500';
  };

  const MetricCard: React.FC<{
    title: string;
    value: number;
    icon: React.ReactNode;
    unit: string;
  }> = ({ title, value, icon, unit }) => (
    <div className="bg-gray-50 p-4 rounded-lg">
      <div className="flex items-center justify-between mb-3">
        <div className="flex items-center space-x-2">
          <div className={`w-8 h-8 rounded-lg flex items-center justify-center ${getUsageColor(value)}`}>
            {icon}
          </div>
          <span className="font-medium text-gray-900">{title}</span>
        </div>
        <span className="text-lg font-bold text-gray-900">
          {value.toFixed(1)}{unit}
        </span>
      </div>
      
      <div className="w-full bg-gray-200 rounded-full h-2">
        <div
          className={`h-2 rounded-full transition-all duration-500 ${getProgressColor(value)}`}
          style={{ width: `${Math.min(value, 100)}%` }}
        />
      </div>
      
      <div className="flex justify-between text-xs text-gray-500 mt-1">
        <span>0{unit}</span>
        <span>100{unit}</span>
      </div>
    </div>
  );

  return (
    <div className="bg-white p-6 rounded-xl border border-gray-200">
      <h3 className="text-lg font-semibold text-gray-900 mb-6 flex items-center">
        <Activity className="w-5 h-5 mr-2" />
        System Performance
      </h3>

      <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
        <MetricCard
          title="CPU Usage"
          value={metrics.cpu_usage}
          icon={<Cpu className="w-4 h-4" />}
          unit="%"
        />
        
        <MetricCard
          title="Memory Usage"
          value={metrics.memory_usage}
          icon={<MemoryStick className="w-4 h-4" />}
          unit="%"
        />
        
        <MetricCard
          title="Disk Usage"
          value={metrics.disk_usage}
          icon={<HardDrive className="w-4 h-4" />}
          unit="%"
        />
      </div>

      {/* System Health Summary */}
      <div className="mt-6 pt-4 border-t border-gray-100">
        <div className="flex items-center justify-between">
          <span className="text-sm font-medium text-gray-700">Overall System Health</span>
          <div className="flex items-center space-x-2">
            {(() => {
              const avgUsage = (metrics.cpu_usage + metrics.memory_usage + metrics.disk_usage) / 3;
              if (avgUsage < 60) {
                return (
                  <>
                    <div className="w-2 h-2 bg-green-500 rounded-full"></div>
                    <span className="text-green-600 font-medium">Excellent</span>
                  </>
                );
              } else if (avgUsage < 80) {
                return (
                  <>
                    <div className="w-2 h-2 bg-yellow-500 rounded-full"></div>
                    <span className="text-yellow-600 font-medium">Good</span>
                  </>
                );
              } else {
                return (
                  <>
                    <div className="w-2 h-2 bg-red-500 rounded-full"></div>
                    <span className="text-red-600 font-medium">High Load</span>
                  </>
                );
              }
            })()}
          </div>
        </div>
      </div>

      {/* Network Stats (if available) */}
      {metrics.network_in > 0 || metrics.network_out > 0 ? (
        <div className="mt-4 pt-4 border-t border-gray-100">
          <h4 className="text-sm font-medium text-gray-700 mb-3">Network Activity</h4>
          <div className="grid grid-cols-2 gap-4">
            <div className="text-center">
              <p className="text-lg font-bold text-gray-900">
                {(metrics.network_in / 1024 / 1024).toFixed(1)} MB
              </p>
              <p className="text-xs text-gray-500">Downloaded</p>
            </div>
            <div className="text-center">
              <p className="text-lg font-bold text-gray-900">
                {(metrics.network_out / 1024 / 1024).toFixed(1)} MB
              </p>
              <p className="text-xs text-gray-500">Uploaded</p>
            </div>
          </div>
        </div>
      ) : null}
    </div>
  );
};

export default SystemMetricsCard;