#!/bin/bash

# WiFi Auto-Reconnect Script

WIFI_INTERFACE="wlan0"

echo "[$(date '+%Y-%m-%d %H:%M:%S')] Checking WiFi..."

# Check if wlan0 is disconnected
if nmcli device status | grep "^$WIFI_INTERFACE" | grep -q "disconnected"; then
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] WiFi disconnected, reconnecting..."
    
    # Get first wifi connection name
    CONNECTION_NAME=$(nmcli -t -f NAME,TYPE connection show | grep :802-11-wireless | cut -d: -f1 | head -1)
    
    if [ -z "$CONNECTION_NAME" ]; then
        echo "[$(date '+%Y-%m-%d %H:%M:%S')] ERROR: No WiFi connection found"
        exit 1
    fi
    
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] Connecting to: $CONNECTION_NAME"
    nmcli connection up "$CONNECTION_NAME"
    
    if [ $? -eq 0 ]; then
        echo "[$(date '+%Y-%m-%d %H:%M:%S')] Connected"
        exit 0
    else
        echo "[$(date '+%Y-%m-%d %H:%M:%S')] Failed to connect"
        exit 1
    fi
else
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] WiFi already connected"
    exit 0
fi