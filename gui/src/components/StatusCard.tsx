import React from "react";

interface StatusCardProps {
  title: string;
  value: string;
  icon: React.ReactNode;
  status: 'success' | 'warning' | 'error' | 'info';
  subtitle?: string;
}

const StatusCard: React.FC<StatusCardProps> = ({ 
  title, 
  value, 
  icon, 
  status, 
  subtitle 
}) => {
  const getStatusColors = () => {
    switch (status) {
      case 'success':
        return 'text-green-600 bg-green-100';
      case 'warning':
        return 'text-yellow-600 bg-yellow-100';
      case 'error':
        return 'text-red-600 bg-red-100';
      case 'info':
        return 'text-carbide-600 bg-carbide-100';
      default:
        return 'text-gray-600 bg-gray-100';
    }
  };

  const getValueColors = () => {
    switch (status) {
      case 'success':
        return 'text-green-700';
      case 'warning':
        return 'text-yellow-700';
      case 'error':
        return 'text-red-700';
      case 'info':
        return 'text-carbide-700';
      default:
        return 'text-gray-700';
    }
  };

  return (
    <div className="bg-white p-6 rounded-xl border border-gray-200 hover:shadow-lg transition-shadow">
      <div className="flex items-center justify-between">
        <div className="flex-1">
          <p className="text-sm font-medium text-gray-600">{title}</p>
          <p className={`text-2xl font-bold ${getValueColors()}`}>
            {value}
          </p>
          {subtitle && (
            <p className="text-sm text-gray-500 mt-1">{subtitle}</p>
          )}
        </div>
        <div className={`w-12 h-12 rounded-lg flex items-center justify-center ${getStatusColors()}`}>
          {icon}
        </div>
      </div>
    </div>
  );
};

export default StatusCard;