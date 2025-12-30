#!/bin/bash
# Carbide Provider Monitoring Dashboard
# Real-time monitoring for your storage provider

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color
BOLD='\033[1m'

CARBIDE_HOME="$HOME/.carbide"
PROVIDER_LOG="$CARBIDE_HOME/logs/provider.log"
STORAGE_DIR="$CARBIDE_HOME/data/storage"
CONFIG_FILE="$CARBIDE_HOME/config/provider.toml"

# Read configuration if available
PORT=8080
PROVIDER_NAME="Unknown"

if [ -f "$CONFIG_FILE" ]; then
    PORT=$(grep "port =" "$CONFIG_FILE" | sed 's/.*= *//' | tr -d ' ')
    PROVIDER_NAME=$(grep "name =" "$CONFIG_FILE" | sed 's/.*= *//' | tr -d '"')
fi

print_header() {
    clear
    echo -e "${BLUE}${BOLD}"
    echo "╔══════════════════════════════════════════════════════════════╗"
    echo "║                                                              ║"
    echo "║           🌟 Carbide Provider Monitoring 🌟                 ║"
    echo "║                                                              ║"
    echo "╚══════════════════════════════════════════════════════════════╝"
    echo -e "${NC}"
    echo -e "Provider: ${BOLD}$PROVIDER_NAME${NC}"
    echo -e "Time: ${BOLD}$(date)${NC}"
    echo ""
}

check_service_status() {
    echo -e "${BOLD}🔧 Service Status${NC}"
    echo "─────────────────────"
    
    # Check if service is loaded
    if launchctl list | grep -q "com.carbide.provider" 2>/dev/null; then
        echo -e "✅ Service: ${GREEN}Running${NC}"
    else
        echo -e "❌ Service: ${RED}Stopped${NC}"
    fi
    
    # Check if port is listening
    if lsof -i :$PORT &> /dev/null; then
        echo -e "✅ Network: ${GREEN}Listening on port $PORT${NC}"
    else
        echo -e "❌ Network: ${RED}Not listening on port $PORT${NC}"
    fi
    
    # Check process
    if pgrep -f "carbide-provider" > /dev/null; then
        PID=$(pgrep -f "carbide-provider")
        echo -e "✅ Process: ${GREEN}Running (PID: $PID)${NC}"
    else
        echo -e "❌ Process: ${RED}Not running${NC}"
    fi
    
    echo ""
}

check_storage_status() {
    echo -e "${BOLD}💾 Storage Status${NC}"
    echo "──────────────────────"
    
    if [ -d "$STORAGE_DIR" ]; then
        STORAGE_USED=$(du -sh "$STORAGE_DIR" 2>/dev/null | cut -f1)
        FILES_COUNT=$(find "$STORAGE_DIR" -type f 2>/dev/null | wc -l)
        echo -e "📁 Directory: ${GREEN}$STORAGE_DIR${NC}"
        echo -e "📊 Used Space: ${YELLOW}${STORAGE_USED:-0B}${NC}"
        echo -e "📄 Files Stored: ${YELLOW}$FILES_COUNT${NC}"
        
        # Check available space
        AVAILABLE_GB=$(df -g "$STORAGE_DIR" 2>/dev/null | awk 'NR==2{print $4}')
        if [ -n "$AVAILABLE_GB" ]; then
            if [ "$AVAILABLE_GB" -lt 5 ]; then
                echo -e "⚠️  Available: ${RED}${AVAILABLE_GB}GB (Low!)${NC}"
            else
                echo -e "✅ Available: ${GREEN}${AVAILABLE_GB}GB${NC}"
            fi
        fi
    else
        echo -e "❌ Storage directory not found: ${RED}$STORAGE_DIR${NC}"
    fi
    
    echo ""
}

check_network_activity() {
    echo -e "${BOLD}🌐 Network Activity${NC}"
    echo "────────────────────────"
    
    # Test provider endpoint
    if curl -s "http://localhost:$PORT/api/v1/provider/status" > /dev/null 2>&1; then
        echo -e "✅ API: ${GREEN}Responding${NC}"
        
        # Get status info
        STATUS_JSON=$(curl -s "http://localhost:$PORT/api/v1/provider/status" 2>/dev/null)
        if [ $? -eq 0 ] && [ -n "$STATUS_JSON" ]; then
            echo -e "📡 Status: ${GREEN}Healthy${NC}"
        fi
    else
        echo -e "❌ API: ${RED}Not responding${NC}"
    fi
    
    # Check recent connections (last 5 minutes)
    CONNECTIONS=$(lsof -i :$PORT 2>/dev/null | grep ESTABLISHED | wc -l)
    if [ "$CONNECTIONS" -gt 0 ]; then
        echo -e "🔗 Connections: ${GREEN}$CONNECTIONS active${NC}"
    else
        echo -e "🔗 Connections: ${YELLOW}None${NC}"
    fi
    
    echo ""
}

show_recent_logs() {
    echo -e "${BOLD}📝 Recent Logs${NC}"
    echo "───────────────────"
    
    if [ -f "$PROVIDER_LOG" ]; then
        echo -e "${YELLOW}Last 10 log entries:${NC}"
        tail -n 10 "$PROVIDER_LOG" | while IFS= read -r line; do
            if echo "$line" | grep -q "ERROR\|error\|Error"; then
                echo -e "${RED}$line${NC}"
            elif echo "$line" | grep -q "WARN\|warn\|Warn"; then
                echo -e "${YELLOW}$line${NC}"
            elif echo "$line" | grep -q "INFO\|info\|Started\|Listening"; then
                echo -e "${GREEN}$line${NC}"
            else
                echo "$line"
            fi
        done
    else
        echo -e "${YELLOW}No logs found at: $PROVIDER_LOG${NC}"
    fi
    
    echo ""
}

