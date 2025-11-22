# Pi WiFi Auto-Reconnect

Reconnects WiFi every minute if disconnected.

## Install

```bash
curl -o install.sh https://raw.githubusercontent.com/NotCoffee418/power_control_center/main/pi-wifi-reconnector/install-wifi-reconnect-cron.sh
chmod +x install.sh
./install.sh
```

## Uninstall

```bash
crontab -l | grep -v wifi-reconnect.sh | crontab -
sudo rm /usr/local/bin/wifi-reconnect.sh
```

## Check logs

```bash
grep CRON /var/log/syslog | grep wifi-reconnect
```