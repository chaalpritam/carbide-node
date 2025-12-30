import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/tauri";
import { listen } from "@tauri-apps/api/event";
import InstallWizard from "./components/InstallWizard";
import Dashboard from "./components/Dashboard";
import { ProviderStatus } from "./types";

function App() {
  const [isInstalled, setIsInstalled] = useState<boolean | null>(null);
  const [isLoading, setIsLoading] = useState(true);

  useEffect(() => {
    checkInstallation();
  }, []);

  const checkInstallation = async () => {
    try {
      const installed = await invoke<boolean>("check_installation");
      setIsInstalled(installed);
    } catch (error) {
      console.error("Failed to check installation:", error);
      setIsInstalled(false);
    } finally {
      setIsLoading(false);
    }
  };

  const handleInstallComplete = () => {
    setIsInstalled(true);
  };

  if (isLoading) {
    return (
      <div className="min-h-screen bg-gradient-to-br from-carbide-50 to-carbide-100 flex items-center justify-center">
        <div className="text-center">
          <div className="w-12 h-12 border-4 border-carbide-500 border-t-transparent rounded-full animate-spin mx-auto mb-4"></div>
          <p className="text-carbide-700 font-medium">Loading Carbide Provider...</p>
        </div>
      </div>
    );
  }

  if (!isInstalled) {
    return <InstallWizard onComplete={handleInstallComplete} />;
  }

  return <Dashboard />;
}

export default App;