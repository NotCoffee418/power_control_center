# power-control-center

## Installation

### Quick Install (One-liner)

Install the latest release with a single command:

```bash
curl -fsSL https://raw.githubusercontent.com/NotCoffee418/power_control_center/main/installation/install.sh | sudo bash
```

### Manual Installation

1. Download the installation script:
```bash
wget https://raw.githubusercontent.com/NotCoffee418/power_control_center/main/installation/install.sh
chmod +x install.sh
```

2. Run the installer:
```bash
sudo ./install.sh
```

The installer will:
- Detect your system architecture (x86_64, arm64, armv7)
- Download the latest release binary
- Create necessary system user and directories
- Install the systemd service
- Set appropriate permissions

### Post-Installation Configuration

1. Create a configuration file at `/etc/power_control_center/config.json`:
```bash
sudo nano /etc/power_control_center/config.json
```

Use the example configuration below and adjust to your needs.

2. Enable and start the service:
```bash
sudo systemctl enable power_control_center
sudo systemctl start power_control_center
```

3. Check the service status:
```bash
sudo systemctl status power_control_center
```

4. View logs:
```bash
sudo journalctl -u power_control_center -f
```

### Supported Architectures
- Linux x86_64
- Linux ARM64 (aarch64)
- Linux ARMv7

## Files
### Config
Config file must be manually created on the device, use config.example.toml as a template.  
Path: `/etc/power_control_center/config.json`

### Database
Database file will be automatically created on the device, if it does not exist.  
Path is defined in config but is quite expected to be at this path.  
Path: `/var/lib/power_control_center/pcc.db`

## API Endpoints

### PIR Detection Endpoints

#### POST /api/pir/detect
Records a PIR (motion) detection event and immediately turns off the corresponding AC device.

**Query Parameters:**
- `device` (required) - The device name (e.g., "Veranda")

**Headers:**
- `Authorization: ApiKey <your_pir_api_key>` or `Authorization: Bearer <your_pir_api_key>`

**Example:**
```bash
curl -X POST "http://localhost:9040/api/pir/detect?device=Veranda" \
  -H "Authorization: ApiKey your_pir_api_key_here"
```

**Behavior:**
- Immediately turns off the AC for the specified device
- Records the detection time
- Prevents the AC from being turned back on for the configured timeout period (default: 5 minutes)

#### POST /api/pir/alive
Receives a keep-alive signal from PIR devices for monitoring purposes.

**Query Parameters:**
- `device` (optional) - The device name

**Headers:**
- `Authorization: ApiKey <your_pir_api_key>` or `Authorization: Bearer <your_pir_api_key>`

**Example:**
```bash
curl -X POST "http://localhost:9040/api/pir/alive?device=Veranda" \
  -H "Authorization: ApiKey your_pir_api_key_here"
```

### Configuration

Add the following fields to your config.json:

```json
{
  "pir_api_key": "your_secure_api_key_here",
  "pir_timeout_minutes": 5
}
```

- `pir_api_key`: API key for authenticating PIR device requests (optional, defaults to empty/no auth)
- `pir_timeout_minutes`: Number of minutes to keep AC off after PIR detection (optional, defaults to 5)