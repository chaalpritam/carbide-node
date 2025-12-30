import React, { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/tauri";
import { FileText, Download, Trash2, RefreshCw } from "lucide-react";

const LogsPanel: React.FC = () => {
  const [logs, setLogs] = useState<string[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [autoRefresh, setAutoRefresh] = useState(true);
  const [filter, setFilter] = useState<string>('');
  const logsEndRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    loadLogs();
    
    if (autoRefresh) {
      const interval = setInterval(loadLogs, 2000); // Refresh every 2 seconds
      return () => clearInterval(interval);
    }
  }, [autoRefresh]);

  useEffect(() => {
    // Auto-scroll to bottom when new logs arrive
    if (logsEndRef.current) {
      logsEndRef.current.scrollIntoView({ behavior: 'smooth' });
    }
  }, [logs]);

  const loadLogs = async () => {
    try {
      const logLines = await invoke<string[]>("get_logs", { lines: 100 });
      setLogs(logLines);
    } catch (error) {
      console.error("Failed to load logs:", error);
      setLogs([`Error loading logs: ${error}`]);
    } finally {
      setIsLoading(false);
    }
  };

  const downloadLogs = async () => {
    try {
      const allLogs = await invoke<string[]>("get_logs", { lines: 1000 });
      const logContent = allLogs.join('\n');
      
      // Create and download file
      const blob = new Blob([logContent], { type: 'text/plain' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `carbide-provider-logs-${new Date().toISOString().split('T')[0]}.txt`;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);
    } catch (error) {
      console.error("Failed to download logs:", error);
    }
  };

  const clearLogs = async () => {
    if (window.confirm('Are you sure you want to clear the logs? This action cannot be undone.')) {
      try {
        // In a real implementation, you would have a clear_logs command
        setLogs(['Logs cleared by user']);
      } catch (error) {
        console.error("Failed to clear logs:", error);
      }
    }
  };

  const getLogStyle = (line: string) => {
    if (line.includes('ERROR') || line.includes('error') || line.includes('Error')) {
      return 'text-red-600 bg-red-50';
    }
    if (line.includes('WARN') || line.includes('warn') || line.includes('Warning')) {
      return 'text-yellow-700 bg-yellow-50';
    }
    if (line.includes('INFO') || line.includes('Started') || line.includes('Listening')) {
      return 'text-green-700 bg-green-50';
    }
    if (line.includes('DEBUG') || line.includes('debug')) {
      return 'text-blue-600 bg-blue-50';
    }
    return 'text-gray-700';
  };

  const filteredLogs = logs.filter(line => 
    !filter || line.toLowerCase().includes(filter.toLowerCase())
  );

  return (
    <div className="bg-white rounded-xl border border-gray-200 h-full">
      {/* Header */}
      <div className="p-6 border-b border-gray-200">
        <div className="flex items-center justify-between">
          <h2 className="text-xl font-semibold text-gray-900 flex items-center">
            <FileText className="w-5 h-5 mr-2" />
            Provider Logs
          </h2>
          
          <div className="flex items-center space-x-3">
            {/* Auto-refresh toggle */}
            <label className="flex items-center space-x-2">
              <input
                type="checkbox"
                checked={autoRefresh}
                onChange={(e) => setAutoRefresh(e.target.checked)}
                className="rounded border-gray-300 text-carbide-600 focus:ring-carbide-500"
              />
              <span className="text-sm text-gray-600">Auto-refresh</span>
            </label>
            
            {/* Action buttons */}
            <button
              onClick={loadLogs}
              disabled={isLoading}
              className="p-2 text-gray-600 hover:text-gray-900 hover:bg-gray-100 rounded-lg transition-colors"
              title="Refresh logs"
            >
              <RefreshCw className={`w-4 h-4 ${isLoading ? 'animate-spin' : ''}`} />
            </button>
            
            <button
              onClick={downloadLogs}
              className="p-2 text-gray-600 hover:text-gray-900 hover:bg-gray-100 rounded-lg transition-colors"
              title="Download logs"
            >
              <Download className="w-4 h-4" />
            </button>
            
            <button
              onClick={clearLogs}
              className="p-2 text-red-600 hover:text-red-700 hover:bg-red-50 rounded-lg transition-colors"
              title="Clear logs"
            >
              <Trash2 className="w-4 h-4" />
            </button>
          </div>
        </div>

        {/* Filter */}
        <div className="mt-4">
          <input
            type="text"
            placeholder="Filter logs..."
            value={filter}
            onChange={(e) => setFilter(e.target.value)}
            className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-carbide-500 focus:border-carbide-500 text-sm"
          />
        </div>
      </div>

      {/* Logs content */}
      <div className="p-6">
        {isLoading ? (
          <div className="text-center py-12">
            <div className="w-8 h-8 border-4 border-carbide-500 border-t-transparent rounded-full animate-spin mx-auto mb-4"></div>
            <p className="text-gray-500">Loading logs...</p>
          </div>
        ) : (
          <div className="bg-gray-900 rounded-lg p-4 h-96 overflow-y-auto font-mono text-sm">
            {filteredLogs.length > 0 ? (
              <div className="space-y-1">
                {filteredLogs.map((line, index) => (
                  <div
                    key={index}
                    className={`px-2 py-1 rounded ${getLogStyle(line)} whitespace-pre-wrap break-all`}
                  >
                    {line}
                  </div>
                ))}
                <div ref={logsEndRef} />
              </div>
            ) : (
              <div className="text-center py-8">
                <FileText className="w-12 h-12 text-gray-400 mx-auto mb-3" />
                <p className="text-gray-400">
                  {filter ? 'No logs match your filter' : 'No logs available yet'}
                </p>
                {!filter && (
                  <p className="text-gray-500 text-xs mt-2">
                    Logs will appear here once the provider starts
                  </p>
                )}
              </div>
            )}
          </div>
        )}

        {/* Log stats */}
        <div className="mt-4 flex items-center justify-between text-sm text-gray-500">
          <span>
            Showing {filteredLogs.length} of {logs.length} log entries
            {filter && ` (filtered by "${filter}")`}
          </span>
          <span>
            Last updated: {new Date().toLocaleTimeString()}
          </span>
        </div>
      </div>
    </div>
  );
};

export default LogsPanel;