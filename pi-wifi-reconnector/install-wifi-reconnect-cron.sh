#!/bin/bash

# Download script
curl -o /tmp/wifi-reconnect.sh https://raw.githubusercontent.com/NotCoffee418/power_control_center/main/pi-wifi-reconnector/wifi-reconnect.sh

# Install to /usr/local/bin
sudo cp /tmp/wifi-reconnect.sh /usr/local/bin/
sudo chmod +x /usr/local/bin/wifi-reconnect.sh

# Add cron job if not exists
(crontab -l 2>/dev/null | grep -q wifi-reconnect.sh) || (crontab -l 2>/dev/null; echo "* * * * * /usr/local/bin/wifi-reconnect.sh") | crontab -

echo "Done. WiFi reconnect runs every minute."