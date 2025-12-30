import React from "react";
import { LineChart, Line, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer, PieChart, Pie, Cell } from "recharts";
import { DollarSign, TrendingUp, Star } from "lucide-react";

interface EarningsChartProps {
  dailyEarnings: number;
  monthlyEarnings: number;
  reputation: number;
}

const EarningsChart: React.FC<EarningsChartProps> = ({ 
  dailyEarnings, 
  monthlyEarnings, 
  reputation 
}) => {
  // Mock data for the chart - in a real app, this would come from historical data
  const earningsData = [
    { day: 'Mon', earnings: dailyEarnings * 0.8 },
    { day: 'Tue', earnings: dailyEarnings * 0.9 },
    { day: 'Wed', earnings: dailyEarnings * 1.1 },
    { day: 'Thu', earnings: dailyEarnings * 0.95 },
    { day: 'Fri', earnings: dailyEarnings * 1.2 },
    { day: 'Sat', earnings: dailyEarnings },
    { day: 'Today', earnings: dailyEarnings },
  ];

  const reputationData = [
    { name: 'Reputation', value: reputation * 100, color: '#0ea5e9' },
    { name: 'Remaining', value: (1 - reputation) * 100, color: '#e5e7eb' },
  ];

  return (
    <div className="bg-white p-6 rounded-xl border border-gray-200">
      <div className="flex items-center justify-between mb-6">
        <h3 className="text-lg font-semibold text-gray-900 flex items-center">
          <DollarSign className="w-5 h-5 mr-2" />
          Earnings Overview
        </h3>
        <div className="flex items-center space-x-2 text-sm">
          <TrendingUp className="w-4 h-4 text-green-500" />
          <span className="text-green-600 font-medium">+12% vs last week</span>
        </div>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Earnings Metrics */}
        <div className="space-y-4">
          <div className="bg-gradient-to-r from-green-50 to-green-100 p-4 rounded-lg">
            <div className="flex items-center justify-between">
              <div>
                <p className="text-sm text-green-700 font-medium">Today's Earnings</p>
                <p className="text-2xl font-bold text-green-800">${dailyEarnings.toFixed(4)}</p>
              </div>
              <div className="text-green-600">💰</div>
            </div>
          </div>

          <div className="bg-gradient-to-r from-carbide-50 to-carbide-100 p-4 rounded-lg">
            <div className="flex items-center justify-between">
              <div>
                <p className="text-sm text-carbide-700 font-medium">Monthly Potential</p>
                <p className="text-2xl font-bold text-carbide-800">${monthlyEarnings.toFixed(3)}</p>
              </div>
              <div className="text-carbide-600">📈</div>
            </div>
          </div>

          <div className="bg-gradient-to-r from-purple-50 to-purple-100 p-4 rounded-lg">
            <div className="flex items-center justify-between">
              <div>
                <p className="text-sm text-purple-700 font-medium">Reputation Score</p>
                <p className="text-2xl font-bold text-purple-800">{(reputation * 100).toFixed(1)}%</p>
              </div>
              <div className="flex items-center">
                <Star className="w-5 h-5 text-yellow-500 fill-current" />
              </div>
            </div>
          </div>
        </div>

        {/* Weekly Earnings Trend */}
        <div>
          <h4 className="text-sm font-medium text-gray-700 mb-3">Weekly Earnings Trend</h4>
          <div className="h-32">
            <ResponsiveContainer width="100%" height="100%">
              <LineChart data={earningsData}>
                <CartesianGrid strokeDasharray="3 3" stroke="#f3f4f6" />
                <XAxis 
                  dataKey="day" 
                  tick={{ fontSize: 12, fill: '#6b7280' }}
                  axisLine={false}
                  tickLine={false}
                />
                <YAxis 
                  tick={{ fontSize: 12, fill: '#6b7280' }}
                  axisLine={false}
                  tickLine={false}
                  tickFormatter={(value) => `$${value.toFixed(3)}`}
                />
                <Tooltip 
                  formatter={(value: number) => [`$${value.toFixed(4)}`, 'Earnings']}
                  labelStyle={{ color: '#374151' }}
                  contentStyle={{ 
                    backgroundColor: 'white', 
                    border: '1px solid #e5e7eb',
                    borderRadius: '8px',
                    fontSize: '12px'
                  }}
                />
                <Line 
                  type="monotone" 
                  dataKey="earnings" 
                  stroke="#0ea5e9" 
                  strokeWidth={2}
                  dot={{ fill: '#0ea5e9', strokeWidth: 2, r: 3 }}
                  activeDot={{ r: 5, fill: '#0ea5e9' }}
                />
              </LineChart>
            </ResponsiveContainer>
          </div>
        </div>
      </div>

      {/* Earnings Breakdown */}
      <div className="mt-6 pt-4 border-t border-gray-100">
        <h4 className="text-sm font-medium text-gray-700 mb-3">Earnings Breakdown</h4>
        <div className="grid grid-cols-3 gap-4">
          <div className="text-center">
            <p className="text-lg font-bold text-gray-900">${(dailyEarnings * 7).toFixed(3)}</p>
            <p className="text-xs text-gray-500">Weekly</p>
          </div>
          <div className="text-center">
            <p className="text-lg font-bold text-gray-900">${monthlyEarnings.toFixed(3)}</p>
            <p className="text-xs text-gray-500">Monthly</p>
          </div>
          <div className="text-center">
            <p className="text-lg font-bold text-gray-900">${(monthlyEarnings * 12).toFixed(2)}</p>
            <p className="text-xs text-gray-500">Yearly</p>
          </div>
        </div>
      </div>
    </div>
  );
};

export default EarningsChart;