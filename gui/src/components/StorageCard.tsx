import React from "react";
import { HardDrive, FolderOpen } from "lucide-react";

interface StorageCardProps {
  used: number;
  total: number;
  onOpenFolder: () => void;
}

const StorageCard: React.FC<StorageCardProps> = ({ used, total, onOpenFolder }) => {
  const usagePercentage = total > 0 ? (used / total) * 100 : 0;
  
  const getUsageColor = () => {
    if (usagePercentage < 70) return 'bg-green-500';
    if (usagePercentage < 90) return 'bg-yellow-500';
    return 'bg-red-500';
  };

  return (
    <div className="bg-white p-6 rounded-xl border border-gray-200">
      <div className="flex items-center justify-between mb-4">
        <h3 className="text-lg font-semibold text-gray-900 flex items-center">
          <HardDrive className="w-5 h-5 mr-2" />
          Storage Usage
        </h3>
        <button
          onClick={onOpenFolder}
          className="flex items-center space-x-1 text-carbide-600 hover:text-carbide-700 text-sm font-medium"
        >
          <FolderOpen className="w-4 h-4" />
          <span>Open Folder</span>
        </button>
      </div>

      <div className="space-y-4">
        <div className="flex items-center justify-between text-sm">
          <span className="text-gray-600">Used: {used.toFixed(2)} GB</span>
          <span className="text-gray-600">Available: {(total - used).toFixed(2)} GB</span>
        </div>

        <div className="w-full bg-gray-200 rounded-full h-4">
          <div
            className={`h-4 rounded-full transition-all duration-500 ${getUsageColor()}`}
            style={{ width: `${Math.min(usagePercentage, 100)}%` }}
          />
        </div>

        <div className="flex items-center justify-between">
          <span className="text-lg font-bold text-gray-900">
            {usagePercentage.toFixed(1)}% used
          </span>
          <span className="text-sm text-gray-500">
            {total} GB total
          </span>
        </div>

        <div className="grid grid-cols-3 gap-4 pt-4 border-t border-gray-100">
          <div className="text-center">
            <p className="text-2xl font-bold text-gray-900">{used > 0 ? Math.ceil(used * 1000 / 100) : 0}</p>
            <p className="text-xs text-gray-500">Files Stored</p>
          </div>
          <div className="text-center">
            <p className="text-2xl font-bold text-gray-900">{(total - used).toFixed(0)}</p>
            <p className="text-xs text-gray-500">GB Free</p>
          </div>
          <div className="text-center">
            <p className="text-2xl font-bold text-green-600">
              {usagePercentage > 80 ? '🔥' : usagePercentage > 50 ? '📈' : '📊'}
            </p>
            <p className="text-xs text-gray-500">Status</p>
          </div>
        </div>
      </div>
    </div>
  );
};

export default StorageCard;