show_performance_metrics() {
    echo -e "${BOLD}📊 Performance Metrics${NC}"
    echo "──────────────────────────"
    
    # CPU usage of carbide-provider
    if pgrep -f "carbide-provider" > /dev/null; then
        PID=$(pgrep -f "carbide-provider")
        CPU_USAGE=$(ps -p $PID -o %cpu | tail -1 | xargs)
        MEM_USAGE=$(ps -p $PID -o %mem | tail -1 | xargs)
        echo -e "💻 CPU Usage: ${YELLOW}${CPU_USAGE}%${NC}"
        echo -e "🧠 Memory Usage: ${YELLOW}${MEM_USAGE}%${NC}"
    else
        echo -e "${RED}Process not running${NC}"
    fi
    
    # System load
    LOAD=$(uptime | awk -F'load averages:' '{print $2}' | xargs)
    echo -e "⚖️  System Load: ${YELLOW}$LOAD${NC}"
    
    # Uptime
    UPTIME=$(uptime | awk -F'up ' '{print $2}' | awk -F', [0-9]+ users' '{print $1}')
    echo -e "⏱️  System Uptime: ${YELLOW}$UPTIME${NC}"
    
    echo ""
}

show_earnings_estimate() {
    echo -e "${BOLD}💰 Earnings Estimate${NC}"
    echo "─────────────────────────"
    
    if [ -d "$STORAGE_DIR" ]; then
        STORAGE_USED_BYTES=$(du -sb "$STORAGE_DIR" 2>/dev/null | cut -f1)
        STORAGE_USED_GB=$(echo "scale=2; $STORAGE_USED_BYTES / 1073741824" | bc -l 2>/dev/null)
        
        # Estimate daily earnings (assuming $0.005/GB/month)
        PRICE_PER_GB_MONTH=0.005
        DAILY_RATE=$(echo "scale=6; $PRICE_PER_GB_MONTH / 30" | bc -l 2>/dev/null)
        DAILY_EARNINGS=$(echo "scale=4; $STORAGE_USED_GB * $DAILY_RATE" | bc -l 2>/dev/null)
        MONTHLY_EARNINGS=$(echo "scale=4; $STORAGE_USED_GB * $PRICE_PER_GB_MONTH" | bc -l 2>/dev/null)
        
        echo -e "💾 Storage Used: ${YELLOW}${STORAGE_USED_GB:-0}GB${NC}"
        echo -e "📈 Est. Daily: ${GREEN}\$${DAILY_EARNINGS:-0.0000}${NC}"
        echo -e "📊 Est. Monthly: ${GREEN}\$${MONTHLY_EARNINGS:-0.0000}${NC}"
    else
        echo -e "${RED}Cannot calculate - storage directory not found${NC}"
    fi
    
    echo ""
}

show_quick_actions() {
    echo -e "${BOLD}🎮 Quick Actions${NC}"
    echo "─────────────────────"
    echo "r) Refresh dashboard"
    echo "s) Start provider service"
    echo "t) Stop provider service"
    echo "l) View full logs"
    echo "c) Clear logs"
    echo "q) Quit"
    echo ""
    echo -n "Enter choice: "
}

handle_action() {
    case $1 in
        r|R)
            # Refresh - do nothing, loop will handle it
            ;;
        s|S)
            echo "🚀 Starting provider service..."
            launchctl load ~/Library/LaunchAgents/com.carbide.provider.plist 2>/dev/null
            if [ $? -eq 0 ]; then
                echo "✅ Service started!"
            else
                echo "❌ Failed to start service"
            fi
            sleep 2
            ;;
        t|T)
            echo "🛑 Stopping provider service..."
            launchctl unload ~/Library/LaunchAgents/com.carbide.provider.plist 2>/dev/null
            if [ $? -eq 0 ]; then
                echo "✅ Service stopped!"
            else
                echo "❌ Failed to stop service"
            fi
            sleep 2
            ;;
        l|L)
            echo "📝 Viewing full logs..."
            if [ -f "$PROVIDER_LOG" ]; then
                less "$PROVIDER_LOG"
            else
                echo "❌ Log file not found: $PROVIDER_LOG"
                sleep 2
            fi
            ;;
        c|C)
            echo "🗑️ Clearing logs..."
            if [ -f "$PROVIDER_LOG" ]; then
                > "$PROVIDER_LOG"
                echo "✅ Logs cleared!"
            else
                echo "❌ Log file not found"
            fi
            sleep 2
            ;;
        q|Q)
            echo "👋 Goodbye!"
            exit 0
            ;;
        *)
            echo "❌ Invalid choice"
            sleep 1
            ;;
    esac
}

# Main monitoring loop
main() {
    # Check if bc is available for calculations
    if ! command -v bc &> /dev/null; then
        echo "⚠️ Warning: 'bc' calculator not found. Earnings calculations disabled."
        echo "Install with: brew install bc"
        echo ""
    fi
    
    while true; do
        print_header
        check_service_status
        check_storage_status
        check_network_activity
        show_performance_metrics
        show_earnings_estimate
        show_recent_logs
        show_quick_actions
        
        read -t 30 choice
        if [ $? -ne 0 ]; then
            # Timeout - refresh automatically
            continue
        fi
        
        handle_action "$choice"
    done
}

# Run main function
main