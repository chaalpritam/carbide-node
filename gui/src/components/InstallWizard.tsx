import React, { useState } from "react";
import { invoke } from "@tauri-apps/api/tauri";
import { listen } from "@tauri-apps/api/event";
import { InstallProgress } from "../types";
import { CheckCircle, Circle, AlertCircle, Download, Settings, Zap } from "lucide-react";

interface InstallWizardProps {
  onComplete: () => void;
}

interface WizardStep {
  id: string;
  title: string;
  description: string;
  icon: React.ReactNode;
}

const steps: WizardStep[] = [
  {
    id: "welcome",
    title: "Welcome to Carbide",
    description: "Transform your Mac mini into a profitable storage provider",
    icon: <Zap className="w-6 h-6" />
  },
  {
    id: "configure", 
    title: "Configure Provider",
    description: "Set up your storage allocation and pricing",
    icon: <Settings className="w-6 h-6" />
  },
  {
    id: "install",
    title: "Install Carbide",
    description: "Download and install the Carbide provider",
    icon: <Download className="w-6 h-6" />
  },
  {
    id: "complete",
    title: "Ready to Earn",
    description: "Your provider is ready to start earning!",
    icon: <CheckCircle className="w-6 h-6" />
  }
];

const InstallWizard: React.FC<InstallWizardProps> = ({ onComplete }) => {
  const [currentStep, setCurrentStep] = useState(0);
  const [installProgress, setInstallProgress] = useState<InstallProgress | null>(null);
  const [isInstalling, setIsInstalling] = useState(false);
  
  // Configuration state
  const [config, setConfig] = useState({
    providerName: `${getUserDisplayName()}-carbide-provider`,
    storageGB: 25,
    pricePerGB: 0.005,
    tier: "Home",
    region: "NorthAmerica"
  });

  React.useEffect(() => {
    const unlisten = listen<InstallProgress>("install-progress", (event) => {
      setInstallProgress(event.payload);
      
      if (event.payload.completed) {
        setTimeout(() => {
          setCurrentStep(3); // Complete step
          setIsInstalling(false);
        }, 1000);
      }
    });

    return () => {
      unlisten.then((unlistenFn) => unlistenFn());
    };
  }, []);

  const handleNext = () => {
    if (currentStep < steps.length - 1) {
      setCurrentStep(currentStep + 1);
    }
  };

  const handleBack = () => {
    if (currentStep > 0) {
      setCurrentStep(currentStep - 1);
    }
  };

  const handleInstall = async () => {
    setIsInstalling(true);
    setCurrentStep(2); // Install step
    
    try {
      await invoke("install_carbide", {
        storageGb: config.storageGB,
        providerName: config.providerName,
        pricePerGb: config.pricePerGB
      });
    } catch (error) {
      console.error("Installation failed:", error);
      setInstallProgress({
        step: "Installation failed",
        progress: 0,
        message: `Error: ${error}`,
        completed: false,
        error: String(error)
      });
      setIsInstalling(false);
    }
  };

  const renderWelcomeStep = () => (
    <div className="text-center space-y-8">
      <div className="w-32 h-32 mx-auto bg-gradient-to-br from-carbide-500 to-carbide-600 rounded-full flex items-center justify-center">
        <Zap className="w-16 h-16 text-white" />
      </div>
      
      <div className="space-y-4">
        <h2 className="text-3xl font-bold text-gray-900">
          Welcome to Carbide Network
        </h2>
        <p className="text-xl text-gray-600 max-w-lg mx-auto">
          Turn your Mac mini into a profitable storage provider and earn passive income 
          by contributing to the decentralized storage marketplace.
        </p>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-3 gap-6 max-w-4xl mx-auto">
        <div className="bg-white p-6 rounded-xl shadow-sm border border-gray-100">
          <div className="w-12 h-12 bg-green-100 rounded-lg flex items-center justify-center mb-4">
            <Circle className="w-6 h-6 text-green-600" />
          </div>
          <h3 className="font-semibold text-gray-900 mb-2">Easy Setup</h3>
          <p className="text-gray-600 text-sm">
            Simple 5-minute installation process with automatic configuration
          </p>
        </div>
        
        <div className="bg-white p-6 rounded-xl shadow-sm border border-gray-100">
          <div className="w-12 h-12 bg-carbide-100 rounded-lg flex items-center justify-center mb-4">
            <Download className="w-6 h-6 text-carbide-600" />
          </div>
          <h3 className="font-semibold text-gray-900 mb-2">Passive Income</h3>
          <p className="text-gray-600 text-sm">
            Earn money 24/7 by providing storage to the network
          </p>
        </div>
        
        <div className="bg-white p-6 rounded-xl shadow-sm border border-gray-100">
          <div className="w-12 h-12 bg-purple-100 rounded-lg flex items-center justify-center mb-4">
            <Settings className="w-6 h-6 text-purple-600" />
          </div>
          <h3 className="font-semibold text-gray-900 mb-2">Auto Management</h3>
          <p className="text-gray-600 text-sm">
            Automatic startup, monitoring, and optimization
          </p>
        </div>
      </div>

      <div className="bg-carbide-50 p-6 rounded-xl border border-carbide-200">
        <h4 className="font-semibold text-carbide-900 mb-2">💰 Earning Potential</h4>
        <p className="text-carbide-700">
          With 25GB allocated at $0.005/GB/month, you could earn up to <span className="font-bold">$0.125/month</span> when fully utilized
        </p>
      </div>
    </div>
  );

  const renderConfigureStep = () => (
    <div className="space-y-8">
      <div className="text-center">
        <h2 className="text-2xl font-bold text-gray-900 mb-2">
          Configure Your Provider
        </h2>
        <p className="text-gray-600">
          Customize your storage provider settings
        </p>
      </div>

      <div className="max-w-lg mx-auto space-y-6">
        <div>
          <label className="block text-sm font-medium text-gray-700 mb-2">
            Provider Name
          </label>
          <input
            type="text"
            value={config.providerName}
            onChange={(e) => setConfig({ ...config, providerName: e.target.value })}
            className="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-carbide-500 focus:border-carbide-500"
            placeholder="My Carbide Provider"
          />
        </div>

        <div>
          <label className="block text-sm font-medium text-gray-700 mb-2">
            Storage Allocation (GB)
          </label>
          <div className="relative">
            <input
              type="number"
              value={config.storageGB}
              onChange={(e) => setConfig({ ...config, storageGB: parseInt(e.target.value) })}
              className="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-carbide-500 focus:border-carbide-500"
              min="1"
              max="100"
            />
            <span className="absolute right-3 top-2 text-gray-500">GB</span>
          </div>
          <p className="text-sm text-gray-500 mt-1">
            Recommended: 25GB for Mac mini
          </p>
        </div>

        <div>
          <label className="block text-sm font-medium text-gray-700 mb-2">
            Price per GB per Month (USD)
          </label>
          <div className="relative">
            <span className="absolute left-3 top-2 text-gray-500">$</span>
            <input
              type="number"
              value={config.pricePerGB}
              onChange={(e) => setConfig({ ...config, pricePerGB: parseFloat(e.target.value) })}
              className="w-full pl-8 pr-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-carbide-500 focus:border-carbide-500"
              min="0.001"
              max="0.1"
              step="0.001"
            />
          </div>
          <p className="text-sm text-gray-500 mt-1">
            Market rate: $0.005 (recommended)
          </p>
        </div>

        <div>
          <label className="block text-sm font-medium text-gray-700 mb-2">
            Provider Tier
          </label>
          <select
            value={config.tier}
            onChange={(e) => setConfig({ ...config, tier: e.target.value })}
            className="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-carbide-500 focus:border-carbide-500"
          >
            <option value="Home">Home (Mac mini, personal use)</option>
            <option value="Professional">Professional</option>
            <option value="Enterprise">Enterprise</option>
          </select>
        </div>

        <div className="bg-gray-50 p-4 rounded-lg">
          <h4 className="font-medium text-gray-900 mb-2">Earning Estimate</h4>
          <div className="space-y-1 text-sm">
            <div className="flex justify-between">
              <span className="text-gray-600">Max Monthly:</span>
              <span className="font-medium">${(config.storageGB * config.pricePerGB).toFixed(3)}</span>
            </div>
            <div className="flex justify-between">
              <span className="text-gray-600">Daily Potential:</span>
              <span className="font-medium">${(config.storageGB * config.pricePerGB / 30).toFixed(4)}</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  );

  const renderInstallStep = () => (
    <div className="text-center space-y-6">
      <h2 className="text-2xl font-bold text-gray-900">
        Installing Carbide Provider
      </h2>
      
      {installProgress && (
        <div className="max-w-lg mx-auto">
          <div className="bg-white p-6 rounded-xl border border-gray-200">
            <div className="space-y-4">
              <div className="flex items-center justify-between">
                <span className="text-sm font-medium text-gray-700">
                  {installProgress.step}
                </span>
                <span className="text-sm text-gray-500">
                  {installProgress.progress}%
                </span>
              </div>
              
              <div className="w-full bg-gray-200 rounded-full h-3">
                <div
                  className="bg-carbide-500 h-3 rounded-full transition-all duration-500"
                  style={{ width: `${installProgress.progress}%` }}
                />
              </div>
              
              <p className="text-sm text-gray-600">
                {installProgress.message}
              </p>
              
              {installProgress.error && (
                <div className="bg-red-50 border border-red-200 rounded-lg p-4">
                  <div className="flex items-start">
                    <AlertCircle className="w-5 h-5 text-red-600 mt-0.5 mr-3" />
                    <div>
                      <h4 className="text-red-800 font-medium">Installation Error</h4>
                      <p className="text-red-700 text-sm mt-1">{installProgress.error}</p>
                    </div>
                  </div>
                </div>
              )}
            </div>
          </div>
        </div>
      )}
      
      {!installProgress && (
        <div className="w-16 h-16 border-4 border-carbide-500 border-t-transparent rounded-full animate-spin mx-auto" />
      )}
    </div>
  );

  const renderCompleteStep = () => (
    <div className="text-center space-y-8">
      <div className="w-32 h-32 mx-auto bg-gradient-to-br from-green-500 to-green-600 rounded-full flex items-center justify-center">
        <CheckCircle className="w-16 h-16 text-white" />
      </div>
      
      <div className="space-y-4">
        <h2 className="text-3xl font-bold text-gray-900">
          🎉 Installation Complete!
        </h2>
        <p className="text-xl text-gray-600">
          Your Carbide provider is ready to start earning
        </p>
      </div>

      <div className="bg-green-50 p-6 rounded-xl border border-green-200 max-w-lg mx-auto">
        <h4 className="font-semibold text-green-900 mb-2">What happens next?</h4>
        <ul className="text-green-800 text-sm space-y-2 text-left">
          <li>• Your provider will automatically start on boot</li>
          <li>• Files will be stored in your allocated 25GB space</li>
          <li>• Earnings will accumulate as clients use your storage</li>
          <li>• Monitor everything from the dashboard</li>
        </ul>
      </div>

      <button
        onClick={onComplete}
        className="bg-carbide-500 hover:bg-carbide-600 text-white px-8 py-3 rounded-lg font-medium transition-colors"
      >
        Open Dashboard
      </button>
    </div>
  );

  const renderCurrentStep = () => {
    switch (currentStep) {
      case 0:
        return renderWelcomeStep();
      case 1:
        return renderConfigureStep();
      case 2:
        return renderInstallStep();
      case 3:
        return renderCompleteStep();
      default:
        return renderWelcomeStep();
    }
  };

  return (
    <div className="min-h-screen bg-gradient-to-br from-carbide-50 to-carbide-100">
      {/* Header */}
      <div className="bg-white border-b border-gray-200">
        <div className="max-w-6xl mx-auto px-4 py-6">
          <div className="flex items-center justify-between">
            <div className="flex items-center space-x-3">
              <div className="w-8 h-8 bg-carbide-500 rounded-lg flex items-center justify-center">
                <span className="text-white font-bold text-sm">C</span>
              </div>
              <h1 className="text-xl font-bold text-gray-900">Carbide Provider Setup</h1>
            </div>
            
            {/* Step indicator */}
            <div className="flex items-center space-x-2">
              {steps.map((step, index) => (
                <div key={step.id} className="flex items-center">
                  <div className={`w-8 h-8 rounded-full flex items-center justify-center ${
                    index <= currentStep 
                      ? 'bg-carbide-500 text-white' 
                      : 'bg-gray-200 text-gray-500'
                  }`}>
                    {index < currentStep ? (
                      <CheckCircle className="w-5 h-5" />
                    ) : (
                      <span className="text-sm font-medium">{index + 1}</span>
                    )}
                  </div>
                  {index < steps.length - 1 && (
                    <div className={`w-8 h-0.5 ${
                      index < currentStep ? 'bg-carbide-500' : 'bg-gray-200'
                    }`} />
                  )}
                </div>
              ))}
            </div>
          </div>
        </div>
      </div>

      {/* Content */}
      <div className="max-w-4xl mx-auto px-4 py-12">
        {renderCurrentStep()}
        
        {/* Navigation */}
        {currentStep < 3 && !isInstalling && (
          <div className="flex justify-between items-center mt-12 max-w-lg mx-auto">
            <button
              onClick={handleBack}
              disabled={currentStep === 0}
              className="px-6 py-2 text-gray-600 disabled:text-gray-400 hover:text-gray-800 transition-colors"
            >
              Back
            </button>
            
            {currentStep === 1 ? (
              <button
                onClick={handleInstall}
                className="bg-carbide-500 hover:bg-carbide-600 text-white px-8 py-3 rounded-lg font-medium transition-colors"
              >
                Install Carbide
              </button>
            ) : currentStep < 2 ? (
              <button
                onClick={handleNext}
                className="bg-carbide-500 hover:bg-carbide-600 text-white px-8 py-3 rounded-lg font-medium transition-colors"
              >
                Next
              </button>
            ) : null}
          </div>
        )}
      </div>
    </div>
  );
};

function getUserDisplayName(): string {
  // Get the current user's display name for default provider name
  return "your-mac-mini";
}

export default InstallWizard;